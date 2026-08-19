//! yt-dlp integration: format scan, download pipeline, install/uninstall.

use crate::aria2::Aria2Client;
use crate::state::{lock, AppState, DownloadRecord};
use crate::util::{cmd_no_window, format_bytes};
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::command;
use tauri::Emitter;
use tauri::Manager;

const YT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const YT_DLP_INSTALL_CANCEL_KEY: &str = "yt-dlp";
const EJS_INSTALL_CANCEL_KEY: &str = "ejs";

fn run_yt_emitting(
    app: &tauri::AppHandle,
    cmd: &mut Command,
    label: &str,
) -> Result<std::process::Output, std::io::Error> {
    let output = cmd.output()?;
    let _ = app.emit(
        "yt-log",
        format!("[{}] exit={}", label, output.status.code().unwrap_or(-1)),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stderr.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let _ = app.emit("yt-log", format!("[{}] {}", label, trimmed));
        }
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            let _ = app.emit("yt-log", format!("[{}] {}", label, trimmed));
        }
    }
    Ok(output)
}

fn yt_dlp_command_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

fn app_bin_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to resolve app local data dir: {}", e))?;
    Ok(base.join("binaries"))
}

fn legacy_bin_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {}", e))?;
    let bin_dir = if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
        cwd.join("binaries")
    } else {
        cwd.join("src-tauri").join("binaries")
    };
    Ok(bin_dir)
}

fn push_cookie_args(args: &mut Vec<String>, cookie_path: &Option<String>, browser: &str) {
    if let Some(cp) = cookie_path {
        args.push("--cookies".to_string());
        args.push(cp.clone());
    } else {
        args.push("--cookies-from-browser".to_string());
        args.push(browser.to_string());
    }
}

// YouTube's `web` player client is heavily throttled/restricted (limited
// format list, sign-in walls, HTTP 403/120). Prefer clients that still return
// full formats; `default` lets yt-dlp pick a working one as a fallback.
const YT_PLAYER_CLIENTS: &str = "youtube:player_client=android,tv,web_safari,default";

fn push_common_yt_flags(args: &mut Vec<String>, app: &tauri::AppHandle) -> Result<(), String> {
    args.push("--user-agent".to_string());
    args.push(YT_UA.to_string());
    args.push("--no-check-formats".to_string());
    // Impersonate Chrome so YouTube/proxies don't reset yt-dlp's requests.
    // Without this, bot-detection force-closes connections (10054/10061) and
    // the format list comes back empty → "Requested format is not available".
    args.push("--impersonate".to_string());
    args.push("chrome".to_string());
    push_yt_runtime_args(args, app)?;
    args.push("--extractor-args".to_string());
    args.push(YT_PLAYER_CLIENTS.to_string());
    Ok(())
}

fn push_yt_runtime_args(args: &mut Vec<String>, app: &tauri::AppHandle) -> Result<(), String> {
    let selected = {
        let state = app.state::<AppState>();
        let settings = lock(&state.proxy_settings)?;
        settings.js_runtime.clone()
    };

    if let Some(runtime_id) = selected {
        if runtime_id == crate::js_runtime::SYSTEM_NODE_ID {
            args.push("--js-runtimes".to_string());
            args.push("node".to_string());
            args.push("--remote-components".to_string());
            args.push("ejs:github".to_string());
            return Ok(());
        }

        if let Ok(path) = crate::js_runtime::runtime_path(app, &runtime_id) {
            if path.exists() {
                args.push("--no-js-runtimes".to_string());
                args.push("--js-runtimes".to_string());
                args.push(format!("{}:{}", runtime_id, path.to_string_lossy()));
                args.push("--remote-components".to_string());
                args.push("ejs:github".to_string());
                return Ok(());
            }
        }
    }

    // Default: prefer node if it is available on the system.
    args.push("--js-runtimes".to_string());
    args.push("node".to_string());
    args.push("--remote-components".to_string());
    args.push("ejs:github".to_string());
    Ok(())
}

fn build_proxy_url(settings: &crate::state::ProxySettings) -> String {
    if settings.proxy_type == "none" || settings.proxy_host.is_empty() {
        return String::new();
    }
    let scheme = if settings.proxy_type == "socks5" {
        "socks5"
    } else {
        "http"
    };
    let credentials = if !settings.proxy_user.is_empty() || !settings.proxy_pass.is_empty() {
        format!(
            "{}:{}@",
            urlencoding::encode(&settings.proxy_user),
            urlencoding::encode(&settings.proxy_pass)
        )
    } else {
        String::new()
    };
    format!(
        "{}://{}{}:{}",
        scheme, credentials, settings.proxy_host, settings.proxy_port
    )
}

fn push_yt_proxy_args(args: &mut Vec<String>, app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = lock(&state.proxy_settings)?.clone();
    let proxy_url = build_proxy_url(&settings);
    if !proxy_url.is_empty() {
        args.push("--proxy".to_string());
        args.push(proxy_url);
    }
    Ok(())
}

fn redact_proxy_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            redacted.push("***".to_string());
            redact_next = false;
            continue;
        }
        redacted.push(arg.clone());
        redact_next = arg == "--proxy";
    }
    redacted
}

fn extension_cookie_path(app: &tauri::AppHandle) -> Option<String> {
    let state = app.state::<AppState>();
    // Prefer the in-memory path (set when /save-cookies last ran).
    if let Some(p) = lock(&state.extension_cookies).ok().and_then(|g| g.clone()) {
        return Some(p);
    }
    // Otherwise fall back to the stable temp file written by the extension so
    // yt-dlp still gets cookies after the app restarts without falling back to
    // --cookies-from-browser (which breaks on Chrome >= v127).
    let fallback = std::env::temp_dir().join("moonhunt_cookies.txt");
    if fallback.exists() {
        // Only use the file if it actually contains cookie lines (the Netscape
        // export always begins with comment headers).
        let has_cookies = std::fs::read_to_string(&fallback)
            .map(|s| s.lines().any(|l| !l.starts_with('#') && l.contains('\t')))
            .unwrap_or(false);
        if has_cookies {
            return Some(fallback.to_string_lossy().to_string());
        }
    }
    None
}

fn download_location(app: &tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let guard = lock(&state.proxy_settings)?;
    let loc = guard.download_location.clone();
    drop(guard);
    Ok(loc)
}

fn run_yt_bin(
    app: &tauri::AppHandle,
    bin_path: &Path,
    args: &[String],
    label: &str,
) -> Result<std::process::Output, std::io::Error> {
    let mut cmd = if bin_path.exists() {
        cmd_no_window(&bin_path.to_string_lossy())
    } else {
        cmd_no_window(yt_dlp_command_name())
    };
    run_yt_emitting(app, cmd.args(args), label)
}

#[command]
pub fn cancel_yt_scan(app: tauri::AppHandle, scan_id: String) -> Result<(), String> {
    if scan_id.trim().is_empty() {
        return Err("A scan id is required to cancel a yt-dlp scan".to_string());
    }

    let state = app.state::<AppState>();
    {
        let mut cancelled = lock(&state.cancelled_yt_scans)?;
        cancelled.insert(scan_id.clone());
    }
    let mut guard = lock(&state.active_yt_scan)?;
    if let Some(mut child) = guard.remove(&scan_id) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = app.emit("yt-log", format!("[scan] cancelled {}", scan_id));
    }
    Ok(())
}

fn choose_media_urls(meta: &Value) -> Vec<String> {
    let mut chosen: Option<String> = None;
    if let Some(u) = meta.get("url").and_then(|s| s.as_str()) {
        chosen = Some(u.to_string());
    }
    if chosen.is_none() {
        if let Some(rf) = meta.get("requested_formats").and_then(|v| v.as_array()) {
            for f in rf {
                let has_audio = f
                    .get("acodec")
                    .and_then(|v| v.as_str())
                    .map(|s| s != "none")
                    .unwrap_or(false);
                let has_video = f
                    .get("vcodec")
                    .and_then(|v| v.as_str())
                    .map(|s| s != "none")
                    .unwrap_or(false);
                if has_audio && has_video {
                    if let Some(u) = f.get("url").and_then(|s| s.as_str()) {
                        chosen = Some(u.to_string());
                        break;
                    }
                }
            }
            if chosen.is_none() {
                if let Some(f0) = rf.first() {
                    if let Some(u) = f0.get("url").and_then(|s| s.as_str()) {
                        chosen = Some(u.to_string());
                    }
                }
            }
        }
    }
    if chosen.is_none() {
        if let Some(frames) = meta.get("formats").and_then(|v| v.as_array()) {
            for f in frames.iter().rev() {
                let has_audio = f
                    .get("acodec")
                    .and_then(|v| v.as_str())
                    .map(|s| s != "none")
                    .unwrap_or(false);
                let has_video = f
                    .get("vcodec")
                    .and_then(|v| v.as_str())
                    .map(|s| s != "none")
                    .unwrap_or(false);
                if has_audio && has_video {
                    if let Some(u) = f.get("url").and_then(|s| s.as_str()) {
                        chosen = Some(u.to_string());
                        break;
                    }
                }
            }
        }
    }
    if let Some(u) = chosen {
        return vec![u];
    }
    let mut all = Vec::new();
    if let Some(rf) = meta.get("requested_formats").and_then(|v| v.as_array()) {
        for f in rf {
            if let Some(u) = f.get("url").and_then(|s| s.as_str()) {
                all.push(u.to_string());
            }
        }
    }
    if all.is_empty() {
        if let Some(u) = meta.get("url").and_then(|s| s.as_str()) {
            all.push(u.to_string());
        }
    }
    all
}

