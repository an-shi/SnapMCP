use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG: OnceLock<Mutex<Option<File>>> = OnceLock::new();

pub fn init() {
    let _ = LOG.set(Mutex::new(open_log()));
}

fn open_log() -> Option<File> {
    let path = crate::paths::log_file();
    crate::paths::ensure_parent(&path).ok()?;
    if std::fs::metadata(&path)
        .map(|m| m.len() > 2 * 1024 * 1024)
        .unwrap_or(false)
    {
        let _ = std::fs::remove_file(&path);
    }
    OpenOptions::new().create(true).append(true).open(path).ok()
}

fn write_line(level: &str, msg: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("[{ts}] [{level}] {msg}\n");
    if cfg!(debug_assertions) {
        eprint!("{line}");
    }
    if let Some(lock) = LOG.get() {
        if let Ok(mut guard) = lock.lock() {
            if guard.is_none() {
                *guard = open_log();
            }
            if let Some(file) = guard.as_mut() {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

pub fn info(msg: impl AsRef<str>) {
    write_line("INFO", msg.as_ref());
}

pub fn warn(msg: impl AsRef<str>) {
    write_line("WARN", msg.as_ref());
}

pub fn error(msg: impl AsRef<str>) {
    write_line("ERROR", msg.as_ref());
}
