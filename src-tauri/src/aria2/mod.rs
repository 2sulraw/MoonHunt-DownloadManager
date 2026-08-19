//! aria2 integration: JSON-RPC client, process lifecycle, and Tauri commands.

mod client;
mod process;

pub use client::Aria2Client;
pub use process::spawn_aria2;

use crate::state::{
    db, lock, set_current_rpc_port, AppState, DownloadRecord, DownloadSchedule, PendingDownload,
    PerDownloadProxy, ProxySettings,
};
use crate::util::{decode_thunder_url, extract_filename_from_url, sanitize_filename};
use urlencoding::decode as url_decode;
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::command;
use tauri::Manager;

static ARIA2_SPAWN_GUARD: AtomicBool = AtomicBool::new(false);
static ARIA2_LAST_SPAWN: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);
static ARIA2_HAS_SPAWNED: AtomicBool = AtomicBool::new(false);

/// Apply proxy credentials to a running aria2 instance via RPC.
async fn apply_proxy_creds(settings: &ProxySettings, client: &Aria2Client) {
    if settings.proxy_type != "none" && !settings.proxy_host.is_empty() {
        let mut cred_options = Map::new();
        if !settings.proxy_user.is_empty() {
            cred_options.insert("all-proxy-user".to_string(), json!(settings.proxy_user));
        }
        if !settings.proxy_pass.is_empty() {
            cred_options.insert("all-proxy-passwd".to_string(), json!(settings.proxy_pass));
        }
        if !cred_options.is_empty() {
            if let Err(e) = client.change_global_option(cred_options).await {
                println!("[!] Failed to apply proxy credentials: {}", e);
            } else {
                println!("[✓] Proxy credentials applied");
            }
        }
    }
}

use std::collections::HashMap;

/// Store headers from extension downloads so reqwest retry can use them.
static DOWNLOAD_HEADERS: std::sync::Mutex<Option<HashMap<String, HashMap<String, String>>>> =
    std::sync::Mutex::new(None);

fn get_download_headers() -> std::sync::MutexGuard<'static, Option<HashMap<String, HashMap<String, String>>>> {
    DOWNLOAD_HEADERS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Look up stored headers for a URL (used by reqwest retry fallback).
pub fn lookup_download_headers(url: &str) -> Option<HashMap<String, String>> {
    let map = DOWNLOAD_HEADERS.lock().unwrap_or_else(|e| e.into_inner());
    map.as_ref()?.get(url).cloned()
}

fn global_speed_limit_value(settings: &ProxySettings) -> String {
    let effective_speed_kb = settings.global_max_download_speed();
    if effective_speed_kb > 0 {
        (effective_speed_kb as u64 * 1024).to_string()
    } else {
        "0".to_string()
    }
}

// ── Process commands ──────────────────────────────────────────────