fn collect_http_headers(meta: &Value, ytdlp_token: &str, page_url: &str) -> Vec<String> {
    let mut hdrs = vec![format!("X-Moonhunt-YTDLP:{}", ytdlp_token)];
    let mut push_obj = |obj: &serde_json::Map<String, Value>| {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                hdrs.push(format!("{}:{}", k, s));
            }
        }
    };
    if let Some(hobj) = meta.get("http_headers").and_then(|v| v.as_object()) {
        push_obj(hobj);
    }
    if let Some(rf) = meta.get("requested_formats").and_then(|v| v.as_array()) {
        for f in rf {
            if let Some(hf) = f.get("http_headers").and_then(|v| v.as_object()) {
                push_obj(hf);
            }
        }
    }
    if let Some(frames) = meta.get("formats").and_then(|v| v.as_array()) {
        for f in frames {
            if let Some(hf) = f.get("http_headers").and_then(|v| v.as_object()) {
                push_obj(hf);
            }
        }
    }
    if !hdrs
        .iter()
        .any(|h| h.to_lowercase().starts_with("user-agent:"))
    {
        hdrs.push(format!("User-Agent:{}", YT_UA));
    }
    if !page_url.trim().is_empty() {
        hdrs.push(format!("Referer:{}", page_url));
    }
    hdrs
}

fn persist_complete(
    app: &tauri::AppHandle,
    gid: String,
    filename: &str,
    url: &str,
    download_dir: &str,
    ytdlp_token: &str,
    source_event: &str,
) -> Result<String, String> {
    // If the user cancelled/paused/removed this download, do not resurrect it.
    if crate::extension::cancelled_gids().contains(&gid) {
        println!("[yt-dlp] Skipping persist_complete for cancelled gid {}", gid);
        return Ok(gid);
    }
    let file_path = Path::new(download_dir).join(filename);
    let size = std::fs::metadata(&file_path)
        .map(|m| format_bytes(m.len()))
        .unwrap_or_else(|_| "0 B".to_string());
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let record = DownloadRecord {
        gid: gid.clone(),
        filename: filename.to_string(),
        url: url.to_string(),
        size,
        status: "complete".to_string(),
        progress: 100,
        added_at: now_ms,
        path: Some(file_path.to_string_lossy().to_string()),
        connections: None,
        source: Some("yt-dlp".to_string()),
        ytdlp_token: Some(ytdlp_token.to_string()),
        proxy: None,
    };
    crate::db::upsert_download(app.clone(), record)?;
    let _ = app.emit(
        "yt-dl-complete",
        json!({"gid": gid, "filename": filename, "source": source_event}),
    );
    Ok(gid)
}

fn persist_error(
    app: &tauri::AppHandle,
    gid: String,
    filename: &str,
    url: &str,
    download_dir: &str,
    message: &str,
) {
    // A cancelled/paused/removed download must not be resurrected as an error.
    if crate::extension::cancelled_gids().contains(&gid) {
        println!("[yt-dlp] Skipping persist_error for cancelled gid {}", gid);
        return;
    }
    let file_path = Path::new(download_dir).join(filename);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let record = DownloadRecord {
        gid: gid.clone(),
        filename: filename.to_string(),
        url: url.to_string(),
        size: "0".to_string(),
        status: "error".to_string(),
        progress: 100,
        added_at: now_ms,
        path: Some(file_path.to_string_lossy().to_string()),
        connections: Some(1),
        source: Some("yt-dlp".to_string()),
        ytdlp_token: None,
        proxy: None,
    };
    if let Err(e) = crate::db::upsert_download(app.clone(), record) {
        println!("[!] Failed to persist yt-dlp error record: {}", e);
    }
    let _ = app.emit(
        "yt-dl-error",
        json!({"gid": gid, "filename": filename, "error": message}),
    );
}

#[derive(Clone, Copy)]
struct YtProgress {
    total: u64,
    completed: u64,
    speed: u64,
}

fn parse_yt_size(raw: &str) -> Option<u64> {
    let raw = raw.trim().replace(',', "");
    if raw.is_empty() {
        return None;
    }
    let split = raw.find(|c: char| c.is_ascii_alphabetic())?;
    let num: f64 = raw[..split].trim().parse().ok()?;
    let unit = raw[split..].trim().to_ascii_lowercase();
    let mult: u64 = match unit.as_str() {
        "b" => 1,
        "kib" | "kb" => 1024,
        "mib" | "mb" => 1024 * 1024,
        "gib" | "gb" => 1024 * 1024 * 1024,
        "tib" | "tb" => 1024_u64.pow(4),
        _ => return None,
    };
    Some((num * mult as f64) as u64)
}

fn parse_yt_progress(line: &str) -> Option<YtProgress> {
    let line = line.trim();
    if !line.contains("[download]") {
        return None;
    }
    let trimmed = &line[line.find("[download]")?..];

    let mut total: Option<u64> = None;
    let mut speed: Option<u64> = None;
    let mut percent: Option<f64> = None;

    let mut parts = trimmed.split_whitespace();
    while let Some(part) = parts.next() {
        if part.ends_with('%') {
            percent = part.trim_end_matches('%').parse::<f64>().ok();
            continue;
        }
        if part == "of" {
            let mut raw = parts.next().unwrap_or("");
            if raw == "~" {
                raw = parts.next().unwrap_or("");
            }
            total = parse_yt_size(raw);
            continue;
        }
        if part == "at" {
            if let Some(raw) = parts.next() {
                speed = parse_yt_size(raw.trim_end_matches("/s"));
            }
        }
    }

    if percent.is_none() && speed.is_none() {
        return None;
    }

    let completed = percent
        .zip(total)
        .map(|(p, t)| ((p / 100.0) * t as f64) as u64)
        .unwrap_or(0);
    Some(YtProgress {
        total: total.unwrap_or(0),
        completed,
        speed: speed.unwrap_or(0),
    })
}

fn update_yt_progress(
    app: &tauri::AppHandle,
    gid: &str,
    filename: &str,
    url: &str,
    download_dir: &str,
    ytdlp_token: &str,
    added_at: i64,
    progress: &YtProgress,
    persist: bool,
) {
    let file_path = Path::new(download_dir).join(filename);
    let percent = if progress.total > 0 {
        ((progress.completed as f64 / progress.total as f64) * 100.0).round() as i32
    } else {
        0
    };
    let size_display = if progress.total > 0 {
        format!(
            "{} / {}",
            format_bytes(progress.completed),
            format_bytes(progress.total)
        )
    } else {
        "0 B / 0 B".to_string()
    };

    let record = DownloadRecord {
        gid: gid.to_string(),
        filename: filename.to_string(),
        url: url.to_string(),
        size: size_display,
        status: "active".to_string(),
        progress: percent,
        added_at,
        path: Some(file_path.to_string_lossy().to_string()),
        connections: Some(1),
        source: Some("yt-dlp".to_string()),
        ytdlp_token: Some(ytdlp_token.to_string()),
        proxy: None,
    };
    if persist {
        if let Err(e) = crate::db::upsert_download(app.clone(), record) {
            println!("[!] Failed to persist yt-dlp progress (non-fatal): {}", e);
        }
    }

    let header = format!("X-Moonhunt-YTDLP:{}", ytdlp_token);
    let item = json!({
        "gid": gid,
        "status": "active",
        "totalLength": progress.total.to_string(),
        "completedLength": progress.completed.to_string(),
        "downloadSpeed": progress.speed.to_string(),
        "uploadSpeed": "0",
        "connections": "1",
        "split": "1",
        "requestHeaders": [header.clone()],
        "files": [{
            "path": file_path.to_string_lossy().to_string(),
            "length": progress.total.to_string(),
            "completedLength": progress.completed.to_string(),
            "uris": [{
                "uri": url,
                "status": "used",
                "downloadSpeed": progress.speed.to_string(),
                "requestHeaders": [header]
            }],
            "requestHeaders": [header]
        }]
    });
    let _ = app.emit("downloads-changed", vec![item]);
}

/// Cancels a running direct yt-dlp download by PID and removes the synthetic
/// DB row. Used by pause/remove from the UI.
#[cfg(target_os = "windows")]
fn kill_pid(pid: u32) {
    use std::os::windows::process::CommandExt;
    // taskkill is available on all Windows versions and works without elevation
    // for child processes we spawned.
    if let Ok(mut cmd) = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F", "/T"])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn()
    {
        let _ = cmd.wait();
    }
}

