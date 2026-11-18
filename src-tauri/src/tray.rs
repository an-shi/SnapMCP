use crate::state::AppState;
use crate::{diag, export, mcp, overlay, paths, settings, util, window};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn create(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let current = state.settings_snapshot();
    let shortcut_label = settings::shortcut_label(&current.shortcut);
    let mcp_status = state.mcp.lock().map(|s| s.clone()).unwrap_or_default();
    let mcp_label = if mcp_status.running {
        format!("MCP 已就绪 · 127.0.0.1:{}", mcp_status.port)
    } else {
        "MCP 未启动".to_string()
    };

    let capture = MenuItem::with_id(
        app,
        "capture",
        format!("截取屏幕  ({shortcut_label})"),
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let open_settings = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let open_captures =
        MenuItem::with_id(app, "open_captures", "打开截图文件夹", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let mcp_item =
        MenuItem::with_id(app, "mcp", mcp_label, false, None::<&str>).map_err(|e| e.to_string())?;
    let open_config = MenuItem::with_id(app, "open_config", "打开配置目录", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "退出 SnapMCP", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let separator2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let separator3 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &capture,
            &open_settings,
            &open_captures,
            &separator1,
            &mcp_item,
            &open_config,
            &separator2,
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;
    let _ = separator3;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "No default application icon".to_string())?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(format!("SnapMCP · {shortcut_label} 截图"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => overlay::request_human_capture(app.clone()),
            "settings" => window::show_settings(app),
            "open_captures" => {
                let state = app.state::<Arc<AppState>>();
                let dir = export::capture_dir(app, &state);
                if let Err(err) = util::open_dir(&dir) {
                    diag::warn(format!("Could not open capture folder: {err}"));
                }
            }
            "open_config" => {
                if let Err(err) = util::open_dir(&paths::data_dir()) {
                    diag::warn(format!("Could not open config folder: {err}"));
                }
            }
            "quit" => {
                let state = app.state::<Arc<AppState>>();
                state.exiting.store(true, Ordering::SeqCst);
                mcp::server::shutdown();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_settings(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    diag::info("System tray initialized");
    Ok(())
}
