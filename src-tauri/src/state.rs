//! Shared application state, settings models, and globals.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex, OnceLock};

/// User-configurable settings persisted in SQLite.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxySettings {
    pub download_location: String,
    pub max_concurrent: u32,
    pub max_download_speed: u32,
    #[serde(default)]
    pub global_speed_limit_enabled: Option<bool>,
    #[serde(default)]
    pub js_runtime: Option<String>,
    #[serde(default)]
    pub media_merger: Option<String>,
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u32,
    pub proxy_user: String,
    pub proxy_pass: String,
    pub rpc_port: u32,
    pub extension_port: u32,
    #[serde(default)]
    pub extension_auth_enabled: bool,
    #[serde(default)]
    pub extension_auth_token: String,
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub theme_mode: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub download_limit_enabled: bool,
    #[serde(default)]
    pub download_limit_bytes: u64,
    #[serde(default)]
    pub download_limit_used_bytes: u64,
    #[serde(default = "default_download_limit_unit")]
    pub download_limit_unit: String,
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
    #[serde(default)]
    pub delete_file_on_cancel: bool,
    /// aria2: max-tries per task (global ceiling). Default 5.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// aria2: --retry per-URI retries before trying the next URI (default 1).
    #[serde(default = "default_retry_per_uri")]
    pub retry_per_uri: u32,
    /// aria2: --retry-wait seconds between retries (default 3).
    #[serde(default = "default_retry_wait")]
    pub retry_wait: u32,
    #[serde(default)]
    pub debug_logging: bool,
    #[serde(default)]
    pub disable_hardware_acceleration: bool,
    #[serde(default)]
    pub torrent_download_location: Option<String>,
    #[serde(default)]
    pub torrent_max_connections: Option<u32>,
    #[serde(default)]
    pub torrent_dht_enabled: Option<bool>,
    #[serde(default)]
    pub torrent_auto_start: Option<bool>,
    #[serde(default)]
    pub torrent_upload_limit_enabled: Option<bool>,
    #[serde(default)]
    pub torrent_upload_limit: Option<u32>,
    #[serde(default)]
    pub open_in_browser_protocol_enabled: bool,
    /// Titlebar search: show matching-downloads preview dropdown. Default on.
    #[serde(default = "default_true")]
    pub search_preview_enabled: bool,
}

fn default_true() -> bool { true }

fn default_download_limit_unit() -> String {
    "MB".to_string()
}

fn default_language() -> String { "en".to_string() }

fn default_max_retries() -> u32 { 5 }
fn default_retry_per_uri() -> u32 { 3 }
fn default_retry_wait() -> u32 { 3 }

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            download_location: default_download_dir(),
            max_concurrent: 5,
            max_download_speed: 0,
            global_speed_limit_enabled: None,
            js_runtime: None,
            media_merger: None,
            proxy_type: "none".to_string(),
            proxy_host: String::new(),
            proxy_port: 1080,
            proxy_user: String::new(),
            proxy_pass: String::new(),
            rpc_port: 6800,
            extension_port: 6802,
            extension_auth_enabled: false,
            extension_auth_token: String::new(),
            theme: "default".to_string(),
            theme_mode: "dark".to_string(),
            language: "en".to_string(),
            download_limit_enabled: false,
            download_limit_bytes: 0,
            download_limit_used_bytes: 0,
            download_limit_unit: "MB".to_string(),
            verify_ssl: true,
            delete_file_on_cancel: false,
            max_retries: 5,
            retry_per_uri: 3,
            retry_wait: 3,
            debug_logging: false,
            disable_hardware_acceleration: false,
            torrent_download_location: None,
            torrent_max_connections: None,
            torrent_dht_enabled: None,
            torrent_auto_start: None,
            torrent_upload_limit_enabled: None,
            torrent_upload_limit: None,
            open_in_browser_protocol_enabled: false,
            search_preview_enabled: true,
        }
    }
}

impl ProxySettings {
    /// Effective global download speed limit in KB/s, respecting the toggle.
    /// Legacy settings without the toggle keep working if a speed was set.
    pub fn global_max_download_speed(&self) -> u32 {
        let enabled = self
            .global_speed_limit_enabled
            .unwrap_or(self.max_download_speed > 0);
        if enabled {
            self.max_download_speed
        } else {
            0
        }
    }
}