#[cfg(not(target_os = "windows"))]
fn kill_pid(pid: u32) {
    // libc::kill(pid, SIGKILL) is the standard cross-nix approach; fall back to
    // `kill` if libc isn't linked. Using the CLI is simplest and dependency-free.
    if let Ok(mut cmd) = Command::new("kill").args(["-9", &pid.to_string()]).spawn() {
        let _ = cmd.wait();
    }
}

/// Cancel a running direct yt-dlp download by killing its process and removing
/// the synthetic DB row. Used by pause/remove from the UI.
#[command]
pub fn cancel_yt_download(app: tauri::AppHandle, gid: String) -> Result<(), String> {
    if !gid.starts_with("yt-dlp-direct-") {
        return Err(format!("Not a direct yt-dlp download: {}", gid));
    }
    let state = app.state::<AppState>();
    let pid = {
        let registry = state.active_yt_downloads.clone();
        let mut guard = registry.lock().map_err(|e| e.to_string())?;
        guard.remove(&gid).unwrap_or(0)
    };
    if pid != 0 {
        kill_pid(pid);
        let _ = app.emit("yt-log", format!("[direct] cancelling {} (pid {})", gid, pid));
    }
    // Mark cancelled so the poller never re-shows the row as complete.
    crate::extension::mark_cancelled(&gid);
    let _ = crate::db::remove_download_from_db(app.clone(), gid.clone());
    let _ = app.emit("downloads-changed", json!([]));
    Ok(())
}

/// Re-run a cancelled/error direct media download from its stored DB record.
/// This is "resume" for yt-dlp-direct rows (which can't be resumed mid-file).
#[command]
pub async fn resume_yt_download(app: tauri::AppHandle, gid: String) -> Result<String, String> {
    if !gid.starts_with("yt-dlp-direct-") {
        return Err(format!("Not a direct yt-dlp download: {}", gid));
    }
    let record = crate::db::get_download_by_gid(app.clone(), gid.clone())
        .ok()
        .flatten();
    let url = record
        .as_ref()
        .and_then(|r| r.get("url").and_then(|u| u.as_str()))
        .unwrap_or("")
        .to_string();
    if url.trim().is_empty() || !url.starts_with("http") {
        return Err("No source URL to resume".to_string());
    }
    let dir = record.as_ref().and_then(|r| {
        r.get("path")
            .and_then(|p| p.as_str())
            .and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_string_lossy().into_owned()))
    });
    let gid = add_yt_download(
        app.clone(),
        url.clone(),
        "mp4".to_string(),
        Some("best".to_string()),
        dir,
        None,
        None,
    )
    .await?;
    Ok(format!("Media download restarted ({})", gid))
}

fn run_direct_yt_download(
    app: tauri::AppHandle,
    bin_path: PathBuf,
    args: Vec<String>,
    gid: String,
    filename: String,
    url: String,
    download_dir: String,
    ytdlp_token: String,
    added_at: i64,
) {
    let state = app.state::<AppState>();

    // YouTube intermittently resets connections (exit 120 / "connection
    // reset") or briefly drops specific formats (SABR/DRM experiments), which
    // surfaces as "Requested format is not available". Retry the same args if
    // no progress was made, but if the specific-format download keeps failing,
    // fall back to the generic bestvideo+bestaudio merge so the download still
    // completes. The fallback args drop any user-picked -f <id> and any
    // --remux-video (merge produces an mp4 already).
    // A "best effort" arg set for the retry: keep everything except the
    // `-f <id>`/`--merge-output-format`/`--remux-video` format flags, then
    // append a generic bestvideo+bestaudio merge. The filter keeps every
    // argument whose PREVIOUS token is not a format/merge flag.
    let mut fallback_args: Vec<String> = Vec::new();
    let mut had_extract_audio = false;
    {
        // Only value-taking flags consume their following token. `--extract-audio`
        // is a boolean flag — skipping the next token would swallow the following
        // `--audio-format` and leave its `mp3` value orphaned as a bogus URL arg.
        let mut skip_next = false;
        for a in &args {
            if skip_next {
                skip_next = false;
                continue;
            }
            match a.as_str() {
                "-f" | "--merge-output-format" | "--remux-video" | "--audio-format"
                | "--audio-quality" => {
                    skip_next = true;
                }
                "--extract-audio" => {
                    had_extract_audio = true;
                }
                _ => fallback_args.push(a.clone()),
            }
        }
    }
    if had_extract_audio {
        // Keep the user's audio-only request: grab the best audio stream and
        // re-extract to mp3 instead of silently merging a video track.
        fallback_args.push("-f".to_string());
        fallback_args.push("bestaudio/best".to_string());
        fallback_args.push("--extract-audio".to_string());
        fallback_args.push("--audio-format".to_string());
        fallback_args.push("mp3".to_string());
        fallback_args.push("--audio-quality".to_string());
        fallback_args.push("0".to_string());
    } else {
        fallback_args.push("-f".to_string());
        fallback_args.push("bestvideo+bestaudio".to_string());
        fallback_args.push("--merge-output-format".to_string());
        fallback_args.push("mp4".to_string());
    }

    // YouTube intermittently resets connections (exit 120 / "connection
    // reset"). A single retry that made no progress usually succeeds.
    let max_attempts = 2;
    let mut last_attempt_used_fallback = false;
    for attempt in 1..=max_attempts {
        // The user may have cancelled while a previous attempt was dying or
        // during the retry sleep; never relaunch a cancelled download.
        if crate::extension::cancelled_gids().contains(&gid) {
            return;
        }
        let attempt_args: &[String] = if attempt > 1 && !last_attempt_used_fallback {
            // The specific format may have vanished (SABR/DRM transient); fall
            // back to a generic bestvideo+bestaudio merge on retry.
            &fallback_args
        } else {
            &args
        };
        let mut cmd = if bin_path.exists() {
            cmd_no_window(&bin_path.to_string_lossy())
        } else {
            cmd_no_window(yt_dlp_command_name())
        };
        if attempt > 1 && !last_attempt_used_fallback {
            last_attempt_used_fallback = true;
        }
        cmd.args(attempt_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("Failed to execute yt-dlp direct: {}", e);
                println!("[✗] {}", msg);
                persist_error(&app, gid, &filename, &url, &download_dir, &msg);
                return;
            }
        };

        // Register the running child's PID so pause/remove can terminate it by gid.
        {
            let registry = state.active_yt_downloads.clone();
            let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            guard.insert(gid.clone(), child.id());
        }

        // yt-dlp writes its `[download]` progress lines to stdout, while
        // warnings/errors go to stderr. Drain stderr on a separate thread so
        // a chatty stderr stream can never block the progress reader. Keep the
        // last lines in a shared buffer so the failure message can show them.
        let stderr_buf: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        if let Some(stderr) = child.stderr.take() {
            let log_app = app.clone();
            let buf = stderr_buf.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let _ = log_app.emit("yt-log", format!("[direct] {}", trimmed));
                        if let Ok(mut v) = buf.lock() {
                            v.push(trimmed.to_string());
                            if v.len() > 20 {
                                v.remove(0);
                            }
                        }
                    }
                }
            });
        }

        let mut saw_progress = false;
        let mut last_progress_emit = Instant::now() - Duration::from_millis(500);
        let mut last_progress_persist = Instant::now() - Duration::from_secs(2);
        let mut last_completed: u64 = 0;
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                let trimmed = line.trim();
                if let Some(mut progress) = parse_yt_progress(&trimmed) {
                    saw_progress = true;
                    if progress.total > 0 && progress.completed > progress.total {
                        progress.completed = progress.total;
                    }
                    // Progress lines from yt-dlp can briefly dip while it retries
                    // fragments or switches between streams. Keep the UI monotonic
                    // so the progress bar never visibly moves backwards.
                    if progress.completed < last_completed {
                        progress.completed = last_completed;
                    } else {
                        last_completed = progress.completed;
                    }
                    let finished = progress.total > 0 && progress.completed >= progress.total;
                    if last_progress_emit.elapsed() >= Duration::from_millis(500) || finished {
                        let persist =
                            last_progress_persist.elapsed() >= Duration::from_secs(2) || finished;
                        update_yt_progress(
                            &app,
                            &gid,
                            &filename,
                            &url,
                            &download_dir,
                            &ytdlp_token,
                            added_at,
                            &progress,
                            persist,
                        );
                        last_progress_emit = Instant::now();
                        if persist {
                            last_progress_persist = Instant::now();
                        }
                    }
                } else if !trimmed.is_empty() {
                    // Progress lines already update the download row; don't also
                    // flood the log panel and WebView DOM with them.
                    let _ = app.emit("yt-log", format!("[direct] {}", trimmed));
                }
            }
        }

        let status = child.wait();
        // Remove the tracked PID now that this attempt's child has exited.
        {
            let registry = state.active_yt_downloads.clone();
            let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
            guard.remove(&gid);
        }

        match status {
            Ok(exit) if exit.success() => {
                if let Err(e) = persist_complete(
                    &app,
                    gid.clone(),
                    &filename,
                    &url,
                    &download_dir,
                    &ytdlp_token,
                    "yt-dlp-direct",
                ) {
                    println!("[!] Failed to persist completed yt-dlp download: {}", e);
                    persist_error(&app, gid.clone(), &filename, &url, &download_dir, &e);
                }
                return;
            }
            Ok(exit) => {
                let tail = stderr_buf
                    .lock()
                    .map(|g| g.join("\n"))
                    .unwrap_or_default();
                let msg = format!(
                    "yt-dlp direct download failed with exit code {}{}",
                    exit.code().unwrap_or(-1),
                    if tail.is_empty() {
                        String::new()
                    } else {
                        format!("\n{}", tail)
                    }
                );
                // Retry only if the attempt made no progress (transient reset
                // before any bytes) and we haven't exhausted attempts.
                if !saw_progress && attempt < max_attempts {
                    println!("[~] Retrying direct yt-dlp download (attempt {}/{})", attempt, max_attempts);
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
                println!("[✗] {}", msg);
                persist_error(&app, gid.clone(), &filename, &url, &download_dir, &msg);
                return;
            }
            Err(e) => {
                let msg = format!("Failed to wait for yt-dlp direct: {}", e);
                println!("[✗] {}", msg);
                persist_error(&app, gid.clone(), &filename, &url, &download_dir, &msg);
                return;
            }
        }
    }
}

