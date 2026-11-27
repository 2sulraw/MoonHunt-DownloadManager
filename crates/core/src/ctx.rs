//! Core application context.
//!
//! In the Tauri build, backend modules received a `tauri::AppHandle` used for
//! three things: shared state access (`.state::<AppState>()`), config/data
//! directory resolution (`.path()`), and UI events (`.emit(...)`).
//! Every Tauri backend function now takes `AppHandle` (this file) instead,
//! so the module code stays 1:1 with the original:
//!
//! * `app.state()`            → the same `AppState` fields
//! * `app.data_dir()` etc.    → the same directories the old app used
//! * `app.emit(event, json)`  → queued to the UI thread over an mpsc channel
//! * `app.wake()`             → replaces `window.show()/set_focus()` focus calls

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub const APP_ID: &str = "com.werlos.moonhunt";
pub const APP_NAME: &str = "MoonHunt";
/// Current application version (drives the in-app updater).
pub const APP_VERSION: &str = "0.0.7";

/// Event delivered to the UI thread: (event name, JSON payload).
pub type Event = (String, serde_json::Value);
/// Backend → UI event channel sender.
pub type EventSender = mpsc::Sender<Event>;
/// Special event name the UI hijacks to show/unminimize/focus its window.
pub const EV_WAKE: &str = "__wake";

/// Shared application state. Field-for-field identical to the struct that
/// Tauri managed via `.manage(AppState { .. })` in the old `lib.rs`.
pub struct AppState {
    pub proxy_settings: Mutex<crate::state::ProxySettings>,
    pub pending_downloads: Arc<Mutex<Vec<crate::state::PendingDownload>>>,
    pub extension_settings: Arc<Mutex<crate::state::ExtensionSettings>>,
    pub extension_cookies: Arc<Mutex<Option<String>>>,
    pub active_yt_scan:
        Arc<Mutex<std::collections::HashMap<String, std::process::Child>>>,
    pub cancelled_yt_scans: Arc<Mutex<std::collections::HashSet<String>>>,
    pub active_yt_downloads: Arc<Mutex<std::collections::HashMap<String, u32>>>,
    /// GIDs the user paused via `pause_yt_download`. The download thread
    /// persists a `paused` row (instead of `error`) and exits; resume
    /// re-runs from the stored record and clears the entry.
    pub paused_yt_downloads: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            proxy_settings: Mutex::new(Default::default()),
            pending_downloads: Arc::new(Mutex::new(Vec::new())),
            extension_settings: Arc::new(Mutex::new(
                crate::state::ExtensionSettings {
                    extension_port: 6802,
                    extension_auth_enabled: false,
                    extension_auth_token: String::new(),
                    open_in_browser_protocol_enabled: false,
                },
            )),
            extension_cookies: Arc::new(Mutex::new(None)),
            active_yt_scan: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cancelled_yt_scans: Arc::new(Mutex::new(std::collections::HashSet::new())),
            active_yt_downloads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            paused_yt_downloads: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

/// Shareable backend handle passed to every migrated backend function
/// in place of the old `tauri::AppHandle`. Clone-capture friendly: any
/// module may `app.clone()` into `std::thread::spawn` / async blocks.
#[derive(Clone)]
pub struct AppHandle {
    inner: Arc<AppState>,
    events: EventSender,
}

impl AppHandle {
    /// Construct with a fresh receiver-owned events channel and default state.
    pub fn new() -> (Self, tokio::sync::mpsc::Receiver<Event>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);
        let handle = Self::with_events(tx);
        (handle, rx)
    }

    pub fn with_events(events: EventSender) -> Self {
        Self {
            inner: Arc::new(AppState::default()),
            events,
        }
    }

    /// Replaces Tauri's `app.state::<AppState>()`. Returns the shared state.
    /// (Non-generic on purpose — modules call `app.state()` directly.)
    pub fn state(&self) -> &AppState {
        &self.inner
    }

