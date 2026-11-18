use crate::state::{AppState, InteractiveOutcome, McpStatus};
use crate::{diag, export, overlay, paths, settings, util};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub version: String,
    pub mcp: McpStatus,
    pub save_dir: String,
    pub shortcut: String,
    pub shortcut_label: String,
    pub shortcut_error: Option<String>,
    pub auto_start: bool,
    pub config_dir: String,
    pub capture_count: u64,
    pub latest_capture: Option<String>,
    pub platform: String,
    pub default_shortcut: String,
    pub busy: bool,
    pub app_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEntry {
    pub name: String,
    pub path: String,
    pub modified_ms: u64,
    pub size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub ok: bool,
    pub action: String,
    pub saved: bool,
    pub copied: bool,
    pub canceled: bool,
    pub path: Option<String>,
}

fn count_captures(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                })
                .count() as u64
        })
        .unwrap_or(0)
}

fn status(app: &AppHandle, state: &AppState) -> AppStatus {
    let current = state.settings_snapshot();
    let mcp = state.mcp.lock().map(|s| s.clone()).unwrap_or_default();
    let latest = state
        .latest_capture
        .lock()
        .ok()
        .and_then(|v| v.clone())
        .map(|p| p.to_string_lossy().to_string());
    let dir = export::capture_dir(app, state);
    let auto_start = app.autolaunch().is_enabled().unwrap_or(false);
    let shortcut_error = state.shortcut_error.lock().ok().and_then(|v| v.clone());
    AppStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        mcp,
        save_dir: dir.to_string_lossy().to_string(),
        shortcut: current.shortcut.clone(),
        shortcut_label: settings::shortcut_label(&current.shortcut),
        shortcut_error,
        auto_start,
        config_dir: paths::data_dir().to_string_lossy().to_string(),
        capture_count: count_captures(&dir),
        latest_capture: latest,
        platform: std::env::consts::OS.to_string(),
        default_shortcut: settings::DEFAULT_SHORTCUT.to_string(),
        busy: state.busy.load(Ordering::SeqCst),
        app_path: std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub fn get_status(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<AppStatus, String> {
    Ok(status(&app, &state))
}

#[tauri::command]
pub fn update_shortcut(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    shortcut: String,
) -> Result<Value, String> {
    let new_shortcut = shortcut.trim().to_string();
    if new_shortcut.is_empty() {
        return Err("快捷键不能为空".into());
    }
    if new_shortcut.len() > 100 {
        return Err("快捷键字符串过长".into());
    }
    let old = state.settings_snapshot();
    let manager = app.global_shortcut();
    let _ = manager.unregister(old.shortcut.as_str());
    if let Err(err) = manager.register(new_shortcut.as_str()) {
        let _ = manager.register(old.shortcut.as_str());
        let message = format!("快捷键注册失败：{err}。请换一个组合键，例如 CmdOrCtrl+Alt+X。");
        if let Ok(mut guard) = state.shortcut_error.lock() {
            *guard = Some(message.clone());
        }
        return Err(message);
    }

    let mut updated = old.clone();
    updated.shortcut = new_shortcut.clone();
    if let Err(err) = state.save_settings(&updated) {
        let _ = manager.unregister(new_shortcut.as_str());
        let _ = manager.register(old.shortcut.as_str());
        return Err(format!("无法保存快捷键：{err}"));
    }
    if let Ok(mut guard) = state.shortcut_error.lock() {
        *guard = None;
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&format!(
            "SnapMCP · {} 截图",
            settings::shortcut_label(&new_shortcut)
        )));
    }
    Ok(json!({
        "shortcut": new_shortcut,
        "label": settings::shortcut_label(&new_shortcut),
    }))
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    if enabled {
        app.autolaunch()
            .enable()
            .map_err(|e| format!("无法开启开机自启：{e}"))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|e| format!("无法关闭开机自启：{e}"))?;
    }
    Ok(app.autolaunch().is_enabled().unwrap_or(enabled))
}

#[tauri::command]
pub async fn choose_save_dir(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<String>, String> {
    let current = export::capture_dir(&app, &state);
    let app_for_dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        export::choose_folder(&app_for_dialog, &current)
    })
    .await
    .map_err(|e| format!("文件夹选择器失败：{e}"))??;
    let Some(dir) = picked else {
        return Ok(None);
    };
    let mut updated = state.settings_snapshot();
    updated.save_dir = dir.to_string_lossy().to_string();
    state.save_settings(&updated)?;
    Ok(Some(updated.save_dir))
}

#[tauri::command]
pub fn open_save_dir(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let dir = export::capture_dir(&app, &state);
    util::open_dir(&dir)
}

#[tauri::command]
pub fn open_config_dir() -> Result<(), String> {
    util::open_dir(&paths::data_dir())
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    util::reveal_path(Path::new(&path))
}

#[tauri::command]
pub fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    export::copy_text(&app, &text)
}

