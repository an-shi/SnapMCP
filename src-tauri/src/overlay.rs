use crate::models::{MonitorSnapshot, RectI};
use crate::state::{AppState, InteractiveOutcome, OverlayMode, PendingInteractive, PendingOverlay};
use crate::{capture, diag, export, paths, util};
use base64::Engine;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tokio::sync::oneshot;

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayBootstrap {
    pub capture_id: String,
    /// PNG as a data URL. Keeping the image same-origin avoids tainting the
    /// annotation canvas, which would make export to PNG/clipboard impossible.
    pub image_data_url: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub display: MonitorSnapshot,
    pub region: RectI,
    pub mode: OverlayMode,
    pub hint: Option<String>,
}

fn new_capture_id() -> String {
    format!(
        "cap_{}_{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f"),
        CAPTURE_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

async fn hide_other_windows(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.hide();
    }
    // Give the compositor a moment to remove the window from the screen.
    tokio::time::sleep(Duration::from_millis(140)).await;
}

fn close_overlay_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.close();
    }
}

fn remove_temp(path: &std::path::Path) {
    if path.starts_with(paths::temp_captures_dir()) {
        let _ = std::fs::remove_file(path);
    }
}

fn cancel_interactive(state: &AppState, reason: &str) {
    if let Ok(mut guard) = state.pending_interactive.lock() {
        if let Some(pending) = guard.take() {
            let _ = pending.tx.send(Err(reason.to_string()));
        }
    }
}

fn cleanup_state(state: &AppState, reason: &str) -> Option<PendingOverlay> {
    let pending = state
        .pending_overlay
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(ref overlay) = pending {
        remove_temp(&overlay.path);
    }
    cancel_interactive(state, reason);
    state.release_busy();
    pending
}

/// Destroyed events can arrive after a new overlay has already been prepared.
/// Only clean up the capture that owned the window that was destroyed.
fn cleanup_matching(state: &AppState, capture_id: &str, reason: &str) -> Option<PendingOverlay> {
    let pending = state.pending_overlay.lock().ok().and_then(|mut guard| {
        if guard
            .as_ref()
            .map(|overlay| overlay.id == capture_id)
            .unwrap_or(false)
        {
            guard.take()
        } else {
            None
        }
    });
    if let Some(ref overlay) = pending {
        remove_temp(&overlay.path);
    }
    let cancelled = state.pending_interactive.lock().ok().and_then(|mut guard| {
        if guard
            .as_ref()
            .map(|pending| pending.capture_id == capture_id)
            .unwrap_or(false)
        {
            guard.take()
        } else {
            None
        }
    });
    if let Some(pending) = cancelled {
        let _ = pending.tx.send(Err(reason.to_string()));
    }
    if pending.is_some() {
        state.release_busy();
    }
    pending
}

async fn start_overlay(
    app: AppHandle,
    state: Arc<AppState>,
    mode: OverlayMode,
    hint: Option<String>,
    interactive_tx: Option<oneshot::Sender<Result<InteractiveOutcome, String>>>,
) -> Result<(), String> {
    if state
        .busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Another screenshot is already in progress".into());
    }

    let result: Result<(), String> = async {
        // A previous overlay may still be destroying itself asynchronously.
        if let Some(stale) = app.get_webview_window("overlay") {
            let _ = stale.close();
            for _ in 0..20 {
                if app.get_webview_window("overlay").is_none() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }

        hide_other_windows(&app).await;
        let (image, display) = tauri::async_runtime::spawn_blocking(move || {
            capture::capture_primary().map_err(|e| format!("Screen capture failed: {e}"))
        })
        .await
        .map_err(|e| format!("Capture task failed: {e}"))??;

        let id = new_capture_id();
        let path = paths::temp_captures_dir().join(format!("{id}.png"));
        util::save_png(&image, &path)?;
        let width = image.width();
        let height = image.height();
        let region = display.bounds();

        if let Some(win) = app.get_webview_window("overlay") {
            let _ = win.close();
        }

        let pending = PendingOverlay {
            id: id.clone(),
            path: path.clone(),
            width,
            height,
            display: display.clone(),
            region,
            mode,
            hint: hint.clone(),
            deadline: None,
        };
        if let Ok(mut guard) = state.pending_overlay.lock() {
            *guard = Some(pending);
        } else {
            let _ = std::fs::remove_file(&path);
            return Err("Screenshot state is unavailable".into());
        }

        if let Some(tx) = interactive_tx {
            if let Ok(mut guard) = state.pending_interactive.lock() {
                *guard = Some(PendingInteractive {
                    capture_id: id.clone(),
                    tx,
                });
            } else {
                cleanup_state(&state, "Screenshot state is unavailable");
                return Err("Interactive state is unavailable".into());
            }
        }

        let window =
            WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("overlay.html".into()))
                .title("SnapMCP")
                .decorations(false)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .focused(true)
                .inner_size(900.0, 600.0)
                .position(0.0, 0.0)
                .build()
                .map_err(|e| format!("Failed to open the capture overlay: {e}"))?;

        let _ = window.set_position(PhysicalPosition::new(display.x, display.y));
        let _ = window.set_size(PhysicalSize::new(
            display.width.max(1),
            display.height.max(1),
        ));
        let _ = window.set_always_on_top(true);

        let state_for_events = Arc::clone(&state);
        let id_for_events = id.clone();
        window.on_window_event(move |event| {
            if matches!(event, WindowEvent::Destroyed) {
                let pending = cleanup_matching(
                    &state_for_events,
                    &id_for_events,
                    "Screenshot window closed",
                );
                if let Some(pending) = pending {
                    diag::info(format!("Overlay {} destroyed", pending.id));
                }
            }
        });

        diag::info(format!(
            "Overlay {} prepared for display {} ({}x{})",
            id, display.name, width, height
        ));
        Ok(())
    }
    .await;

    if result.is_err() {
        cleanup_state(&state, "Screenshot initialization failed");
        close_overlay_window(&app);
    }
    result
}