/// Download list sort preferences persisted in SQLite.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadPreferences {
    #[serde(default = "default_download_sort")]
    pub sort: String,
    #[serde(default)]
    pub sort_ascending: bool,
}

fn default_download_sort() -> String {
    "added".to_string()
}

/// A download intercepted by the browser extension, awaiting user confirmation.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PendingDownload {
    pub url: String,
    pub filename: String,
    /// Extra headers from the browser extension (e.g. cookies for the source site)
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Cookie forwarded from the browser extension (Netscape cookie file source).
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<f64>,
    pub secure: bool,
}

/// Extension bridge settings (HTTP port + optional auth token).
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionSettings {
    pub extension_port: u32,
    #[serde(default)]
    pub extension_auth_enabled: bool,
    #[serde(default)]
    pub extension_auth_token: String,
    #[serde(default)]
    pub open_in_browser_protocol_enabled: bool,
}

/// Scheduled download action.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSchedule {
    pub id: Option<i64>,
    pub download_gid: Option<String>,
    pub url: String,
    pub filename: String,
    pub action: String, // "start", "pause", "resume"
    pub scheduled_time: i64,
    pub repeat_interval: Option<i64>,
    pub enabled: bool,
    pub created_at: i64,
}

/// Download history row for SQLite.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DownloadRecord {
    pub gid: String,
    pub filename: String,
    pub url: String,
    pub size: String,
    pub status: String,
    pub progress: i32,
    pub added_at: i64,
    pub path: Option<String>,
    pub connections: Option<i32>,
    pub source: Option<String>,
    pub ytdlp_token: Option<String>,
    pub proxy: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

/// Per-download proxy override passed when starting a single download.
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct PerDownloadProxy {
    pub enabled: bool,
    #[serde(default)]
    pub proxy_type: String,
    #[serde(default)]
    pub proxy_host: String,
    #[serde(default)]
    pub proxy_port: u32,
    #[serde(default)]
    pub proxy_user: String,
    #[serde(default)]
    pub proxy_pass: String,
}

/// Global application state shared across Tauri commands.
pub struct AppState {
    pub proxy_settings: Mutex<ProxySettings>,
    pub pending_downloads: Arc<Mutex<Vec<PendingDownload>>>,
    pub extension_settings: Arc<Mutex<ExtensionSettings>>,
    pub extension_cookies: Arc<Mutex<Option<String>>>,
    pub active_yt_scan: Arc<Mutex<std::collections::HashMap<String, std::process::Child>>>,
    pub cancelled_yt_scans: Arc<Mutex<std::collections::HashSet<String>>>,
    pub active_yt_downloads: Arc<Mutex<std::collections::HashMap<String, u32>>>,
}

pub static RPC_PORT: OnceLock<AtomicU32> = OnceLock::new();
pub static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Default aria2 RPC secret used by this app. Empty by default; require user to set one at install.
pub const ARIA2_RPC_TOKEN: &str = "";

/// Safely lock a mutex, converting poison errors to strings.
pub fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    mutex
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))
}

/// Get a guard to the global database connection. Must call `init_db` first.
pub fn db() -> Result<std::sync::MutexGuard<'static, Connection>, String> {
    DB.get()
        .ok_or_else(|| "Database not initialized".to_string())?
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))
}

pub fn current_rpc_port() -> u32 {
    RPC_PORT
        .get()
        .map(|p| p.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(6800)
}

pub fn set_current_rpc_port(port: u32) {
    let port_ref = RPC_PORT.get_or_init(|| AtomicU32::new(6800));
    port_ref.store(port, std::sync::atomic::Ordering::Relaxed);
}

/// Platform-aware fallback for the default download directory.
fn default_download_dir() -> String {
    dirs::download_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            #[cfg(target_os = "windows")]
            { "C:/Downloads".to_string() }
            #[cfg(not(target_os = "windows"))]
            {
                dirs::home_dir()
                    .map(|h| format!("{}/Downloads", h.display()))
                    .unwrap_or_else(|| "/tmp".to_string())
            }
        })
}

/// Get the default download directory based on the OS.
#[tauri::command]
pub fn get_default_download_dir() -> String {
    default_download_dir()
}
