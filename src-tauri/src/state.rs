use crate::models::{MonitorSnapshot, RectI};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMode {
    Capture,
    Interactive,
}

#[derive(Debug, Clone)]
pub struct PendingOverlay {
    pub id: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub display: MonitorSnapshot,
    pub region: RectI,
    pub mode: OverlayMode,
    pub hint: Option<String>,
    pub deadline: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveOutcome {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub region: RectI,
    pub display: MonitorSnapshot,
    pub annotations: Vec<serde_json::Value>,
}

pub struct PendingInteractive {
    pub capture_id: String,
    pub tx: oneshot::Sender<Result<InteractiveOutcome, String>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    pub port: u16,
    pub endpoint: String,
    pub token_required: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub pid: u32,
    pub port: u16,
    pub endpoint: String,
    pub token: String,
    pub version: String,
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub mcp: Mutex<McpStatus>,
    pub latest_capture: Mutex<Option<PathBuf>>,
    pub pending_overlay: Mutex<Option<PendingOverlay>>,
    pub pending_interactive: Mutex<Option<PendingInteractive>>,
    pub busy: AtomicBool,
    pub shortcut_error: Mutex<Option<String>>,
    pub exiting: AtomicBool,
}

pub struct BusyGuard {
    state: Arc<AppState>,
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.state.busy.store(false, Ordering::SeqCst);
    }
}

impl AppState {
    pub fn new() -> Arc<Self> {
        let settings_path = crate::paths::settings_file();
        let settings = Settings::load(&settings_path);
        Arc::new(Self {
            settings: Mutex::new(settings),
            settings_path,
            mcp: Mutex::new(McpStatus::default()),
            latest_capture: Mutex::new(None),
            pending_overlay: Mutex::new(None),
            pending_interactive: Mutex::new(None),
            busy: AtomicBool::new(false),
            shortcut_error: Mutex::new(None),
            exiting: AtomicBool::new(false),
        })
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.save(&self.settings_path)?;
        if let Ok(mut current) = self.settings.lock() {
            *current = settings.clone();
        }
        Ok(())
    }

    pub fn acquire_busy(self: &Arc<Self>) -> Option<BusyGuard> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            Some(BusyGuard {
                state: Arc::clone(self),
            })
        } else {
            None
        }
    }

    pub fn release_busy(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    pub fn set_latest_capture(&self, path: PathBuf) {
        if let Ok(mut latest) = self.latest_capture.lock() {
            *latest = Some(path);
        }
    }
}