#[command]
pub async fn scan_yt_formats(
    app: tauri::AppHandle,
    url: String,
    browser: Option<String>,
    scan_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let bin_path = yt_dlp_bin_path(&app)?;
    let selected_browser = browser.unwrap_or_else(|| "firefox".to_string());
    let cookie_path = extension_cookie_path(&app);

    let mut meta_args: Vec<String> = Vec::new();
    meta_args.push("--no-config".to_string());
    meta_args.push("-j".to_string());
    meta_args.push("--no-warnings".to_string());
    meta_args.push("--no-playlist".to_string());
    meta_args.push("--socket-timeout".to_string());
    meta_args.push("30".to_string());
    push_cookie_args(&mut meta_args, &cookie_path, &selected_browser);
    push_common_yt_flags(&mut meta_args, &app)?;
    push_yt_proxy_args(&mut meta_args, &app)?;
    meta_args.push(url.clone());

    let scan_key = scan_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut cmd = if bin_path.exists() {
        cmd_no_window(&bin_path.to_string_lossy())
    } else {
        cmd_no_window(yt_dlp_command_name())
    };
    cmd.args(&meta_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let state = app.state::<AppState>();
    {
        let mut guard = lock(&state.active_yt_scan)?;
        guard.insert(scan_key.clone(), child);
    }
    {
        let cancelled = lock(&state.cancelled_yt_scans)?;
        if cancelled.contains(&scan_key) {
            drop(cancelled);
            let mut guard = lock(&state.active_yt_scan)?;
            if let Some(mut child) = guard.remove(&scan_key) {
                let _ = child.kill();
                let _ = child.wait();
            }
            return Err("Scan cancelled".to_string());
        }
    }

    let stdout_lines = Arc::new(Mutex::new(Vec::new()));
    let stderr_lines = Arc::new(Mutex::new(Vec::new()));

    let stdout_writer = stdout_lines.clone();
    let stdout_thread = stdout_pipe.map(|pipe| {
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = stdout_writer.lock().map(|mut v| v.push(trimmed));
                }
            }
        })
    });

    let stderr_writer = stderr_lines.clone();
    let stderr_thread = stderr_pipe.map(|pipe| {
        let log_app = app.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let _ = log_app.emit("yt-log", format!("[scan] {}", trimmed));
                    let _ = stderr_writer
                        .lock()
                        .map(|mut v| v.push(trimmed.to_string()));
                }
            }
        })
    });

    let mut exit_status: Option<std::process::ExitStatus> = None;
    for _ in 0..2400 {
        let (status, missing) = {
            let mut guard = lock(&state.active_yt_scan)?;
            match guard.get_mut(&scan_key) {
                Some(process) => match process.try_wait() {
                    Ok(status) => (status, false),
                    Err(e) => {
                        drop(guard);
                        let mut guard = lock(&state.active_yt_scan)?;
                        if let Some(mut child) = guard.remove(&scan_key) {
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                        let _ = lock(&state.cancelled_yt_scans).map(|mut s| s.remove(&scan_key));
                        if let Some(thread) = stdout_thread {
                            let _ = thread.join();
                        }
                        if let Some(thread) = stderr_thread {
                            let _ = thread.join();
                        }
                        return Err(format!("Failed to poll yt-dlp: {}", e));
                    }
                },
                None => (None, true),
            }
        };
        if missing {
            let _ = lock(&state.cancelled_yt_scans).map(|mut s| s.remove(&scan_key));
            if let Some(thread) = stdout_thread {
                let _ = thread.join();
            }
            if let Some(thread) = stderr_thread {
                let _ = thread.join();
            }
            return Err("Scan cancelled".to_string());
        }
        if let Some(status) = status {
            exit_status = Some(status);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let status = match exit_status {
        Some(status) => {
            let mut guard = lock(&state.active_yt_scan)?;
            guard.remove(&scan_key);
            status
        }
        None => {
            let mut guard = lock(&state.active_yt_scan)?;
            if let Some(mut child) = guard.remove(&scan_key) {
                let _ = child.kill();
                let _ = child.wait();
            }
            let _ = lock(&state.cancelled_yt_scans).map(|mut s| s.remove(&scan_key));
            if let Some(thread) = stdout_thread {
                let _ = thread.join();
            }
            if let Some(thread) = stderr_thread {
                let _ = thread.join();
            }
            return Err("yt-dlp scan timed out after 4 minutes".to_string());
        }
    };

    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }
    let _ = lock(&state.cancelled_yt_scans).map(|mut s| s.remove(&scan_key));

    if !status.success() {
        let stderr = stderr_lines
            .lock()
            .map(|guard| guard.join("\n"))
            .unwrap_or_default();
        return Err(format!("yt-dlp failed: {}", stderr));
    }

    let stdout = stdout_lines
        .lock()
        .map(|guard| guard.join("\n"))
        .unwrap_or_default();
    let start = stdout
        .find('{')
        .ok_or("yt-dlp returned no JSON metadata".to_string())?;
    let end = stdout
        .rfind('}')
        .ok_or("yt-dlp returned malformed JSON".to_string())?;
    let json_str = &stdout[start..=end];
    let meta: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))?;

    let mut formats: Vec<serde_json::Value> = Vec::new();

    if let Some(frames) = meta.get("formats").and_then(|v| v.as_array()) {
        for f in frames {
            let format_id = f
                .get("format_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ext = f
                .get("ext")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let resolution = f
                .get("resolution")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let fps = f.get("fps").and_then(|v| v.as_f64()).map(|n| n as u64);
            let vcodec = f
                .get("vcodec")
                .and_then(|v| v.as_str())
                .unwrap_or("none")
                .to_string();
            let acodec = f
                .get("acodec")
                .and_then(|v| v.as_str())
                .unwrap_or("none")
                .to_string();
            let filesize = f.get("filesize").and_then(|v| v.as_f64()).map(|n| n as u64);
            let tbr = f.get("tbr").and_then(|v| v.as_f64());
            let format_note = f
                .get("format_note")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let has_video = vcodec != "none";
            let has_audio = acodec != "none";

            // YouTube storyboards are thumbnail strips for player previews,
            // not downloadable media. Skip those plus any other pseudo
            // entries that have neither audio nor video codecs.
            let is_downloadable = has_video || has_audio;
            let looks_like_metadata = format_id.to_ascii_lowercase().contains("storyboard")
                || format_note.to_ascii_lowercase().contains("storyboard");
            if !is_downloadable || looks_like_metadata {
                continue;
            }

            // Build a human-readable label
            let mut label = String::new();
            if has_video && has_audio {
                label.push_str(&resolution);
                if let Some(fps_val) = fps {
                    label.push_str(&format!(" {}fps", fps_val));
                }
                label.push_str(" (video+audio)");
            } else if has_video {
                label.push_str(&resolution);
                if let Some(fps_val) = fps {
                    label.push_str(&format!(" {}fps", fps_val));
                }
                label.push_str(" (video only)");
            } else if has_audio {
                label.push_str(&format!("Audio {}", ext));
                if let Some(tbr_val) = tbr {
                    label.push_str(&format!(" {}kbps", tbr_val as u64));
                }
            }
            if !format_note.is_empty() && !label.contains(&format_note) {
                label.push_str(&format!(" [{}]", format_note));
            }
            if label.is_empty() {
                label = format!("{} ({})", format_id, ext);
            }

            // A video-only format can't be played as a standalone file unless a
            // merger combines it with audio. Mark it so the UI can tag it and
            // hide it when no merger is active.
            let requires_merger = has_video && !has_audio;

            formats.push(json!({
                "format_id": format_id,
                "label": label,
                "ext": ext,
                "resolution": resolution,
                "fps": fps,
                "vcodec": vcodec,
                "acodec": acodec,
                "filesize": filesize,
                "tbr": tbr,
                "format_note": format_note,
                "has_video": has_video,
                "has_audio": has_audio,
                "requires_merger": requires_merger,
            }));
        }
    }

    // Sort: combined video+audio first, then by resolution height desc
    formats.sort_by(|a, b| {
        let a_combined = a
            .get("has_video")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            && a.get("has_audio")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let b_combined = b
            .get("has_video")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            && b.get("has_audio")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        b_combined.cmp(&a_combined).then_with(|| {
            let a_h: u64 = a
                .get("resolution")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_end_matches('p').parse().ok())
                .unwrap_or(0);
            let b_h: u64 = b
                .get("resolution")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_end_matches('p').parse().ok())
                .unwrap_or(0);
            b_h.cmp(&a_h)
        })
    });

    // yt-dlp never lists mp3 as a native format because it is produced by
    // --extract-audio. Expose it as a first-class option so every UI that
    // calls scan_yt_formats (including already-built clients) can offer it.
    let already_has_mp3 = formats.iter().any(|f| {
        f.get("ext").and_then(|v| v.as_str()).unwrap_or("") == "mp3"
            && f.get("has_audio")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
    });
    if !already_has_mp3 {
        formats.insert(
            0,
            json!({
                "format_id": "bestaudio",
                "ext": "mp3",
                "resolution": "",
                "fps": null,
                "vcodec": "none",
                "acodec": "mp3",
                "filesize": null,
                "tbr": null,
                "format_note": "Audio converted to MP3",
                "has_video": false,
                "has_audio": true,
                "requires_merger": false,
                "label": "MP3 (best audio quality)",
            }),
        );
    }

    // YouTube only sporadically lists native combined (video+audio) formats
    // anymore — most videos expose just a low-res 360p combined stream plus
    // separate video-only/audio-only tracks. Always offer a best-quality
    // combined option so the scan modal reliably has a video+audio choice.
    // `add_yt_download` translates this synthetic id into a bestvideo+bestaudio
    // merge (falling back to a pre-merged stream when no merger is installed).
    formats.insert(
        0,
        json!({
            "format_id": "best",
            "ext": "mp4",
            "resolution": "",
            "fps": null,
            "vcodec": "avc1.64002a",
            "acodec": "mp4a.40.2",
            "filesize": null,
            "tbr": null,
            "format_note": "",
            "has_video": true,
            "has_audio": true,
            "requires_merger": false,
            "label": "Best quality (video+audio)",
        }),
    );

    Ok(json!({
        "formats": formats,
        "mergerAvailable": resolved_merger(&app).is_some(),
    }))
}

