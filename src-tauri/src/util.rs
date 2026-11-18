use base64::Engine;
use chrono::Local;
use image::RgbaImage;
use std::path::{Path, PathBuf};

pub fn filename_stamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S_%3f").to_string()
}

pub fn auto_file_name(prefix: &str) -> String {
    format!("{}_{}.png", prefix, filename_stamp())
}

pub fn decode_data_url(input: &str) -> Result<Vec<u8>, String> {
    let raw = if let Some(idx) = input.find(",") {
        if input[..idx].contains("base64") {
            &input[idx + 1..]
        } else {
            return Err("Only base64 data URLs are supported".into());
        }
    } else {
        input
    };
    let raw = raw.trim();
    if raw.len() > 120 * 1024 * 1024 {
        return Err("Image payload is too large".into());
    }
    base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| format!("Invalid base64 image: {e}"))
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;
    Ok(cursor.into_inner())
}

pub fn save_png(image: &RgbaImage, path: &Path) -> Result<(), String> {
    crate::paths::ensure_parent(path).map_err(|e| format!("Cannot create save folder: {e}"))?;
    image
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}

pub fn expand_tilde(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(trimmed)
}

/// Resolve a user/AI-supplied path. Relative paths are interpreted inside the
/// capture directory. A missing extension becomes `.png`.
pub fn resolve_save_path(
    requested: Option<&str>,
    capture_dir: &Path,
    prefix: &str,
) -> Result<PathBuf, String> {
    let Some(raw) = requested.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(capture_dir.join(auto_file_name(prefix)));
    };
    let mut path = expand_tilde(raw);
    if path.is_relative() {
        path = capture_dir.join(path);
    }
    if path.extension().is_none() {
        path.set_extension("png");
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if ext != "png" {
        return Err("SnapMCP only writes PNG files; use a .png path".into());
    }
    if path.is_dir() {
        return Err("savePath points to a directory; include a file name".into());
    }
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err("Invalid save path".into());
    };
    if file_name.is_empty() {
        return Err("Invalid save path".into());
    }
    Ok(path)
}

pub fn reveal_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("Path does not exist".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = std::process::Command::new("explorer.exe");
        if path.is_dir() {
            cmd.arg(path);
        } else {
            cmd.arg(format!("/select,{}", path.to_string_lossy()));
        }
        cmd.creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open Explorer: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        if path.is_dir() {
            cmd.arg(path);
        } else {
            cmd.arg("-R").arg(path);
        }
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open Finder: {e}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open file manager: {e}"))
    }
}

pub fn open_dir(path: &Path) -> Result<(), String> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| format!("Cannot create folder: {e}"))?;
    }
    reveal_path(path)
}
