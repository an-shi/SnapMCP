use crate::state::AppState;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

pub fn capture_dir(app: &AppHandle, state: &AppState) -> PathBuf {
    let pictures = app.path().picture_dir().ok();
    let dir = state.settings_snapshot().effective_save_dir(pictures);
    let _ = crate::paths::ensure_dir(&dir);
    dir
}

pub fn copy_image(app: &AppHandle, image: &RgbaImage) -> Result<(), String> {
    let raw = image.as_raw().clone();
    let width = image.width();
    let height = image.height();
    let clipboard_image = tauri::image::Image::new_owned(raw, width, height);
    app.clipboard()
        .write_image(&clipboard_image)
        .map_err(|e| format!("Failed to write the image to the clipboard: {e}"))
}

pub fn copy_text(app: &AppHandle, text: &str) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to write text to the clipboard: {e}"))
}

pub fn choose_save_png(
    app: &AppHandle,
    default_dir: &Path,
    default_name: &str,
) -> Result<Option<PathBuf>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("保存标注截图")
        .set_directory(default_dir)
        .set_file_name(default_name)
        .add_filter("PNG 图片", &["png"])
        .blocking_save_file();
    match picked {
        None => Ok(None),
        Some(file) => file
            .into_path()
            .map(Some)
            .map_err(|e| format!("Invalid save path: {e}")),
    }
}

pub fn choose_folder(app: &AppHandle, default_dir: &Path) -> Result<Option<PathBuf>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("选择截图保存目录")
        .set_directory(default_dir)
        .blocking_pick_folder();
    match picked {
        None => Ok(None),
        Some(file) => file
            .into_path()
            .map(Some)
            .map_err(|e| format!("Invalid folder path: {e}")),
    }
}

pub fn show_error_message(app: &AppHandle, message: &str) {
    app.dialog()
        .message(message)
        .title("SnapMCP")
        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
        .show(|_| {});
}