#[tauri::command]
pub fn capture_now(app: AppHandle) -> Result<(), String> {
    overlay::request_human_capture(app);
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.exiting.store(true, Ordering::SeqCst);
    crate::mcp::server::shutdown();
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn list_captures(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    limit: Option<u32>,
) -> Result<Vec<CaptureEntry>, String> {
    let limit = limit.unwrap_or(8).clamp(1, 50) as usize;
    let dir = export::capture_dir(&app, &state);
    let mut entries: Vec<CaptureEntry> = std::fs::read_dir(&dir)
        .map_err(|e| format!("无法读取截图目录：{e}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
            {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            let modified_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            Some(CaptureEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                modified_ms,
                size: metadata.len(),
            })
        })
        .collect();
    entries.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms));
    entries.truncate(limit);
    Ok(entries)
}

#[tauri::command]
pub fn overlay_bootstrap(
    state: State<'_, Arc<AppState>>,
    capture_id: Option<String>,
) -> Result<overlay::OverlayBootstrap, String> {
    overlay::bootstrap(&state, capture_id)
}

#[tauri::command]
pub fn overlay_ready(app: AppHandle) -> Result<(), String> {
    overlay::ready(&app)
}

#[tauri::command]
pub fn overlay_cancel(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    overlay::cancel_all(&app, &state, "用户取消了截图");
    Ok(())
}

#[tauri::command]
pub fn overlay_close(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    capture_id: String,
) -> Result<(), String> {
    overlay::close(&app, &state, &capture_id, "截图已完成");
    Ok(())
}

#[tauri::command]
pub async fn export_capture(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    capture_id: String,
    png_data_url: String,
    action: String,
    annotations: Value,
    region: Option<crate::models::RectI>,
) -> Result<ExportResult, String> {
    let pending = overlay::pending(&state).ok_or_else(|| "截图已过期，请重新截图".to_string())?;
    if pending.id != capture_id {
        return Err("截图已过期，请重新截图".into());
    }

    let raw = util::decode_data_url(&png_data_url)?;
    let image = image::load_from_memory(&raw)
        .map_err(|e| format!("无法解析截图数据：{e}"))?
        .to_rgba8();
    if image.width() > 20_000
        || image.height() > 20_000
        || image.width() as u64 * image.height() as u64 > 160_000_000
    {
        return Err("截图尺寸异常".into());
    }

    let action_key = action.as_str();
    let mut saved = false;
    let mut copied = false;
    let mut canceled = false;
    let mut saved_path: Option<std::path::PathBuf> = None;
    let capture_dir = export::capture_dir(&app, &state);

    match action_key {
        "clipboard" => {
            export::copy_image(&app, &image)?;
            copied = true;
        }
        "save" => {
            let path = util::resolve_save_path(None, &capture_dir, "SnapMCP")?;
            util::save_png(&image, &path)?;
            state.set_latest_capture(path.clone());
            saved = true;
            saved_path = Some(path);
        }
        "saveAs" => {
            let default_name = util::auto_file_name("SnapMCP");
            let dir = capture_dir.clone();
            let app_for_dialog = app.clone();
            let picked = tauri::async_runtime::spawn_blocking(move || {
                export::choose_save_png(&app_for_dialog, &dir, &default_name)
            })
            .await
            .map_err(|e| format!("保存对话框失败：{e}"))??;
            if let Some(mut path) = picked {
                if path.extension().is_none() {
                    path.set_extension("png");
                }
                util::save_png(&image, &path)?;
                state.set_latest_capture(path.clone());
                saved = true;
                saved_path = Some(path);
            } else {
                canceled = true;
            }
        }
        "confirm" => {
            let path = util::resolve_save_path(None, &capture_dir, "SnapMCP")?;
            util::save_png(&image, &path)?;
            state.set_latest_capture(path.clone());
            saved = true;
            saved_path = Some(path.clone());

            let annotation_values = annotations.as_array().cloned().unwrap_or_default();
            let selected_region = region
                .filter(|r| r.width > 0 && r.height > 0)
                .and_then(|r| r.intersect(pending.display.bounds()))
                .unwrap_or(pending.region);
            let outcome = InteractiveOutcome {
                path: path.to_string_lossy().to_string(),
                width: image.width(),
                height: image.height(),
                region: selected_region,
                display: pending.display.clone(),
                annotations: annotation_values,
            };
            if let Ok(mut guard) = state.pending_interactive.lock() {
                if guard
                    .as_ref()
                    .map(|p| p.capture_id == capture_id)
                    .unwrap_or(false)
                {
                    if let Some(pending_ai) = guard.take() {
                        if pending_ai.tx.send(Ok(outcome)).is_err() {
                            diag::warn("Interactive MCP request was already closed");
                        }
                    }
                }
            }
        }
        other => return Err(format!("未知导出操作：{other}")),
    }

    Ok(ExportResult {
        ok: true,
        action: action_key.to_string(),
        saved,
        copied,
        canceled,
        path: saved_path.map(|p| p.to_string_lossy().to_string()),
    })
}