#[command]
pub async fn start_aria2_with_settings(
    settings: ProxySettings,
    app: tauri::AppHandle,
) -> Result<String, String> {
    {
        let state = app.state::<AppState>();
        let mut guard = lock(&state.proxy_settings)?;
        *guard = settings.clone();
    }

    // One-shot: only spawn once per app lifecycle
    if ARIA2_HAS_SPAWNED.load(Ordering::SeqCst) {
        println!("[aria2] Already spawned — skipping");
        return Ok("already spawned".into());
    }

    if ARIA2_SPAWN_GUARD.swap(true, Ordering::SeqCst) {
        println!("[aria2] Spawn already in progress — skipping");
        ARIA2_SPAWN_GUARD.store(false, Ordering::SeqCst);
        return Ok("spawn in progress".into());
    }

    *ARIA2_LAST_SPAWN.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
    spawn_aria2(&settings, &app, true)?;
    ARIA2_HAS_SPAWNED.store(true, Ordering::SeqCst);

    // Wait for aria2 to be ready, then apply proxy credentials via RPC
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let client = loop {
        if std::time::Instant::now() >= deadline {
            ARIA2_SPAWN_GUARD.store(false, Ordering::SeqCst);
            return Ok("aria2 started (proxy credentials may not be applied — timed out waiting)".into());
        }
        match Aria2Client::new() {
            Ok(c) => {
                if c.call_string("aria2.getVersion", vec![]).await.is_ok() {
                    break c;
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };

    // Apply proxy credentials via RPC
    apply_proxy_creds(&settings, &client).await;

    ARIA2_SPAWN_GUARD.store(false, Ordering::SeqCst);
    Ok("success".to_string())
}

// ── Settings → aria2 ──────────────────────────────────────────────

#[command]
pub async fn set_proxy(settings: ProxySettings, app: tauri::AppHandle) -> Result<String, String> {
    {
        let state = app.state::<AppState>();
        let mut guard = lock(&state.proxy_settings)?;
        *guard = settings.clone();
    }

    let proxy_url = if settings.proxy_type != "none" && !settings.proxy_host.is_empty() {
        let scheme = if settings.proxy_type == "socks5" {
            "socks5"
        } else {
            "http"
        };
        format!(
            "{}://{}:{}",
            scheme, settings.proxy_host, settings.proxy_port
        )
    } else {
        String::new()
    };

    let mut proxy_options = Map::new();
    proxy_options.insert("all-proxy".to_string(), json!(proxy_url));
    if !proxy_url.is_empty() {
        if !settings.proxy_user.is_empty() {
            proxy_options.insert("all-proxy-user".to_string(), json!(settings.proxy_user));
        }
        if !settings.proxy_pass.is_empty() {
            proxy_options.insert("all-proxy-passwd".to_string(), json!(settings.proxy_pass));
        }
    }

    let speed_limit = global_speed_limit_value(&settings);
    if speed_limit == "0" {
        println!("[*] Speed limit disabled (unlimited)");
    } else {
        println!("[*] Speed limit set to: {} bytes/s", speed_limit);
    }

    let client = Aria2Client::new()?;

    // Apply the speed limit first so new downloads pick it up immediately.
    let mut speed_options = Map::new();
    speed_options.insert("max-download-limit".to_string(), json!(speed_limit.clone()));
    if let Err(e) = client.change_global_option(speed_options).await {
        return Err(format!("Failed to apply global speed limit to aria2: {}", e));
    }

    // Proxy stays best-effort: aria2 may reject proxy types it does not
    // support, but the speed limit above must still take effect.
    if let Err(e) = client.change_global_option(proxy_options).await {
        println!("[!] changeGlobalOption (proxy) failed: {}", e);
    }

    // Apply the limit to active and queued (including paused) downloads.
    let mut dl_options = Map::new();
    dl_options.insert("max-download-limit".to_string(), json!(speed_limit));
    let (applied, failed) = client.change_option_on_all(dl_options).await?;
    if failed > 0 {
        println!(
            "[!] Global speed limit: applied to {} downloads, {} failed",
            applied, failed
        );
    }

    Ok(format!(
        "Settings saved and applied to {} active/waiting downloads",
        applied
    ))
}

#[command]
pub async fn apply_global_speed_limit(settings: ProxySettings) -> Result<String, String> {
    let speed_limit = global_speed_limit_value(&settings);
    if speed_limit == "0" {
        println!("[*] Applying unlimited download speed");
    } else {
        println!("[*] Applying download speed limit: {} bytes/s", speed_limit);
    }

    let client = Aria2Client::new()?;

    let mut global_options = Map::new();
    global_options.insert("max-download-limit".to_string(), json!(speed_limit.clone()));
    client
        .change_global_option(global_options)
        .await
        .map_err(|e| format!("Failed to apply global speed limit to aria2: {}", e))?;

    let mut dl_options = Map::new();
    dl_options.insert("max-download-limit".to_string(), json!(speed_limit));
    let (applied, failed) = client.change_option_on_all(dl_options).await?;
    if failed > 0 {
        println!(
            "[!] Global speed limit: applied to {} downloads, {} failed",
            applied, failed
        );
    }

    Ok(format!(
        "Global speed limit applied to {} active/waiting downloads",
        applied
    ))
}

#[command]
pub async fn apply_download_speed_limit(gid: String, speed_limit: u64) -> Result<String, String> {
    let client = Aria2Client::new()?;
    let mut options = Map::new();
    options.insert("max-download-limit".to_string(), json!(speed_limit.to_string()));
    client.change_option(&gid, options).await?;
    Ok(format!("Speed limit applied to {}", gid))
}

/// Set (or clear) the per-download proxy on a live download and persist it.
#[command]
pub async fn set_download_proxy(
    app: tauri::AppHandle,
    gid: String,
    proxy: Option<PerDownloadProxy>,
) -> Result<String, String> {
    let client = Aria2Client::new()?;

    let mut options = Map::new();
    let mut stored: Option<String> = None;
    if let Some(p) = proxy {
        if p.enabled && p.proxy_type != "none" && !p.proxy_host.is_empty() {
            let scheme = if p.proxy_type == "socks5" { "socks5" } else { "http" };
            let proxy_url = format!("{}://{}:{}", scheme, p.proxy_host, p.proxy_port);
            options.insert("all-proxy".to_string(), json!(proxy_url));
            if !p.proxy_user.is_empty() {
                options.insert("all-proxy-user".to_string(), json!(p.proxy_user));
            }
            if !p.proxy_pass.is_empty() {
                options.insert("all-proxy-passwd".to_string(), json!(p.proxy_pass));
            }
            stored = if p.proxy_user.is_empty() && p.proxy_pass.is_empty() {
                Some(proxy_url)
            } else {
                Some(format!(
                    "{}://{}:{}@{}:{}",
                    scheme, p.proxy_user, p.proxy_pass, p.proxy_host, p.proxy_port
                ))
            };
        }
    }
    // Clear any previous per-download proxy options when disabled.
    if stored.is_none() {
        options.insert("all-proxy".to_string(), json!(""));
        options.insert("all-proxy-user".to_string(), json!(""));
        options.insert("all-proxy-passwd".to_string(), json!(""));
    }

    if !options.is_empty() {
        // Best-effort: the download may be finished/removed and only exist in
        // the DB (aria2 no longer holds the GID) — the setting must still be
        // persisted below.
        if let Err(e) = client.change_option(&gid, options).await {
            println!("[!] changeOption (per-download proxy) for {} failed: {}", gid, e);
        }
    }

    // Persist the change so it shows in the list/details across restarts.
    if let Ok(Some(row)) = crate::db::get_download_by_gid(app.clone(), gid.clone()) {
        let record = DownloadRecord {
            gid: row.get("gid").and_then(|v| v.as_str()).unwrap_or(&gid).to_string(),
            filename: row.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            url: row.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            size: row.get("size").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
            status: row.get("status").and_then(|v| v.as_str()).unwrap_or("active").to_string(),
            progress: row.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(0),
            path: row.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()),
            connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as i32),
            source: row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
            ytdlp_token: row.get("ytdlp_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
            proxy: stored,
        };
        let _ = crate::db::upsert_download(app, record);
    }

    Ok(format!("Proxy updated for {}", gid))
}

// ── Query / control ───────────────────────────────────────────────

#[command]
pub async fn get_aria2_version() -> Result<Value, String> {
    Aria2Client::new()?.get_version().await
}

#[command]
pub async fn get_all_downloads() -> Result<Vec<Value>, String> {
    let client = Aria2Client::new()?;
    let (active, waiting, stopped) = futures_util::future::join3(
        client.tell_active(),
        client.tell_waiting(0, 100),
        client.tell_stopped(0, 1000),
    )
    .await;

    let mut all = Vec::new();
    all.extend(active?);
    all.extend(waiting?);
    all.extend(stopped?);
    Ok(all)
}

#[command]
pub async fn get_download_details(gid: String) -> Result<Value, String> {
    Aria2Client::new()?.tell_status(&gid).await
}

#[command]
pub async fn pause_download(gid: String) -> Result<String, String> {
    println!("[>] Pause request: {}", gid);
    let result = Aria2Client::new()?.pause(&gid).await?;
    println!("[✓] Paused successfully: {}", result);
    Ok(result)
}

#[command]
pub async fn resume_download(gid: String) -> Result<String, String> {
    Aria2Client::new()?.unpause(&gid).await
}

#[command]
pub async fn pause_all_downloads() -> Result<String, String> {
    let client = Aria2Client::new()?;
    let (active, waiting) = futures_util::future::join(
        client.tell_active(),
        client.tell_waiting(0, 100),
    )
    .await;
    let mut paused = 0u32;
    for item in active?.into_iter().chain(waiting?) {
        if let Some(gid) = item.get("gid").and_then(|g| g.as_str()) {
            if client.pause(gid).await.is_ok() {
                paused += 1;
            }
        }
    }
    println!("[~] Paused {} downloads (limit reached)", paused);
    Ok(format!("Paused {} downloads", paused))
}

fn default_add_options(filename: &str, connections: u32, dir: Option<&str>) -> Map<String, Value> {
    let mut options = Map::new();
    options.insert("out".to_string(), json!(filename));
    options.insert("split".to_string(), json!(connections.to_string()));
    options.insert("max-connection-per-server".to_string(), json!(connections.to_string()));
    options.insert("min-split-size".to_string(), json!("1M"));
    options.insert("allow-overwrite".to_string(), json!("false"));
    options.insert("auto-file-renaming".to_string(), json!("true"));
    // Retry settings — prevent single-connection failures from killing the download
    options.insert("max-tries".to_string(), json!("5"));
    options.insert("retry".to_string(), json!("3"));
    options.insert("retry-wait".to_string(), json!("3"));
    options.insert("timeout".to_string(), json!("120"));
    options.insert("connect-timeout".to_string(), json!("60"));
    options.insert("socket-timeout".to_string(), json!("120"));
    if let Some(d) = dir {
        if !d.trim().is_empty() {
            options.insert("dir".to_string(), json!(d));
        }
    }
    options
}

/// Inject per-download proxy options into an aria2 addUri/addTorrent options map.
/// Returns the serialized proxy string for the DB, or None when not enabled.
fn apply_per_download_proxy(options: &mut Map<String, Value>, proxy: Option<&PerDownloadProxy>) -> Option<String> {
    let proxy = proxy?.clone();
    if !proxy.enabled || proxy.proxy_type == "none" || proxy.proxy_host.is_empty() {
        return None;
    }
    let scheme = if proxy.proxy_type == "socks5" { "socks5" } else { "http" };
    let proxy_url = format!("{}://{}:{}", scheme, proxy.proxy_host, proxy.proxy_port);
    options.insert("all-proxy".to_string(), json!(proxy_url));
    if !proxy.proxy_user.is_empty() {
        options.insert("all-proxy-user".to_string(), json!(proxy.proxy_user));
    }
    if !proxy.proxy_pass.is_empty() {
        options.insert("all-proxy-passwd".to_string(), json!(proxy.proxy_pass));
    }
    if proxy.proxy_user.is_empty() && proxy.proxy_pass.is_empty() {
        Some(proxy_url)
    } else {
        Some(format!(
            "{}://{}:{}@{}:{}",
            scheme, proxy.proxy_user, proxy.proxy_pass, proxy.proxy_host, proxy.proxy_port
        ))
    }
}

#[command]
pub async fn add_download(
    app: tauri::AppHandle,
    url: String,
    filename: Option<String>,
    connections: Option<u32>,
    dir: Option<String>,
    schedule: Option<DownloadSchedule>,
    proxy: Option<PerDownloadProxy>,
) -> Result<String, String> {
    let download_url = decode_thunder_url(&url).unwrap_or_else(|| url.clone());
    let thunder_tail = decode_thunder_url(&url).map(|_| {
        url.get("thunder://".len()..)
            .unwrap_or("")
            .trim()
            .to_string()
    });

    let filename = match filename {
        Some(f) if !f.trim().is_empty() => {
            let f = f.trim().to_string();
            let looks_auto = f == url
                || f == "download"
                || f.contains("://")
                || thunder_tail.as_deref() == Some(f.as_str());
            if looks_auto {
                extract_filename_from_url(&download_url)
            } else {
                f
            }
        }
        _ => extract_filename_from_url(&download_url),
    };
    let filename = url_decode(&filename)
        .map(|c| c.to_string())
        .unwrap_or_else(|_| filename.clone());
    let filename = sanitize_filename(&filename);

    let effective_dir = if let Some(ref d) = dir {
        if !d.trim().is_empty() {
            d.clone()
        } else {
            default_download_dir()?
        }
    } else {
        default_download_dir()?
    };
    std::fs::create_dir_all(&effective_dir)
        .map_err(|e| format!("Failed to create download folder: {}", e))?;
    let is_duplicate = std::path::Path::new(&effective_dir)
        .join(&filename)
        .exists();

    let connections = connections.unwrap_or(1).max(1);

    // Extra headers for the download (set by download URL resolution)
    let extra_headers: Vec<String> = Vec::new();

    println!(
        "[~] URL: {} | Filename: {} | Connections: {} | Duplicate: {}",
        download_url, filename, connections, is_duplicate
    );

    let mut options = default_add_options(&filename, connections, Some(&effective_dir));
    let proxy_db = apply_per_download_proxy(&mut options, proxy.as_ref());
    if let Some(p) = &proxy_db {
        println!("[~] Per-download proxy set: {}", p);
    }
    if !extra_headers.is_empty() {
        options.insert("header".to_string(), json!(extra_headers));
    }
    let client = Aria2Client::new()?;
    let gid = client.add_uri(&[download_url.clone()], options).await?;

    // Persist the record at add time so per-download settings (proxy) survive
    // for active downloads, not only on terminal state.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let record = DownloadRecord {
        gid: gid.clone(),
        filename: filename.clone(),
        url: download_url.clone(),
        size: "0".to_string(),
        status: "active".to_string(),
        progress: 0,
        added_at: now_ms,
        path: Some(effective_dir.clone()),
        connections: Some(connections as i32),
        source: Some("aria2".to_string()),
        ytdlp_token: None,
        proxy: proxy_db.clone(),
    };
    if let Err(e) = crate::db::upsert_download(app, record) {
        println!("[!] Failed to persist new download record: {}", e);
    }

    if let Some(mut sched) = schedule {
        sched.download_gid = Some(gid.clone());
        let _ = crate::db::create_schedule(sched);
    }

    Ok(json!({"gid": gid, "duplicate": is_duplicate}).to_string())
}

/// Add a BitTorrent download — magnet link or .torrent file.
///
/// - `magnet`: a `magnet:?xt=urn:btih:...` URI → passed to `aria2.addUri`
///   (aria2 resolves the DHT/tracker swarm natively).
/// - `torrent_path`: absolute path to a `.torrent` file → read + base64-encode
///   the content and call `aria2.addTorrent` with the download dir option.
#[command]
pub async fn add_torrent(
    magnet: Option<String>,
    torrent_path: Option<String>,
    dir: Option<String>,
    selected_files: Option<Vec<u32>>,
) -> Result<String, String> {
    use base64::Engine as _;

    let effective_dir = if let Some(ref d) = dir {
        if !d.trim().is_empty() {
            d.clone()
        } else {
            default_download_dir()?
        }
    } else {
        default_download_dir()?
    };

    let client = Aria2Client::new()?;

    // Magnet link → addUri (aria2 treats it as a BitTorrent magnet)
    if let Some(m) = magnet {
        let m = m.trim().to_string();
        if m.is_empty() {
            return Err("Magnet link cannot be empty".to_string());
        }
        if !m.to_lowercase().starts_with("magnet:") {
            return Err("Not a valid magnet link (must start with magnet:)".to_string());
        }
        println!("[~] Adding magnet: {} → {}", m, effective_dir);

        let mut options = Map::new();
        options.insert("dir".to_string(), json!(effective_dir));
        // Magnet + file selection: aria2 can't select files before metadata
        // is fetched, so this is only honored for .torrent files.
        let gid = client.add_uri(&[m], options).await?;
        println!("[✓] Magnet added, GID: {}", gid);
        return Ok(json!({"gid": gid, "type": "magnet"}).to_string());
    }

    // .torrent file → read + base64 → addTorrent
    if let Some(p) = torrent_path {
        let p = p.trim().to_string();
        if p.is_empty() {
            return Err("Torrent file path cannot be empty".to_string());
        }
        let path = std::path::Path::new(&p);
        if !path.exists() {
            return Err(format!("Torrent file not found: {}", p));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read torrent file {}: {}", p, e))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        println!("[~] Adding .torrent {} ({} bytes) → {}", p, bytes.len(), effective_dir);

        let mut options = Map::new();
        options.insert("dir".to_string(), json!(effective_dir));
        // Restrict to selected files if a subset was chosen (aria2 select-file,
        // 1-based indices). When null, all files download.
        if let Some(files) = selected_files {
            if !files.is_empty() {
                let sel = files
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                options.insert("select-file".to_string(), json!(sel));
                println!("[~] select-file={}", sel);
            }
        }
        let gid = client.add_torrent(&b64, &[], options).await?;
        println!("[✓] Torrent added, GID: {}", gid);
        return Ok(json!({"gid": gid, "type": "torrent"}).to_string());
    }

    Err("Provide either a magnet link or a .torrent file".to_string())
}

/// Parse a .torrent file and return the list of contained files (name + size).
/// Uses a minimal bencode parser — no external crate needed.
#[command]
pub fn parse_torrent_files(torrent_path: String) -> Result<Vec<serde_json::Value>, String> {
    let path = std::path::Path::new(&torrent_path);
    if !path.exists() {
        return Err(format!("Torrent file not found: {}", torrent_path));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read torrent file {}: {}", torrent_path, e))?;

    // Minimal bencode parser: we only need to walk the top-level dict and find
    // "name" / "files" keys (multi-file) or "name" + "length" (single-file).
    fn parse_bencode(data: &[u8], pos: &mut usize) -> Result<serde_json::Value, String> {
        if *pos >= data.len() {
            return Err("Unexpected end of torrent data".to_string());
        }
        match data[*pos] {
            b'i' => {
                *pos += 1;
                let start = *pos;
                while *pos < data.len() && data[*pos] != b'e' {
                    *pos += 1;
                }
                if *pos >= data.len() {
                    return Err("Unterminated integer in torrent".to_string());
                }
                let s = std::str::from_utf8(&data[start..*pos])
                    .map_err(|_| "Invalid integer in torrent".to_string())?;
                *pos += 1; // consume 'e'
                Ok(serde_json::Value::from(s.parse::<i64>().map_err(|_| "Invalid integer value".to_string())?))
            }
            b'l' => {
                *pos += 1;
                let mut arr = Vec::new();
                while *pos < data.len() && data[*pos] != b'e' {
                    arr.push(parse_bencode(data, pos)?);
                }
                if *pos >= data.len() {
                    return Err("Unterminated list in torrent".to_string());
                }
                *pos += 1; // consume 'e'
                Ok(serde_json::Value::Array(arr))
            }
            b'd' => {
                *pos += 1;
                let mut map = serde_json::Map::new();
                while *pos < data.len() && data[*pos] != b'e' {
                    let key = parse_bencode(data, pos)?;
                    let val = parse_bencode(data, pos)?;
                    if let Some(k) = key.as_str() {
                        map.insert(k.to_string(), val);
                    }
                }
                if *pos >= data.len() {
                    return Err("Unterminated dict in torrent".to_string());
                }
                *pos += 1; // consume 'e'
                Ok(serde_json::Value::Object(map))
            }
            b'0'..=b'9' => {
                let start = *pos;
                while *pos < data.len() && data[*pos] != b':' {
                    *pos += 1;
                }
                if *pos >= data.len() {
                    return Err("Unterminated string length in torrent".to_string());
                }
                let len_str = std::str::from_utf8(&data[start..*pos])
                    .map_err(|_| "Invalid string length".to_string())?;
                let len: usize = len_str.parse().map_err(|_| "Invalid string length value".to_string())?;
                *pos += 1; // consume ':'
                if *pos + len > data.len() {
                    return Err("String length exceeds torrent data".to_string());
                }
                let s = &data[*pos..*pos + len];
                *pos += len;
                // Try UTF-8; if not valid, fall back to hex/escaped
                match std::str::from_utf8(s) {
                    Ok(utf) => Ok(serde_json::Value::String(utf.to_string())),
                    Err(_) => Ok(serde_json::Value::String(format!("<binary {} bytes>", s.len()))),
                }
            }
            _ => Err(format!("Unexpected byte in torrent: {}", data[*pos])),
        }
    }

    let mut pos = 0usize;
    let root = parse_bencode(&bytes, &mut pos)?;
    let info = root.get("info").ok_or("No info dict in torrent")?;

    let mut files: Vec<serde_json::Value> = Vec::new();

    // Multi-file torrent: info.files = [{ path: [...], length: N }, ...]
    if let Some(file_list) = info.get("files").and_then(|v| v.as_array()) {
        let base_name = info.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        for f in file_list {
            let path_parts: Vec<String> = f
                .get("path")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let rel = if base_name.is_empty() {
                path_parts.join("/")
            } else {
                std::path::Path::new(&base_name)
                    .join(path_parts.join("/"))
                    .to_string_lossy()
                    .to_string()
            };
            let len = f.get("length").and_then(|v| v.as_i64()).unwrap_or(0);
            files.push(serde_json::json!({ "path": rel, "size": len }));
        }
    } else {
        // Single-file torrent: info.name + info.length
        let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        let len = info.get("length").and_then(|v| v.as_i64()).unwrap_or(0);
        files.push(serde_json::json!({ "path": name, "size": len }));
    }

    Ok(files)
}

fn default_download_dir() -> Result<String, String> {
    let conn = db()?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = 'download_location'")
        .map_err(|e| format!("Query error: {}", e))?;
    let d: String = stmt
        .query_row([], |row| row.get(0))
        .unwrap_or_else(|_| {
            dirs::download_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "Downloads".to_string())
        });
    Ok(d)
}

#[command]
pub async fn confirm_download_action(
    app: tauri::AppHandle,
    url: String,
    filename: String,
    connections: Option<u32>,
    dir: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    proxy: Option<PerDownloadProxy>,
) -> Result<String, String> {
    let connections = connections.unwrap_or(1).max(1);

    let download_url = decode_thunder_url(&url).unwrap_or_else(|| url.clone());
    let thunder_tail = decode_thunder_url(&url).map(|_| {
        url.get("thunder://".len()..)
            .unwrap_or("")
            .trim()
            .to_string()
    });

    let filename = if filename.trim().is_empty()
        || filename.trim() == url
        || filename.trim() == "download"
        || filename.contains("://")
        || thunder_tail.as_deref() == Some(filename.trim())
    {
        extract_filename_from_url(&download_url)
    } else {
        filename
    };

    // Decode URL-encoded filenames returned by provider APIs
    let filename = url_decode(&filename)
        .map(|c| c.to_string())
        .unwrap_or_else(|_| filename.clone());
    // Strip path separators / illegal characters (mirrors add_download) so a
    // server-provided Content-Disposition name can never escape the target dir.
    let filename = sanitize_filename(&filename);

    let effective_dir = if let Some(ref d) = dir {
        if !d.trim().is_empty() {
            d.clone()
        } else {
            default_download_dir()?
        }
    } else {
        default_download_dir()?
    };
    std::fs::create_dir_all(&effective_dir)
        .map_err(|e| format!("Failed to create download folder: {}", e))?;

    // Merge headers from the browser extension
    let mut options = default_add_options(&filename, connections, Some(&effective_dir));
    let proxy_db = apply_per_download_proxy(&mut options, proxy.as_ref());
    if let Some(p) = &proxy_db {
        println!("[confirm_download_action] Per-download proxy set: {}", p);
    }

    let mut header_list: Vec<String> = Vec::new();
    if let Some(hdrs) = headers {
        if !hdrs.is_empty() {
            let mut has_ua = false;
            for (k, v) in &hdrs {
                if k.eq_ignore_ascii_case("protocol") {
                    continue;
                }
                header_list.push(format!("{}: {}", k, v));
                if k.to_lowercase() == "user-agent" { has_ua = true; }
            }
            // Add browser-like User-Agent if extension didn't send one
            if !has_ua {
                header_list.push("User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36".to_string());
            }
            println!("[confirm_download_action] Passing {} headers to aria2: {:?}", header_list.len(), header_list);
            options.insert("header".to_string(), json!(header_list));
            // Store headers for reqwest retry fallback
            {
                let mut map = get_download_headers();
                if map.is_none() { *map = Some(HashMap::new()); }
                let mut filtered = hdrs.clone();
                filtered.retain(|k, _| !k.eq_ignore_ascii_case("protocol"));
                map.as_mut().unwrap().insert(url.clone(), filtered);
            }
        } else {
            println!("[confirm_download_action] No headers to pass to aria2");
        }
    } else {
        println!("[confirm_download_action] headers=None — no headers will be sent to aria2");
    }

    let client = Aria2Client::new()?;

    println!("[>] Adding download from extension: {} as {}", download_url, filename);
    let gid = client.add_uri(&[download_url], options.clone()).await?;
    let _ = client.change_option(&gid, options).await;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let record = DownloadRecord {
        gid: gid.clone(),
        filename: filename.clone(),
        url: url.clone(),
        size: "0".to_string(),
        status: "active".to_string(),
        progress: 0,
        added_at: now_ms,
        path: Some(effective_dir.clone()),
        connections: Some(connections as i32),
        source: Some("aria2".to_string()),
        ytdlp_token: None,
        proxy: proxy_db,
    };
    if let Err(e) = crate::db::upsert_download(app, record) {
        println!("[!] Failed to persist confirm download record: {}", e);
    }

    println!("[✓] Download added successfully: {}", gid);
    Ok(gid)
}

#[command]
pub async fn remove_download(app: tauri::AppHandle, gid: String) -> Result<String, String> {
    // Look up the download record for file path info (before removing it)
    let record = crate::db::get_download_by_gid(app.clone(), gid.clone())
        .ok()
        .flatten();
    let filename = record
        .as_ref()
        .and_then(|r| r.get("filename").and_then(|f| f.as_str()))
        .unwrap_or("")
        .to_string();

    let client = Aria2Client::new()?;
    // Mark cancelled FIRST so the polling loop never shows "complete" for this GID
    crate::extension::mark_cancelled(&gid);
    println!("[>] Attempting to remove active download: {}", gid);

    let mut removed_ok = false;

    match client.remove(&gid).await {
        Ok(result) if !result.is_null() => {
            println!("[✓] Successfully removed active download: {}", gid);
            removed_ok = true;
        }
        _ => {}
    }

    if !removed_ok {
        println!("[>] Attempting to remove from stopped downloads: {}", gid);
        match client.remove_download_result(&gid).await {
            Ok(result) if !result.is_null() => {
                println!("[✓] Successfully removed stopped download: {}", gid);
                removed_ok = true;
            }
            Err(e) => {
                let lower = e.to_lowercase();
                if lower.contains("not found") || e.contains("Invalid GID") {
                    println!(
                        "[✓] Aria2 reports missing/invalid GID for {}: {}. Removing local DB entry.",
                        gid, e
                    );
                    crate::db::remove_download_from_db(app.clone(), gid.clone())?;
                    removed_ok = true;
                } else {
                    return Err(format!("[✗] Failed to remove download {}: {}", gid, e));
                }
            }
            Ok(_) => {}
        }
    }

    // Delete file from disk if setting is enabled (best-effort, never fail the command)
    if removed_ok && !filename.is_empty() {
        if let Err(e) = delete_file_if_enabled(&filename) {
            println!("[!] File deletion failed (non-fatal): {}", e);
        }
    }

    if removed_ok {
        Ok(gid)
    } else {
        Err(format!("[✗] Failed to remove download {}", gid))
    }
}

/// If `delete_file_on_cancel` is enabled in settings, delete the file from disk.
/// Also deletes the aria2 control file (`.aria2` sidecar) if present.
fn delete_file_if_enabled(filename: &str) -> Result<(), String> {
    // Read setting directly from DB to avoid stale AppState
    let settings = crate::db::load_settings_from_db()?;
    if !settings.delete_file_on_cancel {
        return Ok(());
    }
    let dir = &settings.download_location;

    // Resolve canonical download directory first
    let canonical_dir = match std::fs::canonicalize(dir) {
        Ok(p) => p,
        Err(_) => {
            // If the configured download dir cannot be resolved, refuse to delete to be safe
            return Err(format!("Configured download directory does not exist: {}", dir));
        }
    };

    let file_path = std::path::Path::new(dir).join(filename);

    // If file doesn't exist, treat as no-op
    if !file_path.exists() {
        println!("[remove] File not found (already deleted?): {}", file_path.display());
        return Ok(());
    }

    // Canonicalize the target file and ensure it's inside the configured download dir
    let canonical_file = match std::fs::canonicalize(&file_path) {
        Ok(p) => p,
        Err(e) => {
            println!("[remove] Failed to canonicalize file {}: {}", file_path.display(), e);
            return Ok(());
        }
    };

    if !canonical_file.starts_with(&canonical_dir) {
        // Refuse to delete files outside the configured download directory
        println!("[!] Refusing to delete file outside download dir: {}", canonical_file.display());
        return Err(format!("Refusing to delete file outside download directory: {}", canonical_file.display()));
    }

    // Delete the main file
    println!("[remove] Deleting file: {}", canonical_file.display());
    std::fs::remove_file(&canonical_file)
        .map_err(|e| format!("Failed to delete file {}: {}", canonical_file.display(), e))?;
    println!("[✓] Deleted file: {}", canonical_file.display());

    // Delete aria2 control file (.aria2 sidecar)
    let control_file = format!("{}.aria2", canonical_file.display());
    let control_path = std::path::Path::new(&control_file);
    if control_path.exists() {
        println!("[remove] Deleting aria2 control file: {}", control_path.display());
        let _ = std::fs::remove_file(control_path);
    }

    Ok(())
}

#[command]
pub async fn retry_download(app: tauri::AppHandle, gid: String) -> Result<String, String> {
    println!("[~] Retrying download for GID: {}", gid);

    let record = crate::db::get_download_by_gid(app.clone(), gid.clone())
        .ok()
        .flatten();

    let mut url = record
        .as_ref()
        .and_then(|r| r.get("url").and_then(|u| u.as_str()))
        .unwrap_or("")
        .to_string();

    let filename_opt: Option<String> = record
        .as_ref()
        .and_then(|r| r.get("filename").and_then(|f| f.as_str()))
        .map(|s| s.to_string());

    let connections_opt: Option<u32> = record
        .as_ref()
        .and_then(|r| {
            r.get("connections")
                .and_then(|s| s.as_i64())
                .or_else(|| r.get("connections").and_then(|s| s.as_u64().map(|v| v as i64)))
        })
        .map(|s| s.max(1) as u32);

    let source = record
        .as_ref()
        .and_then(|r| r.get("source").and_then(|s| s.as_str()))
        .unwrap_or("")
        .to_string();

    let dir_opt: Option<String> = record.as_ref().and_then(|r| {
        r.get("path")
            .and_then(|p| p.as_str())
            .and_then(|p| {
                std::path::Path::new(p)
                    .parent()
                    .map(|parent| parent.to_string_lossy().to_string())
            })
    });

    if url.is_empty() {
        if let Ok(details) = get_download_details(gid.clone()).await {
            if let Some(files) = details.get("files").and_then(|f| f.as_array()) {
                if let Some(first) = files.first() {
                    if let Some(uris) = first.get("uris").and_then(|u| u.as_array()) {
                        if let Some(first_uri) = uris.first() {
                            if let Some(uri) = first_uri.get("uri").and_then(|v| v.as_str()) {
                                url = uri.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    if url.is_empty() {
        return Err("Could not determine original URL for retry".to_string());
    }

    let _ = remove_download(app.clone(), gid.clone()).await;
    let _ = crate::db::remove_download_from_db(app.clone(), gid.clone());

    println!("[~] Re-adding for retry. source={}, url={}", source, url);

    if source == "yt-dlp" {
        crate::ytdlp::add_yt_download(app, url, "best".to_string(), None, dir_opt, None, None).await
    } else {
        add_download(app, url, filename_opt, connections_opt, dir_opt, None, None).await
    }
}

#[command]
pub async fn get_pending_downloads(app: tauri::AppHandle) -> Result<Vec<PendingDownload>, String> {
    let state = app.state::<AppState>();
    let downloads = lock(&state.pending_downloads)?.clone();
    Ok(downloads)
}

#[command]
pub async fn remove_pending_download(index: usize, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut downloads = lock(&state.pending_downloads)?;
    if index < downloads.len() {
        downloads.remove(index);
        Ok(())
    } else {
        Err("Invalid index".to_string())
    }
}

// Re-export set_current_rpc_port usage for settings commands that live in db
#[allow(dead_code)]
pub fn touch_rpc_port(port: u32) {
    set_current_rpc_port(port);
}

/// Check if proxy is configured from the app settings (not via aria2 RPC — too flaky).
#[command]
pub async fn check_aria2_proxy(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let state = app.state::<AppState>();
    let settings = lock(&state.proxy_settings)?;
    let configured = settings.proxy_type != "none" && !settings.proxy_host.is_empty();
    let proxy = if configured {
        let scheme = if settings.proxy_type == "socks5" { "socks5" } else { "http" };
        format!("{}://{}:{}", scheme, settings.proxy_host, settings.proxy_port)
    } else {
        String::new()
    };
    let has_auth = !settings.proxy_user.is_empty();
    println!("[check-aria2-proxy] configured={}, proxy={}, hasAuth={}", configured, proxy, has_auth);
    Ok(serde_json::json!({
        "configured": configured,
        "proxy": proxy,
        "hasAuth": has_auth,
    }))
}

/// Test proxy connection by making a request through the configured proxy.
#[command]
pub async fn test_proxy(
    proxy_type: String,
    proxy_host: String,
    proxy_port: u32,
    proxy_user: Option<String>,
    proxy_pass: Option<String>,
) -> Result<serde_json::Value, String> {
    let start = std::time::Instant::now();

    let proxy_url = if proxy_type == "socks5" {
        format!("socks5://{}:{}", proxy_host, proxy_port)
    } else {
        format!("http://{}:{}", proxy_host, proxy_port)
    };

    println!("[proxy-test] Testing proxy: {}", proxy_url);

    let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| format!("Invalid proxy URL: {}", e))?;

    let proxy = if let (Some(user), Some(pass)) = (&proxy_user, &proxy_pass) {
        if !user.is_empty() {
            proxy.basic_auth(user, pass)
        } else {
            proxy
        }
    } else {
        proxy
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .proxy(proxy)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    match client.get("https://httpbin.org/ip").send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            println!("[proxy-test] Proxy OK: {} ({}ms) — {}", status, elapsed, body.trim());
            Ok(serde_json::json!({
                "ok": status >= 200 && status < 400,
                "status": status,
                "latencyMs": elapsed,
                "message": format!("Connected through proxy ({}ms)", elapsed),
                "ip": serde_json::from_str::<serde_json::Value>(&body)
                    .ok().and_then(|v| v.get("origin").cloned())
                    .unwrap_or(serde_json::Value::Null),
            }))
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            println!("[proxy-test] Proxy FAILED ({}ms): {}", elapsed, e);
            Ok(serde_json::json!({
                "ok": false,
                "status": 0,
                "latencyMs": elapsed,
                "message": format!("Connection failed: {}", e),
            }))
        }
    }
}

/// Test whether the configured proxy can reach a specific download URL.
#[command]
pub async fn test_proxy_url(
    app: tauri::AppHandle,
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<serde_json::Value, String> {
    let start = std::time::Instant::now();
    let state = app.state::<AppState>();
    let settings = lock(&state.proxy_settings)?.clone();

    let proxy_url = if settings.proxy_type != "none" && !settings.proxy_host.is_empty() {
        let scheme = if settings.proxy_type == "socks5" { "socks5" } else { "http" };
        format!("{}://{}:{}", scheme, settings.proxy_host, settings.proxy_port)
    } else {
        String::new()
    };

    if proxy_url.is_empty() {
        return Ok(serde_json::json!({
            "ok": false,
            "status": 0,
            "latencyMs": 0,
            "direct": true,
            "message": "Proxy is disabled",
        }));
    }

    println!("[proxy-url-test] Testing {} through {}", url, proxy_url);

    let mut proxy = reqwest::Proxy::all(&proxy_url)
        .map_err(|e| format!("Invalid proxy URL: {}", e))?;
    if !settings.proxy_user.is_empty() {
        proxy = proxy.basic_auth(&settings.proxy_user, &settings.proxy_pass);
    }

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36")
        .danger_accept_invalid_certs(!settings.verify_ssl)
        .proxy(proxy);

    if let Some(hdrs) = headers {
        let mut hdr_map = reqwest::header::HeaderMap::new();
        for (k, v) in &hdrs {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                if name == reqwest::header::USER_AGENT {
                    continue;
                }
                hdr_map.insert(name, val);
            }
        }
        builder = builder.default_headers(hdr_map);
    }

    let client = builder
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let (resp, method) = match client.head(&url).send().await {
        Ok(resp) if !matches!(resp.status().as_u16(), 403 | 405 | 501) => (resp, "HEAD"),
        Ok(_) | Err(_) => {
            match client
                .get(&url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await
            {
                Ok(resp) => (resp, "GET (range)"),
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    println!("[proxy-url-test] {} FAILED ({}ms): {}", url, elapsed, e);
                    return Ok(serde_json::json!({
                        "ok": false,
                        "status": 0,
                        "latencyMs": elapsed,
                        "method": "GET (range)",
                        "error": e.to_string(),
                    }));
                }
            }
        }
    };

    let elapsed = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();
    let status_text = resp.status().canonical_reason().unwrap_or("").to_string();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let content_length = resp.content_length().unwrap_or(0);
    let ok = status >= 200 && status < 400;
    let message = if ok {
        format!("URL reachable through proxy ({} in {}ms)", status, elapsed)
    } else {
        format!("URL not reachable through proxy (HTTP {} {})", status, status_text)
    };

    println!("[proxy-url-test] {} → {} ({}ms, {})", url, status, elapsed, message);

    Ok(serde_json::json!({
        "ok": ok,
        "status": status,
        "statusText": status_text,
        "latencyMs": elapsed,
        "contentLength": content_length,
        "contentType": content_type,
        "method": method,
        "message": message,
    }))
}

/// Detect system HTTP proxy settings.
/// Returns { enabled, host, port } — works on Windows, macOS, and Linux.
#[command]
pub async fn detect_system_proxy() -> Result<serde_json::Value, String> {
    let result = detect_proxy_inner()?;
    println!("[detect-proxy] enabled={}, host='{}', port={}", result.0, result.1, result.2);
    Ok(serde_json::json!({
        "enabled": result.0,
        "host": result.1,
        "port": result.2,
        "message": if result.0 { format!("Detected: {}:{}", result.1, result.2) } else { "No system proxy detected".into() }
    }))
}

/// (enabled, host, port)
fn detect_proxy_inner() -> Result<(bool, String, u32), String> {
    #[cfg(target_os = "windows")]
    { return detect_proxy_windows(); }

    #[cfg(target_os = "macos")]
    { return detect_proxy_macos(); }

    #[cfg(target_os = "linux")]
    { return detect_proxy_linux(); }

    #[allow(unreachable_code)]
    Ok((false, String::new(), 0))
}

#[cfg(target_os = "windows")]
fn detect_proxy_windows() -> Result<(bool, String, u32), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let settings = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .map_err(|e| format!("Failed to open registry: {}", e))?;

    let enabled: u32 = settings.get_value("ProxyEnable").unwrap_or(0);
    if enabled == 0 { return Ok((false, String::new(), 0)); }

    let server: String = settings.get_value("ProxyServer").unwrap_or_default();
    if server.is_empty() { return Ok((false, String::new(), 0)); }

    parse_proxy_string(&server)
}

#[cfg(target_os = "macos")]
fn detect_proxy_macos() -> Result<(bool, String, u32), String> {
    use std::process::Command;

    let output = Command::new("scutil").arg("--proxy").output()
        .map_err(|e| format!("Failed to run scutil: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut proxy_enabled = false;
    let mut proxy_host = String::new();
    let mut proxy_port: u32 = 0;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed == "HTTPEnable : 1" || trimmed == "HTTPSEnable : 1" {
            proxy_enabled = true;
        }
        if let Some(v) = trimmed.strip_prefix("HTTPProxy : ") {
            if !v.is_empty() { proxy_host = v.to_string(); }
        }
        if let Some(v) = trimmed.strip_prefix("HTTPPort : ") {
            proxy_port = v.parse().unwrap_or(0);
        }
    }

    Ok((proxy_enabled && !proxy_host.is_empty() && proxy_port > 0, proxy_host, proxy_port))
}

#[cfg(target_os = "linux")]
fn detect_proxy_linux() -> Result<(bool, String, u32), String> {
    // Check environment variables (standard for GNOME/KDE/Wayland proxy settings)
    for var in &["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                if let Some((h, p)) = parse_proxy_url(&val) {
                    return Ok((true, h, p));
                }
            }
        }
    }
    Ok((false, String::new(), 0))
}

/// Parse "host:port" or "http=host:port;https=host:port" format (Windows).
fn parse_proxy_string(server: &str) -> Result<(bool, String, u32), String> {
    // Try "http=host:port" first
    if let Some(http_part) = server.split(';').find(|s| s.starts_with("http=")) {
        let addr = http_part.trim_start_matches("http=");
        if let Some((h, p)) = addr.rsplit_once(':') {
            return Ok((true, h.to_string(), p.parse().unwrap_or(0)));
        }
    }
    // Fallback: "host:port"
    if let Some((h, p)) = server.rsplit_once(':') {
        return Ok((true, h.to_string(), p.parse().unwrap_or(0)));
    }
    Ok((false, String::new(), 0))
}

/// Parse "http://host:port" or "socks5://host:port" URL format.
#[allow(dead_code)]
fn parse_proxy_url(url: &str) -> Option<(String, u32)> {
    let stripped = url.trim();
    // Remove scheme: http://host:port or socks5://host:port
    let addr = if let Some(pos) = stripped.find("://") {
        &stripped[pos + 3..]
    } else { stripped };
    if let Some((h, p)) = addr.rsplit_once(':') {
        let port: u32 = p.parse().ok()?;
        if !h.is_empty() && port > 0 { return Some((h.to_string(), port)); }
    }
    None
}

#[command]
pub async fn reqwest_download(
    app: tauri::AppHandle,
    url: String,
    filename: String,
    dir: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    let download_dir = if let Some(d) = dir.filter(|s| !s.trim().is_empty()) {
        d
    } else {
        let state = app.state::<AppState>();
        let settings = lock(&state.proxy_settings)?;
        settings.download_location.clone()
    };

    let file_path = std::path::PathBuf::from(&download_dir).join(&filename);
    println!("[reqwest] Downloading {} → {}", url, file_path.display());

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36");

    if let Some(hdrs) = headers {
        let mut hdr_map = reqwest::header::HeaderMap::new();
        for (k, v) in &hdrs {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                reqwest::header::HeaderValue::from_str(v),
            ) {
                // Skip user-agent — already set above
                if name == reqwest::header::USER_AGENT { continue; }
                hdr_map.insert(name, val);
            }
        }
        builder = builder.default_headers(hdr_map);
    }

    let client = builder.build().map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client.get(&url).send().await.map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or("")));
    }

    let total = response.content_length().unwrap_or(0);
    println!("[reqwest] Response: {} — {} bytes", status, total);

    let mut file = std::fs::File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        std::io::Write::write_all(&mut file, &chunk).map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
    }

    println!("[reqwest] Downloaded {} bytes → {}", downloaded, file_path.display());

    // Persist to DB as completed
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let record = crate::state::DownloadRecord {
        gid: format!("reqwest-{}", uuid::Uuid::new_v4()),
        filename,
        url,
        size: crate::util::format_bytes(downloaded),
        status: "complete".to_string(),
        progress: 100,
        added_at: now_ms,
        path: Some(file_path.to_string_lossy().to_string()),
        connections: None,
        source: Some("reqwest".to_string()),
        ytdlp_token: None,
        proxy: None,
    };
    let _ = crate::db::upsert_download(app, record);

    Ok(downloaded.to_string())
}