pub fn request_human_capture(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>().inner().clone();
        if let Err(err) = start_overlay(
            app.clone(),
            Arc::clone(&state),
            OverlayMode::Capture,
            None,
            None,
        )
        .await
        {
            diag::warn(format!("Hotkey capture failed: {err}"));
            export::show_error_message(&app, &format!("截图失败：\n{err}"));
        }
    });
}

pub async fn start_interactive(
    app: AppHandle,
    state: Arc<AppState>,
    hint: Option<String>,
) -> Result<oneshot::Receiver<Result<InteractiveOutcome, String>>, String> {
    let (tx, rx) = oneshot::channel();
    start_overlay(app, state, OverlayMode::Interactive, hint, Some(tx)).await?;
    Ok(rx)
}

pub fn bootstrap(state: &AppState, capture_id: Option<String>) -> Result<OverlayBootstrap, String> {
    let guard = state
        .pending_overlay
        .lock()
        .map_err(|_| "Screenshot state is unavailable".to_string())?;
    let pending = guard
        .as_ref()
        .ok_or_else(|| "No pending screenshot".to_string())?;
    if let Some(ref requested) = capture_id {
        if requested != &pending.id {
            return Err("This screenshot request has expired".into());
        }
    }
    if !pending.path.is_file() {
        return Err("The temporary capture file is missing".into());
    }
    let bytes = std::fs::read(&pending.path)
        .map_err(|e| format!("Could not read the temporary capture: {e}"))?;
    let image_data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    Ok(OverlayBootstrap {
        capture_id: pending.id.clone(),
        image_data_url,
        width: pending.width,
        height: pending.height,
        scale_factor: pending.display.scale_factor,
        display: pending.display.clone(),
        region: pending.region,
        mode: pending.mode,
        hint: pending.hint.clone(),
    })
}

pub fn ready(app: &AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("overlay")
        .ok_or_else(|| "Screenshot window is not ready".to_string())?;
    win.show()
        .map_err(|e| format!("Failed to show the screenshot overlay: {e}"))?;
    let _ = win.set_always_on_top(true);
    win.set_focus()
        .map_err(|e| format!("Failed to focus the screenshot overlay: {e}"))?;
    Ok(())
}

pub fn pending(state: &AppState) -> Option<PendingOverlay> {
    state
        .pending_overlay
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

pub fn close(app: &AppHandle, state: &AppState, capture_id: &str, reason: &str) {
    let current_id = state
        .pending_overlay
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|p| p.id.clone()));
    if let Some(current_id) = current_id {
        if current_id != capture_id {
            // Do not let a stale overlay close a newer capture.
            return;
        }
    }
    let matches = state
        .pending_overlay
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    if let Some(pending) = matches {
        remove_temp(&pending.path);
        cancel_interactive(state, reason);
        state.release_busy();
    }
    close_overlay_window(app);
}

pub fn cancel_all(app: &AppHandle, state: &AppState, reason: &str) {
    cleanup_state(state, reason);
    close_overlay_window(app);
}
