use std::path::{Path, PathBuf};

/// Locations used by the app. Kept independent of Tauri's path resolver so the
/// stdio bridge can find the running app without booting a webview.
pub fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("SnapMCP");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home)
                .join("AppData")
                .join("Roaming")
                .join("SnapMCP");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("SnapMCP");
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(config) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(config).join("snapmcp");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("snapmcp");
        }
    }
    std::env::temp_dir().join("snapmcp")
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn runtime_file() -> PathBuf {
    data_dir().join("runtime.json")
}

pub fn log_file() -> PathBuf {
    data_dir().join("snapmcp.log")
}

pub fn temp_captures_dir() -> PathBuf {
    std::env::temp_dir().join("snapmcp").join("captures")
}

pub fn fallback_pictures_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join("Pictures");
        }
    }
    #[cfg(any(target_os = "macos", all(unix, not(target_os = "macos"))))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Pictures");
        }
    }
    data_dir().join("captures")
}

pub fn ensure_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    ensure_parent(path)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(tmp, path)
}