#[command]
pub async fn add_yt_download(
    app: tauri::AppHandle,
    url: String,
    format: String,
    quality: Option<String>,
    path: Option<String>,
    browser: Option<String>,
    format_id: Option<String>,
) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    // Hold onto the user-supplied quality so we can pick the right code
    // path (direct yt-dlp vs aria2 handoff) BEFORE later code consumes
    // the original.
    let orig_quality = quality.clone();

    let _ = app.emit("yt-log", format!("[start] Downloading: {}", url));

    let download_dir = if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        p
    } else {
        download_location(&app)?
    };

    // Prepare yt-dlp args
    let mut args: Vec<String> = Vec::new();

    // Output template (use forward slashes for yt-dlp)
    let out_template = format!("{}/%(title)s.%(ext)s", download_dir.replace('\\', "/"));
    args.push("-o".to_string());
    args.push(out_template);

    let _ = app.emit(
        "yt-log",
        format!(
            "[setup] Format: {}, Quality: {}, Browser: {}",
            format,
            quality.as_deref().unwrap_or("best"),
            browser.as_deref().unwrap_or("firefox")
        ),
    );

    println!("[~] Launching yt-dlp with args: {:?}", redact_proxy_args(&args));

    // A merger (MP4Box by default, or optional ffmpeg) lets yt-dlp combine
    // separate video+audio streams, so high-res formats (which are video-only)
    // become usable with audio.
    let has_merger = resolved_merger(&app).is_some();

    if let Some(ref fid) = format_id {
        // User selected a specific format from the scan results.
        if format.to_lowercase() == "mp3" {
            args.push("-f".to_string());
            args.push(fid.clone());
            args.push("--extract-audio".to_string());
            args.push("--audio-format".to_string());
            args.push("mp3".to_string());
            args.push("--audio-quality".to_string());
            args.push("0".to_string());
        } else if fid == "best" {
            // The synthetic "Best quality (video+audio)" scan option: merge the
            // best video and audio streams (or fall back to a pre-merged stream
            // when no merger is installed).
            if has_merger {
                args.push("-f".to_string());
                args.push("bestvideo+bestaudio".to_string());
                args.push("--merge-output-format".to_string());
                args.push("mp4".to_string());
                args.push("--remux-video".to_string());
                args.push("mp4".to_string());
            } else {
                args.push("-f".to_string());
                args.push("best[ext=mp4]/best".to_string());
                args.push("--remux-video".to_string());
                args.push("mp4".to_string());
            }
        } else if has_merger {
            // The selected format may be video-only; attach the best audio and
            // merge so the file always has sound. yt-dlp picks bestaudio that
            // matches the video's container; remux to mp4 afterwards.
            args.push("-f".to_string());
            args.push(format!("{}+bestaudio", fid));
            args.push("--merge-output-format".to_string());
            args.push("mp4".to_string());
            args.push("--remux-video".to_string());
            args.push("mp4".to_string());
        } else {
            // No merger — a video-only format would yield a silent file, so
            // fall back to the best combined format (has audio).
            args.push("-f".to_string());
            args.push("best[ext=mp4]/best".to_string());
            args.push("--remux-video".to_string());
            args.push("mp4".to_string());
        }
    } else if format.to_lowercase() == "mp3" {
        args.push("--extract-audio".to_string());
        args.push("--audio-format".to_string());
        args.push("mp3".to_string());
        args.push("--audio-quality".to_string());
        args.push("0".to_string());
    } else {
        // mp4 branch
        let q = quality.unwrap_or_else(|| "best".to_string());
        if q == "best" {
            if has_merger {
                // Best quality shouldn't cap at the low-res combined format.
                args.push("-f".to_string());
                args.push("bestvideo+bestaudio".to_string());
                args.push("--merge-output-format".to_string());
                args.push("mp4".to_string());
                args.push("--remux-video".to_string());
                args.push("mp4".to_string());
            } else {
                args.push("-f".to_string());
                args.push("best[ext=mp4]/best".to_string());
                args.push("--remux-video".to_string());
                args.push("mp4".to_string());
            }
        } else if let Ok(qn) = q.parse::<u32>() {
            args.push("-f".to_string());
            if has_merger {
                args.push(format!("bestvideo[height<={}]+bestaudio", qn));
                args.push("--merge-output-format".to_string());
                args.push("mp4".to_string());
            } else {
                args.push(format!("best[height<={}]", qn));
            }
            args.push("--remux-video".to_string());
            args.push("mp4".to_string());
        } else {
            args.push("-f".to_string());
            args.push("best".to_string());
            args.push("--remux-video".to_string());
            args.push("mp4".to_string());
        }
    }

    // Point yt-dlp at ffmpeg's dir so merges (video+audio) always work, even
    // when ffmpeg is not on PATH.
    if let Some(ff) = resolved_merger(&app) {
        if let Some(dir) = ff.parent() {
            args.push("--ffmpeg-location".to_string());
            args.push(dir.to_string_lossy().into_owned());
        }
    }

    // Authentication: Use extension cookies if available, otherwise browser cookies
    let selected_browser = browser.unwrap_or_else(|| "firefox".to_string());
    let cookie_path = extension_cookie_path(&app);

    if let Some(ref cp) = cookie_path {
        args.push("--cookies".to_string());
        args.push(cp.clone());
        println!("[🍪] add_yt_download using extension cookies: {}", cp);
    } else {
        args.push("--cookies-from-browser".to_string());
        args.push(selected_browser.clone());
        println!("[*] Using browser for cookies: {}", selected_browser);
    }

    // Request headers to avoid YouTube throttling + impersonate Chrome so
    // bot-detection doesn't reset the connection (10054/10061).
    args.push("--user-agent".to_string());
    args.push("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());
    args.push("--impersonate".to_string());
    args.push("chrome".to_string());

    // Retry logic for 429 errors (too many requests)
    args.push("--socket-timeout".to_string());
    args.push("30".to_string());
    args.push("--retries".to_string());
    args.push("5".to_string());
    args.push("--retry-sleep".to_string());
    args.push("http:5".to_string()); // 5 second delay between retries

    push_yt_proxy_args(&mut args, &app)?;

    // Format selection is already handled above in the mp4 branch (lines 706-720)
    // Do not add another -f flag here, as it would override the quality selection

    args.push("--extractor-args".to_string());
    args.push(YT_PLAYER_CLIENTS.to_string());

    push_yt_runtime_args(&mut args, &app)?;

    // Instead of launching yt-dlp to do the download via an external aria2 process (which won't be visible
    // to the app's aria2 RPC), ask yt-dlp for the direct media URL(s) and add them to the app's aria2 RPC.
    // This keeps all transfers in the same aria2 instance the UI monitors.

    // Generate a short unique token to tag aria2 requests started by yt-dlp.
    // ponytail: timestamp token avoids adding deps like uuid.
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let ytdlp_token = format!("mh-{}", ts);
    println!("[*] yt-dlp aria2 token: {}", ytdlp_token);

    // Force progress output onto separate lines so BufReader can stream
    // status updates while the direct download is running.
    args.push("--progress".to_string());
    args.push("--newline".to_string());

    // YouTube throttles individual connections on DASH streams. Download a
    // few fragments concurrently while keeping the buffer reasonably small so
    // memory stays flat even on large playlists/files.
    args.push("--concurrent-fragments".to_string());
    args.push("4".to_string());
    args.push("--buffer-size".to_string());
    args.push("1M".to_string());
    // Chunked HTTP requests are yt-dlp's recommended way around
    // per-connection throttling. 10M keeps the ceiling low (about 40MB
    // worst case with 4 concurrent fragments) while still helping
    // single-stream progressive downloads.
    args.push("--http-chunk-size".to_string());
    args.push("10M".to_string());

    // Append the URL as the final positional argument so yt-dlp sees it
    // when we run the same `args` Vec directly (best-quality path).
    // For metadata-only runs we still copy `args` without this URL into
    // `meta_args` below, then push the URL there.
    args.push(url.clone());

    // Instead of only getting the URL, request yt-dlp JSON metadata so a sensible filename can be built.
    // Include cookies and UA so metadata resolves for private videos.
    let mut meta_args: Vec<String> = Vec::new();
    // Avoid loading any user/global yt-dlp config which may force formats or other options
    meta_args.push("--no-config".to_string());
    meta_args.push("-j".to_string()); // print JSON
    meta_args.push("--no-warnings".to_string());
    meta_args.push("--socket-timeout".to_string());
    meta_args.push("20".to_string());
    meta_args.push("--no-check-formats".to_string());
    meta_args.push("--extractor-args".to_string());
    meta_args.push(YT_PLAYER_CLIENTS.to_string());
    if let Some(ref cp) = cookie_path {
        meta_args.push("--cookies".to_string());
        meta_args.push(cp.clone());
    } else {
        meta_args.push("--cookies-from-browser".to_string());
        meta_args.push(selected_browser.clone());
    }
    meta_args.push("--user-agent".to_string());
    meta_args.push("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());
    meta_args.push("--impersonate".to_string());
    meta_args.push("chrome".to_string());
    push_yt_runtime_args(&mut meta_args, &app)?;
    push_yt_proxy_args(&mut meta_args, &app)?;
    meta_args.push(url.clone());

    println!(
        "[~] Running yt-dlp to fetch metadata: {:?}",
        redact_proxy_args(&meta_args)
    );

    let _ = app.emit("yt-log", "[meta] Fetching video metadata...".to_string());

    let bin_path = yt_dlp_bin_path(&app)?;
    let output = run_yt_bin(&app, &bin_path, &meta_args, "meta");

    match output {
        Ok(out) => {
            // final_out will hold the successful process output (initial or retry)
            let mut final_out = out;

            if !final_out.status.success() {
                let stderr = String::from_utf8_lossy(&final_out.stderr).to_string();
                println!("[!] yt-dlp failed to get metadata: {}", stderr);

                // Retry: some videos fail when yt-dlp is implicitly asked for a format not available.
                // Try a secondary metadata attempt forcing -f best (no-config) before giving up.
                println!("[~] Retrying yt-dlp metadata with -f best (no-config)");
                let mut retry_args = meta_args.clone();
                // url is last element; pop it, insert -f best, then push url back
                let url_arg = retry_args.pop().unwrap_or_else(|| url.clone());
                retry_args.push("-f".to_string());
                retry_args.push("best".to_string());
                retry_args.push(url_arg.clone());

                let retry_output = run_yt_bin(&app, Path::new(&bin_path), &retry_args, "retry");

                if let Ok(rout) = retry_output {
                    if rout.status.success() {
                        println!("[✓] yt-dlp metadata retry succeeded");
                        final_out = rout; // use retry output for downstream parsing
                    } else {
                        // If retry also failed, try to collect available formats to help debugging
                        let mut list_args = vec![
                            "--no-config".to_string(),
                            "--list-formats".to_string(),
                        ];
                        push_yt_proxy_args(&mut list_args, &app)?;
                        list_args.push(url.clone());
                        let list_out =
                            run_yt_bin(&app, Path::new(&bin_path), &list_args, "formats");
                        let mut formats_txt = String::new();
                        if let Ok(lo) = list_out {
                            // include both stdout and stderr since yt-dlp may write to either
                            let so = String::from_utf8_lossy(&lo.stdout).to_string();
                            let se = String::from_utf8_lossy(&lo.stderr).to_string();
                            formats_txt = format!("{}\n{}", so, se).trim().to_string();
                        }
                        println!(
                            "[✗] yt-dlp metadata retry failed: {}\nFormats:\n{}",
                            String::from_utf8_lossy(&rout.stderr),
                            formats_txt
                        );
                        return Err(format!(
                            "yt-dlp failed: {}\nAvailable formats:\n{}",
                            String::from_utf8_lossy(&rout.stderr),
                            formats_txt
                        ));
                    }
                } else {
                    return Err(format!("yt-dlp failed to execute retry: {}", stderr));
                }
            }

            let stdout = String::from_utf8_lossy(&final_out.stdout).to_string();
            // Extract first JSON object from stdout
            let start = stdout
                .find('{')
                .ok_or("yt-dlp returned no JSON metadata".to_string())?;
            let end = stdout
                .rfind('}')
                .ok_or("yt-dlp returned malformed JSON".to_string())?;
            let json_str = &stdout[start..=end];
            let meta: serde_json::Value = serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))?;

            let _ = app.emit(
                "yt-log",
                format!(
                    "[meta] Title: {}",
                    meta.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                ),
            );

            let urls: Vec<String> = choose_media_urls(&meta);

            if urls.is_empty() {
                return Err("yt-dlp returned no resolved URLs".to_string());
            }

            // Build filename early so both code paths (aria2 below and
            // the direct-yt-dlp branch above) can use it.
            let title = meta
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("download");
            let ext = if format.to_lowercase() == "mp3" {
                "mp3"
            } else {
                "mp4"
            };
            let safe_title: String = title
                .chars()
                .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
                .collect();
            let filename = format!("{}.{}", safe_title, ext);

            // ------------------------------------------------------------
            // Decide: hand off to aria2 OR run yt-dlp directly.
            // YouTube's CDN throttles parallel connections per-connection
            // (not per-IP). aria2's split+max-connection-per-server=4 makes
            // Best-quality downloads grind to ~2KB/s or stall at 0KB.
            // For Best quality we already need mp4 merging, so let yt-dlp
            // do the whole job (download + mp4box remux) — same code path
            // as the 403 fallback. Specific resolutions (1080p/720p/...)
            // use pre-muxed formats that are fine through aria2 with
            // split=1.
            //
            // We check the original `quality` BEFORE it gets consumed by
            // `quality.unwrap_or_else(...)` further up.
            // ------------------------------------------------------------
            let best_quality = orig_quality
                .as_deref()
                .map(|q| q == "best" || q.is_empty())
                .unwrap_or(true)
                && format.to_lowercase() != "mp3";

            // Pre-detect whether yt-dlp returned a single muxed URL or
            // separate audio+video streams. If separate, aria2 is a bad
            // path even at split=1 because we lose the audio stream.
            let has_separate_streams = meta
                .get("requested_formats")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len() >= 2)
                .unwrap_or(false);

            if best_quality || has_separate_streams || format_id.is_some() {
                let gid = format!("yt-dlp-direct-{}", ts);
                println!(
                    "[~] Using direct yt-dlp download in background (best quality or split streams): {:?}",
                    redact_proxy_args(&args)
                );

                let expected_path = Path::new(&download_dir).join(&filename);
                let active_record = DownloadRecord {
                    gid: gid.clone(),
                    filename: filename.clone(),
                    url: url.clone(),
                    size: "0".to_string(),
                    status: "active".to_string(),
                    progress: 0,
                    added_at: ts as i64,
                    path: Some(expected_path.to_string_lossy().to_string()),
                    connections: Some(1),
                    source: Some("yt-dlp".to_string()),
                    ytdlp_token: Some(ytdlp_token.clone()),
                    proxy: None,
                };
                if let Err(e) = crate::db::upsert_download(app.clone(), active_record) {
                    println!(
                        "[!] Failed to persist active yt-dlp download record (non-fatal): {}",
                        e
                    );
                }
                let _ = app.emit(
                    "yt-dl-started",
                    json!({"gid": gid, "filename": filename, "url": url}),
                );

                let task_app = app.clone();
                let task_bin = bin_path.clone();
                let task_args = args.clone();
                let task_dir = download_dir.clone();
                let task_filename = filename.clone();
                let task_url = url.clone();
                let task_token = ytdlp_token.clone();
                let task_gid = gid.clone();
                let added_at = ts as i64;

                std::thread::spawn(move || {
                    run_direct_yt_download(
                        task_app,
                        task_bin,
                        task_args,
                        task_gid,
                        task_filename,
                        task_url,
                        task_dir,
                        task_token,
                        added_at,
                    );
                });

                return Ok(gid);
            }

            // Handoff resolved media URLs to the app's aria2 via Aria2Client
            let mut options_map = Map::new();
            options_map.insert("out".to_string(), json!(filename));
            options_map.insert("split".to_string(), json!("1"));
            options_map.insert("max-connection-per-server".to_string(), json!("1"));
            options_map.insert("allow-overwrite".to_string(), json!("true"));
            if !download_dir.trim().is_empty() {
                options_map.insert("dir".to_string(), json!(download_dir.replace('\\', "/")));
            }
            let hdrs = collect_http_headers(&meta, &ytdlp_token, &url);
            options_map.insert("header".to_string(), json!(hdrs));

            let aria = Aria2Client::new()?;
            let _ = app.emit(
                "yt-log",
                format!(
                    "[aria2] Adding {} resolved URL(s) to download...",
                    urls.len()
                ),
            );
            let gid_str = aria.add_uri(&urls, options_map).await?;
            let _ = app.emit(
                "yt-log",
                format!("[aria2] Download queued — GID: {}", gid_str),
            );
            println!("[✓] Added to aria2 with GID: {}", gid_str);

            // Detect quick 403 failures from signed URLs
            if let Ok(details) = aria.tell_status(&gid_str).await {
                let stat = details.get("status").and_then(|s| s.as_str()).unwrap_or("");
                let err_msg = details
                    .get("errorMessage")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let err_code = details
                    .get("errorCode")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                if stat == "error"
                    && (err_msg.contains("status=403")
                        || err_msg.contains("403")
                        || err_code == "22")
                {
                    let _ = app.emit(
                        "yt-log",
                        "[fallback] aria2 403 — switching to direct yt-dlp download...".to_string(),
                    );
                    println!(
                        "[~] Aria2 reported 403 for GID {} — falling back to direct yt-dlp",
                        gid_str
                    );
                    let direct_out = run_yt_bin(&app, Path::new(&bin_path), &args, "fallback");
                    match direct_out {
                        Ok(dout) if dout.status.success() => {
                            return persist_complete(
                                &app,
                                format!("yt-dlp-local-{}", ts),
                                &filename,
                                &url,
                                &download_dir,
                                &ytdlp_token,
                                "yt-dlp-fallback",
                            );
                        }
                        Ok(dout) => {
                            let derr = String::from_utf8_lossy(&dout.stderr).to_string();
                            return Err(format!("yt-dlp direct download failed: {}", derr));
                        }
                        Err(e) => {
                            return Err(format!(
                                "Failed to execute yt-dlp for direct download: {}",
                                e
                            ));
                        }
                    }
                }
            }

            let expected_path = Path::new(&download_dir).join(&filename);
            let upsert_record = DownloadRecord {
                gid: gid_str.clone(),
                filename: filename.clone(),
                url: url.clone(),
                size: "0".to_string(),
                status: "active".to_string(),
                progress: 0,
                added_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64,
                path: Some(expected_path.to_string_lossy().to_string()),
                connections: Some(1),
                source: Some("yt-dlp".to_string()),
                ytdlp_token: Some(ytdlp_token.clone()),
                proxy: None,
            };
            if let Err(e) = crate::db::upsert_download(app.clone(), upsert_record) {
                println!(
                    "[!] Failed to persist yt-dlp download record (non-fatal): {}",
                    e
                );
            }
            let _ = app.emit(
                "yt-dl-started",
                json!({"gid": gid_str, "filename": filename, "source": "aria2"}),
            );
            return Ok(gid_str);
        }
        Err(e) => {
            return Err(format!("Failed to execute yt-dlp: {}", e));
        }
    }
}

