use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_SHORTCUT: &str = "CmdOrCtrl+Shift+X";
pub const DEFAULT_MCP_PORT: u16 = 37621;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Tauri accelerator syntax, e.g. `CmdOrCtrl+Shift+X`.
    pub shortcut: String,
    /// Empty means the OS "Pictures/SnapMCP" folder.
    pub save_dir: String,
    pub mcp_enabled: bool,
    /// Preferred loopback port. The app falls back to the next free port.
    pub mcp_port: u16,
    /// Optional bearer token for the HTTP MCP endpoint. Empty disables auth.
    pub auth_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            save_dir: String::new(),
            mcp_enabled: true,
            mcp_port: DEFAULT_MCP_PORT,
            auth_token: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        match std::fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(mut settings) => {
                    settings.normalize();
                    settings
                }
                Err(err) => {
                    crate::diag::warn(format!("Invalid settings file, using defaults: {err}"));
                    let _ = std::fs::rename(path, path.with_extension("json.bak"));
                    Settings::default()
                }
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        crate::paths::ensure_parent(path).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        crate::paths::write_atomic(path, &bytes).map_err(|e| e.to_string())
    }

    pub fn normalize(&mut self) {
        if self.shortcut.trim().is_empty() {
            self.shortcut = DEFAULT_SHORTCUT.to_string();
        }
        if self.mcp_port < 1024 {
            self.mcp_port = DEFAULT_MCP_PORT;
        }
        self.auth_token = self.auth_token.trim().to_string();
    }

    pub fn effective_save_dir(&self, pictures_dir: Option<PathBuf>) -> PathBuf {
        if !self.save_dir.trim().is_empty() {
            return PathBuf::from(self.save_dir.trim());
        }
        pictures_dir
            .unwrap_or_else(crate::paths::fallback_pictures_dir)
            .join("SnapMCP")
    }
}

pub fn shortcut_label(shortcut: &str) -> String {
    let mac = cfg!(target_os = "macos");
    let mut parts: Vec<String> = Vec::new();
    for raw in shortcut.split('+') {
        let p = raw.trim();
        if p.is_empty() {
            continue;
        }
        let label = match p.to_ascii_lowercase().as_str() {
            "cmdorctrl" | "commandorcontrol" | "cmdorcontrol" => {
                if mac {
                    "⌘"
                } else {
                    "Ctrl"
                }
            }
            "ctrl" | "control" => {
                if mac {
                    "⌃"
                } else {
                    "Ctrl"
                }
            }
            "cmd" | "command" | "super" | "meta" => {
                if mac {
                    "⌘"
                } else {
                    "Win"
                }
            }
            "shift" => {
                if mac {
                    "⇧"
                } else {
                    "Shift"
                }
            }
            "alt" | "option" => {
                if mac {
                    "⌥"
                } else {
                    "Alt"
                }
            }
            _ => &p.to_uppercase(),
        };
        parts.push(label.to_string());
    }
    if mac {
        parts.join("")
    } else {
        parts.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_shortcut_labels() {
        let label = shortcut_label(DEFAULT_SHORTCUT);
        if cfg!(target_os = "macos") {
            assert_eq!(label, "⌘⇧X");
        } else {
            assert_eq!(label, "Ctrl + Shift + X");
        }
        assert!(shortcut_label("Alt+F1").contains("F1"));
    }

    #[test]
    fn normalizes_invalid_ports() {
        let mut settings = Settings {
            mcp_port: 80,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.mcp_port, DEFAULT_MCP_PORT);
    }
}