    pub fn state_arc(&self) -> Arc<AppState> {
        self.inner.clone()
    }

    /// Replaces `.emit(...)`: queues one event for the UI thread.
    /// Payload is anything JSON-serializable (the UI receives `serde_json::Value`).
    pub fn emit<E: Into<String>, P: serde::Serialize + std::fmt::Debug>(
        &self,
        event: E,
        payload: P,
    ) {
        match serde_json::to_value(&payload) {
            Ok(value) => {
                let _ = self.events.try_send((event.into(), value));
            }
            Err(e) => {
                eprintln!("[events] serialize failed for {}: {}", event.into(), e);
            }
        }
    }

    /// Like `emit`, but waits for channel space instead of dropping the event.
    /// Use this when a later event must not overtake or replace an earlier one.
    pub async fn emit_async<E: Into<String>, P: serde::Serialize + std::fmt::Debug>(
        &self,
        event: E,
        payload: P,
    ) {
        let event = event.into();
        match serde_json::to_value(&payload) {
            Ok(value) => {
                let _ = self.events.send((event, value)).await;
            }
            Err(e) => {
                eprintln!("[events] serialize failed for {event}: {e}");
            }
        }
    }

    /// Replaces `window.unminimize(); window.show(); window.set_focus();`
    /// used by the extension bridge and deep-link wake paths.
    pub fn wake(&self) {
        let _ = self.events.try_send((EV_WAKE.to_string(), serde_json::json!(())));
    }

    fn app_id(&self) -> String {
        // The desktop port runs sandboxed from the original Tauri app's data
        // dir unless explicitly pointed at it (parity testing).
        std::env::var("MOONHUNT_APP_ID").unwrap_or_else(|_| "com.werlos.moonhunt-rust".into())
    }

    /// Matches Tauri's `appDataDir`: `%APPDATA%\<id>` on Windows,
    /// `$XDG_DATA_HOME/<id>` (i.e. `~/.local/share/<id>`) on Linux.
    pub fn data_dir(&self) -> Result<PathBuf, String> {
        dirs::data_dir()
            .map(|base| base.join(self.app_id()))
            .ok_or_else(|| "Failed to resolve app data directory".to_string())
    }

    /// Alias kept for 1:1 parity with call sites ported from Tauri.
    pub fn app_data_dir(&self) -> Result<PathBuf, String> {
        self.data_dir()
    }

    /// Matches Tauri's `appLocalDataDir`: `%LOCALAPPDATA%\<id>` on Windows,
    /// same as `data_dir` on Linux.
    pub fn local_data_dir(&self) -> Result<PathBuf, String> {
        dirs::data_local_dir()
            .map(|base| base.join(self.app_id()))
            .ok_or_else(|| "Failed to resolve app local data directory".to_string())
    }

    /// Alias kept for 1:1 parity with call sites ported from Tauri.
    pub fn app_local_data_dir(&self) -> Result<PathBuf, String> {
        self.local_data_dir()
    }

    /// Matches Tauri's `appCacheDir` semantics closely enough for directory
    /// size accounting: a `cache` folder under the app's own data area.
    pub fn cache_dir(&self) -> Result<PathBuf, String> {
        self.local_data_dir().map(|base| base.join("cache"))
    }

    /// Alias kept for 1:1 parity with call sites ported from Tauri.
    pub fn app_cache_dir(&self) -> Result<PathBuf, String> {
        self.cache_dir()
    }

    /// Matches Tauri's `temp_dir`: the system temporary directory.
    pub fn temp_dir(&self) -> Result<PathBuf, String> {
        Ok(std::env::temp_dir())
    }
}

impl Default for AppHandle {
    fn default() -> Self {
        let (tx, _rx) = tokio::sync::mpsc::channel::<Event>(256);
        Self {
            inner: Arc::new(AppState::default()),
            events: tx,
        }
    }
}