#[command]
// ── yt-dlp Integration ────────────────────────────────────────────

pub fn check_yt_dlp_exists(app: tauri::AppHandle) -> Result<bool, String> {
    let exe_path =
        yt_dlp_bin_path(&app).map_err(|e| format!("yt-dlp lookup failed: {}", e))?;
    Ok(exe_path.exists())
}

/// Shared helper for resolving the `yt-dlp` executable path. Searches in
/// order:
///
/// 1. App-local data binaries (e.g. `%LOCALAPPDATA%/com.werlos.moonhunt/binaries`)
/// 2. Legacy `src-tauri/binaries/` from earlier development installs
/// 3. `yt-dlp.exe` on PATH (release installs / system installs)
pub fn yt_dlp_bin_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let packaged = app_bin_dir(app)?.join("yt-dlp-x86_64-pc-windows-msvc.exe");
    if packaged.exists() {
        return Ok(packaged);
    }
    let legacy = legacy_bin_dir()?.join("yt-dlp-x86_64-pc-windows-msvc.exe");
    if legacy.exists() {
        return Ok(legacy);
    }
    // Fall back to PATH lookup or plain `yt-dlp`
    Ok(std::path::PathBuf::from(yt_dlp_command_name()))
}

/// Resolve the effective media merger (bundled ffmpeg, or a system ffmpeg),
/// returning the path to the executable if one is available.
pub(crate) fn resolved_merger(app: &tauri::AppHandle) -> Option<PathBuf> {
    crate::merger::resolved_merger_path(app)
}

