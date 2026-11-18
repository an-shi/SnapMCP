use crate::diag;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    let result = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("SnapMCP · 截图与 MCP")
        .inner_size(980.0, 760.0)
        .min_inner_size(820.0, 620.0)
        .resizable(true)
        .center()
        .visible(true)
        .focused(true)
        .theme(Some(tauri::Theme::Dark))
        .build();

    match result {
        Ok(_) => diag::info("Settings window opened"),
        Err(err) => diag::error(format!("Could not open settings window: {err}")),
    }
}
