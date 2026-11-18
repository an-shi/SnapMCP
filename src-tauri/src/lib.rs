pub mod annotate;
pub mod capture;
pub mod cli;
pub mod commands;
pub mod diag;
pub mod export;
pub mod mcp;
pub mod models;
pub mod overlay;
pub mod paths;
pub mod settings;
pub mod state;
pub mod tray;
pub mod util;
pub mod window;

use crate::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diag::init();
    let args: Vec<String> = std::env::args().collect();
    let hidden = args.iter().any(|a| a == "--hidden");
    let initial_capture = args.iter().any(|a| a == "--capture");
    let show_window = !hidden && !initial_capture;

    std::panic::set_hook(Box::new(|info| {
        diag::error(format!("Panic: {info}"));
    }));

    let state = AppState::new();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == "--capture") {
                overlay::request_human_capture(app.clone());
            } else if !argv.iter().any(|a| a == "--hidden") {
                window::show_settings(app);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        overlay::request_human_capture(app.clone());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden".into()]),
        ))
        .manage(Arc::clone(&state))
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::update_shortcut,
            commands::set_autostart,
            commands::choose_save_dir,
            commands::open_save_dir,
            commands::open_config_dir,
            commands::reveal_path,
            commands::copy_text,
            commands::capture_now,
            commands::quit_app,
            commands::list_captures,
            commands::overlay_bootstrap,
            commands::overlay_ready,
            commands::overlay_cancel,
            commands::overlay_close,
            commands::export_capture,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let settings = state.settings_snapshot();

            if let Err(err) = app.global_shortcut().register(settings.shortcut.as_str()) {
                let message = format!(
                    "全局快捷键 {} 注册失败：{err}。请在设置中更换组合键。",
                    settings::shortcut_label(&settings.shortcut)
                );
                diag::error(&message);
                if let Ok(mut guard) = state.shortcut_error.lock() {
                    *guard = Some(message);
                }
            }

            if let Err(err) = mcp::server::start(app_handle.clone(), Arc::clone(&state)) {
                diag::error(format!("Cannot start MCP server: {err}"));
                if let Ok(mut status) = state.mcp.lock() {
                    status.running = false;
                    status.error = Some(err);
                }
            }

            if let Err(err) = tray::create(app.handle()) {
                diag::error(format!("Cannot create system tray: {err}"));
            }

            if show_window {
                window::show_settings(app.handle());
            }
            if initial_capture {
                overlay::request_human_capture(app.handle().clone());
            }
            Ok(())
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building SnapMCP");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            let state = app_handle.state::<Arc<AppState>>();
            if !state.exiting.load(Ordering::SeqCst) && code.is_none() {
                api.prevent_exit();
            }
        }
        tauri::RunEvent::Exit => {
            mcp::server::shutdown();
            let state = app_handle.state::<Arc<AppState>>();
            if let Ok(pending) = state.pending_overlay.lock() {
                if let Some(overlay) = pending.as_ref() {
                    let _ = std::fs::remove_file(&overlay.path);
                }
            }
            diag::info("SnapMCP exited");
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            window::show_settings(app_handle);
        }
        _ => {}
    });
}