#[command]
pub fn get_yt_dlp_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = yt_dlp_bin_path(&app)?;
    Ok(path.to_string_lossy().into_owned())
}

#[command]
pub fn open_app_bin_folder(app: tauri::AppHandle) -> Result<String, String> {
    let dir = app_bin_dir(&app)?;
    tauri_plugin_opener::open_path(&dir, None::<&str>)
        .map_err(|e| format!("Failed to open folder {}: {}", dir.display(), e))?;
    Ok(format!("Opened folder: {}", dir.display()))
}

#[command]
pub fn open_yt_dlp_location(app: tauri::AppHandle) -> Result<String, String> {
    let exe_path = yt_dlp_bin_path(&app)?;
    if !exe_path.exists() {
        return Err("yt-dlp not installed".to_string());
    }
    let target = exe_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| exe_path.clone());
    tauri_plugin_opener::open_path(&target, None::<&str>)
        .map_err(|e| format!("Failed to open yt-dlp folder {}: {}", target.display(), e))?;
    Ok(format!("Opened yt-dlp folder: {}", target.display()))
}

#[command]
pub fn uninstall_yt_dlp(app: tauri::AppHandle) -> Result<String, String> {
    // Remove the yt-dlp binary from binaries/ directory.
    let exe_path = yt_dlp_bin_path(&app).map_err(|e| format!("yt-dlp lookup failed: {}", e))?;
    if exe_path.exists() {
        std::fs::remove_file(&exe_path).map_err(|e| format!("Failed to delete yt-dlp: {}", e))?;
        Ok("Uninstalled yt-dlp".to_string())
    } else {
        Err("yt-dlp not found".to_string())
    }
}

#[command]
pub async fn install_yt_dlp(app: tauri::AppHandle) -> Result<String, String> {
    use futures_util::StreamExt;
    use std::io::Write;

    crate::install_cancel::clear(YT_DLP_INSTALL_CANCEL_KEY);
    let client = crate::util::configured_http_client(&app)?;
    let download_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

    println!("[~] Downloading yt-dlp from {}", download_url);
    if crate::install_cancel::is_cancelled(YT_DLP_INSTALL_CANCEL_KEY) {
        return Err("Install cancelled".to_string());
    }
    let resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("yt-dlp HTTP error: {}", resp.status()));
    }
    if crate::install_cancel::is_cancelled(YT_DLP_INSTALL_CANCEL_KEY) {
        return Err("Install cancelled".to_string());
    }

    let total = resp.content_length().unwrap_or(0);

    let bin_dir = app_bin_dir(&app)?;
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    let out_path = bin_dir.join("yt-dlp-x86_64-pc-windows-msvc.exe");

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&out_path)
        .map_err(|e| format!("Failed to open output file: {}", e))?;

    let mut downloaded: u64 = 0;

    let mut stream = resp.bytes_stream();
    while let Some(item) = stream.next().await {
        if crate::install_cancel::is_cancelled(YT_DLP_INSTALL_CANCEL_KEY) {
            drop(file);
            let _ = std::fs::remove_file(&out_path);
            crate::install_cancel::clear(YT_DLP_INSTALL_CANCEL_KEY);
            return Err("Install cancelled".to_string());
        }

        let chunk = item.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let _ = app.emit(
            "yt_install_progress",
            json!({
                "downloaded": downloaded,
                "total": total,
                "percent": percent
            }),
        );
    }

    if crate::install_cancel::is_cancelled(YT_DLP_INSTALL_CANCEL_KEY) {
        drop(file);
        let _ = std::fs::remove_file(&out_path);
        crate::install_cancel::clear(YT_DLP_INSTALL_CANCEL_KEY);
        return Err("Install cancelled".to_string());
    }

    file.flush()
        .map_err(|e| format!("Failed to flush file: {}", e))?;
    drop(file);

    if crate::install_cancel::is_cancelled(YT_DLP_INSTALL_CANCEL_KEY) {
        let _ = std::fs::remove_file(&out_path);
        crate::install_cancel::clear(YT_DLP_INSTALL_CANCEL_KEY);
        return Err("Install cancelled".to_string());
    }

    // Emit completion (100%) if total was unknown
    let _ = app.emit(
        "yt_install_progress",
        json!({
            "downloaded": downloaded,
            "total": total,
            "percent": 100.0
        }),
    );

    crate::install_cancel::clear(YT_DLP_INSTALL_CANCEL_KEY);
    println!("[✓] yt-dlp saved to {}", out_path.display());
    Ok(format!("Saved yt-dlp to {}", out_path.display()))
}

