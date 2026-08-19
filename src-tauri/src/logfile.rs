//! File logging for the app. When debug logging is enabled, a timestamped
//! line is appended to moonhunt.log in the app data dir. The rest of the app
//! keeps calling println! (which always reaches stdout) for the dev console;
//! these helpers add the file sink on top, gated by the setting.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();

/// Set the on/off switch and the target file path (called at boot + on toggle).
pub fn configure_logging(enabled: bool, app_data_dir: PathBuf) {
    LOGGING_ENABLED.store(enabled, Ordering::SeqCst);
    let _ = LOG_FILE.set(app_data_dir.join("moonhunt.log"));
}

pub fn logging_enabled() -> bool {
    LOGGING_ENABLED.load(Ordering::SeqCst)
}

/// Path to the log file, if configured.
pub fn log_file_path() -> Option<PathBuf> {
    LOG_FILE.get().cloned()
}

/// Append a timestamped line to the log file. Never fails the caller: if it
/// can't write, the error is printed to stdout like any other dev log.
pub fn log_line(msg: &str) {
    if !logging_enabled() {
        return;
    }
    let Some(path) = LOG_FILE.get() else { return };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let msecs = d.subsec_millis();
            format!("{}.{:03}", secs, msecs)
        })
        .unwrap_or_else(|_| "?".to_string());
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", ts, msg);
    }
}

/// `logln!` expands to a normal `println!` plus a file append when enabled.
#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {
        println!($($arg)*);
        $crate::logfile::log_line(&format!($($arg)*));
    };
}

// ── Tauri commands ─────────────────────────────────────────────────
use tauri::Manager;

/// Turn debug logging on/off and persist the setting so it survives restarts.
#[tauri::command]
pub fn set_debug_logging(enabled: bool, app: tauri::AppHandle) -> Result<String, String> {
    {
        let state = app.state::<crate::state::AppState>();
        let mut guard = crate::state::lock(&state.proxy_settings)?;
        guard.debug_logging = enabled;
    }
    crate::db::save_settings_to_db(&{
        let state = app.state::<crate::state::AppState>();
        let guard = crate::state::lock(&state.proxy_settings)?;
        guard.clone()
    })?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    configure_logging(enabled, data_dir.clone());
    if enabled {
        println!("[log] debug logging enabled → {}/moonhunt.log", data_dir.display());
        log_line("debug logging enabled");
    } else {
        println!("[log] debug logging disabled");
    }
    Ok(if enabled { "enabled".into() } else { "disabled".into() })
}

/// Absolute path to the current log file (moonhunt.log in the app data dir).
#[tauri::command]
pub fn get_log_file_path(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let log_path = data_dir.join("moonhunt.log");
    Ok(log_path.to_string_lossy().into_owned())
}

/// Append a single line from the frontend (used by the console bridge).
#[tauri::command]
pub fn append_log_line(msg: String) -> Result<(), String> {
    log_line(&msg);
    Ok(())
}

/// Reveal the log file folder in the OS file explorer.
#[tauri::command]
pub fn open_log_folder(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    tauri_plugin_opener::open_path(&data_dir, None::<&str>)
        .map_err(|e| format!("Failed to open log folder: {}", e))?;
    Ok(data_dir.to_string_lossy().into_owned())
}