#[command]
pub fn cancel_yt_dlp_install() -> Result<String, String> {
    crate::install_cancel::request_cancel(YT_DLP_INSTALL_CANCEL_KEY);
    Ok("Cancelling yt-dlp install".to_string())
}

#[command]
// ── Browser Extension (EJS) Management ────────────────────────────

pub fn uninstall_ejs(app: tauri::AppHandle) -> Result<String, String> {
    // Remove EJS solver files from binaries/.
    let ejs_dir = ejs_bin_dir(&app)?;
    if ejs_dir.exists() {
        std::fs::remove_dir_all(&ejs_dir)
            .map_err(|e| format!("Failed to delete EJS dir: {}", e))?;
        Ok("Uninstalled EJS files".to_string())
    } else {
        Err("EJS not found".to_string())
    }
}

#[command]
pub fn check_ejs_exists(app: tauri::AppHandle) -> Result<bool, String> {
    // Check if EJS solver scripts exist in app-local data binaries/.
    let ejs_dir = ejs_bin_dir(&app)?;
    // Consider installed if primary solver file exists
    Ok(ejs_dir.join("yt.solver.lib.min.js").exists() || ejs_dir.join("yt.solver.lib.js").exists())
}

fn ejs_bin_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let new_dir = app_bin_dir(app)?.join("ejs");
    let legacy_dir = legacy_bin_dir()?.join("ejs");
    let solver_exists = |dir: &Path| {
        dir.join("yt.solver.lib.min.js").exists() || dir.join("yt.solver.lib.js").exists()
    };
    if solver_exists(&new_dir) {
        return Ok(new_dir);
    }
    if solver_exists(&legacy_dir) {
        return Ok(legacy_dir);
    }
    Ok(new_dir)
}

#[command]
pub fn get_ejs_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(ejs_bin_dir(&app)?.to_string_lossy().into_owned())
}

#[command]
pub fn open_ejs_location(app: tauri::AppHandle) -> Result<String, String> {
    let ejs_dir = ejs_bin_dir(&app)?;
    if !ejs_dir.exists() {
        return Err("EJS not installed".to_string());
    }
    tauri_plugin_opener::open_path(&ejs_dir, None::<&str>)
        .map_err(|e| format!("Failed to open EJS folder {}: {}", ejs_dir.display(), e))?;
    Ok(format!("Opened EJS folder: {}", ejs_dir.display()))
}

#[command]
pub async fn install_ejs(app: tauri::AppHandle) -> Result<String, String> {
    use futures_util::StreamExt;
    use std::io::Write;

    crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
    let client = crate::util::configured_http_client(&app)?;
    let api_url = "https://api.github.com/repos/yt-dlp/ejs/releases/latest";

    println!(
        "[~] Querying EJS latest release via GitHub API: {}",
        api_url
    );
    let resp = client
        .get(api_url)
        .header("User-Agent", "moonhunt")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to query GitHub API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API HTTP error: {}", resp.status()));
    }

    let release: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub release JSON: {}", e))?;

    let assets = release
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or("No assets found in release")?;

    // Prefer minified solver files
    let preferred = [
        "yt.solver.lib.min.js",
        "yt.solver.lib.js",
        "yt.solver.core.min.js",
        "yt.solver.core.js",
    ];
    let mut to_download: Vec<(String, String)> = Vec::new(); // (name, url)

    for p in &preferred {
        for asset in assets {
            if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                if name == *p {
                    if let Some(url) = asset.get("browser_download_url").and_then(|u| u.as_str()) {
                        to_download.push((name.to_string(), url.to_string()));
                    }
                }
            }
        }
    }

    // If none matched, try to collect any .js files as fallback
    if to_download.is_empty() {
        for asset in assets {
            if let Some(name) = asset.get("name").and_then(|n| n.as_str()) {
                if name.ends_with(".js") {
                    if let Some(url) = asset.get("browser_download_url").and_then(|u| u.as_str()) {
                        to_download.push((name.to_string(), url.to_string()));
                    }
                }
            }
        }
    }

    if to_download.is_empty() {
        return Err("No suitable JS assets found in ejs release".to_string());
    }

    let ejs_dir = app_bin_dir(&app)?.join("ejs");
    std::fs::create_dir_all(&ejs_dir).map_err(|e| format!("Failed to create ejs dir: {}", e))?;

    let mut written: Vec<PathBuf> = Vec::new();
    for (name, url) in to_download.iter() {
        if crate::install_cancel::is_cancelled(EJS_INSTALL_CANCEL_KEY) {
            cleanup_written_files(&written);
            crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
            return Err("Install cancelled".to_string());
        }

        println!("[~] Downloading {} -> {}", name, url);
        let resp = client
            .get(url)
            .header("User-Agent", "moonhunt")
            .send()
            .await
            .map_err(|e| format!("Failed to download {}: {}", name, e))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Download HTTP error for {}: {}",
                name,
                resp.status()
            ));
        }
        if crate::install_cancel::is_cancelled(EJS_INSTALL_CANCEL_KEY) {
            cleanup_written_files(&written);
            crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
            return Err("Install cancelled".to_string());
        }
        let total = resp.content_length().unwrap_or(0);
        let out_path = ejs_dir.join(name);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&out_path)
            .map_err(|e| format!("Failed to open {}: {}", out_path.display(), e))?;
        written.push(out_path.clone());
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();
        while let Some(item) = stream.next().await {
            if crate::install_cancel::is_cancelled(EJS_INSTALL_CANCEL_KEY) {
                drop(file);
                cleanup_written_files(&written);
                crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
                return Err("Install cancelled".to_string());
            }

            let chunk = item.map_err(|e| format!("Stream error: {}", e))?;
            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
            downloaded += chunk.len() as u64;
            let percent = if total > 0 {
                (downloaded as f64 / total as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            let _ = app.emit(
                "ejs_install_progress",
                json!({"name": name, "downloaded": downloaded, "total": total, "percent": percent}),
            );
        }
        if crate::install_cancel::is_cancelled(EJS_INSTALL_CANCEL_KEY) {
            drop(file);
            cleanup_written_files(&written);
            crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
            return Err("Install cancelled".to_string());
        }
        file.flush()
            .map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);
    }

    if crate::install_cancel::is_cancelled(EJS_INSTALL_CANCEL_KEY) {
        cleanup_written_files(&written);
        crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
        return Err("Install cancelled".to_string());
    }

    crate::install_cancel::clear(EJS_INSTALL_CANCEL_KEY);
    Ok(format!("Saved EJS assets to {}", ejs_dir.display()))
}

#[command]
pub fn cancel_ejs_install() -> Result<String, String> {
    crate::install_cancel::request_cancel(EJS_INSTALL_CANCEL_KEY);
    Ok("Cancelling EJS install".to_string())
}

fn cleanup_written_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_yt_dlp_progress() {
        let p = parse_yt_progress("[download]  45.2% of 10.30MiB at 1.50MiB/s ETA 00:04")
            .expect("plain progress line should parse");
        assert!(p.total > 0);
        assert!(p.completed > 0);
        assert_eq!(p.speed, 1_572_864);
    }

    #[test]
    fn parses_progress_with_ansi_prefix() {
        let p = parse_yt_progress("\u{1b}[K[download]  45.2% of 10.30MiB at 1.50MiB/s ETA 00:04")
            .expect("ansi-prefixed progress line should parse");
        assert!(p.total > 0);
        assert_eq!(p.speed, 1_572_864);
    }

    #[test]
    fn ignores_non_progress_lines() {
        assert!(parse_yt_progress("[youtube] Extracting URL: https://example.com").is_none());
    }
}
