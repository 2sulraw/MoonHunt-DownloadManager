//! Backend orchestration: shared tokio runtime, extension bridge startup,
//! and the aria2 poller thread. Ported 1:1 from the old Tauri lib.rs setup()
//! with `app_handle.emit(...)` now feeding the UI event channel instead of a WebView.

use crate::ctx::AppHandle;
use crate::state::{lock, set_current_rpc_port, DownloadRecord, ExtensionSettings, ProxySettings};
use serde_json::Value;

/// aria2 reports numeric fields as strings; accept either form.
fn aria2_u64(value: Option<&Value>) -> u64 {
    value
        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
        .or_else(|| value.and_then(|v| v.as_u64()))
        .unwrap_or(0)
}

/// One-shot bootstrap the app calls at startup (replaces most of Tauri's
/// `setup()`: DB already initialized by the app, extension bridge started,
/// and the aria2 poller thread spawned).
pub fn start_backend(app: &AppHandle) -> Result<(), String> {
    let mut settings = crate::db::load_settings_from_db().unwrap_or_else(|_| ProxySettings::default());
    crate::state::apply_env_proxy(&mut settings);
    set_current_rpc_port(crate::state::effective_rpc_port(settings.rpc_port));

    // Configure file logging from the saved setting (off by default).
    let app_data = app.data_dir().unwrap_or_default();
    crate::logfile::configure_logging(settings.debug_logging, app_data.clone());
    if settings.debug_logging {
        println!("[log] debug logging enabled -> {}/moonhunt.log", app_data.display());
    }

    {
        let mut guard = lock(&app.state().proxy_settings)?;
        *guard = settings.clone();
    }

    // Mirror saved extension bridge settings (port + auth) so the HTTP
    // server enforces auth from the first boot, not after a re-save.
    {
        let mut ext_guard = lock(&app.state().extension_settings)?;
        *ext_guard = ExtensionSettings {
            extension_port: settings.extension_port,
            extension_auth_enabled: settings.extension_auth_enabled,
            extension_auth_token: settings.extension_auth_token,
            open_in_browser_protocol_enabled: settings.open_in_browser_protocol_enabled,
        };
    }

    crate::extension::start_http_server(
        app.state_arc().pending_downloads.clone(),
        app.state_arc().extension_settings.clone(),
        app.clone(),
    );

    spawn_poller(app.clone());
    Ok(())
}
/// The aria2 poller: a dedicated OS thread with its own tokio runtime.
pub fn spawn_poller(app: AppHandle) {
    let app_handle = app;
    // Spawn background thread that polls aria2 and emits download state events.
    // Filters out cancelled GIDs so they don't re-appear as "complete".
    std::thread::spawn(move || {
            // current_thread: the poller only sleeps and does two RPC calls — a
            // multi-thread runtime would park 12 idle workers for its lifetime.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for aria2 poller");
            let mut last_json = String::new();
            let mut last_stopped: Vec<Value> = Vec::new();
            let mut last_live_stats: Vec<(String, u32, Option<u64>)> = Vec::new();
            let mut retried_gids: std::collections::HashSet<String> = std::collections::HashSet::new();
            // Gids whose terminal state has already been persisted this run,
            // so a stopped transfer is written to the DB exactly once. The DB â€”
            // not the frontend â€” is the durable source for "was this completed".
            let mut persisted_terminal: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut last_status: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            // Connection-drop detection: per gid, the last observed byte
            // count, when that count froze, and whether the drop was
            // already reported. (bytes, frozen_at, reported)
            let mut stall_state: std::collections::HashMap<String, (u64, Option<std::time::Instant>, bool)> = std::collections::HashMap::new();
            let mut poll_tick: u32 = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                poll_tick = poll_tick.wrapping_add(1);
                // Completed downloads are comparatively expensive to
                // fetch/serialize, and they rarely change second-to-second.
                // Poll active/waiting every second but only refresh the
                // full stopped history on the first tick and every 5s.
                let include_stopped = poll_tick == 1 || poll_tick % 5 == 0;
                // Read the DB *before* the live snapshot. `add_download` persists
                // its row only after aria2 has accepted the gid, so any row read
                // here is guaranteed to be in the snapshot taken just after —
                // otherwise a download added in the gap (a browser-extension
                // download is the usual one) looked like an orphan and got
                // re-added as paused / flipped to error.
                let orphan_scan_rows = if include_stopped {
                    Some(crate::db::get_all_downloads_from_db(app_handle.clone()).unwrap_or_default())
                } else {
                    None
                };
                let client = match crate::aria2::Aria2Client::new() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let result = rt.block_on(async {
                    let (active, waiting) = futures_util::future::join(
                        client.tell_active(),
                        client.tell_waiting(0, 100),
                    ).await;
                    let mut all = Vec::new();
                    if let Ok(a) = active { all.extend(a); }
                    if let Ok(w) = waiting { all.extend(w); }
                    if include_stopped {
                        if let Ok(s) = client.tell_stopped(0, 1000).await {
                            all.extend(s.clone());
                            last_stopped = s;
                        }
                    }
                    all
                });

                let cancelled_gids: Vec<String> = crate::extension::cancelled_gids();

                // Record state-change events for the download log.
                for item in &result {
                    if let Some(gid) = item.get("gid").and_then(|v| v.as_str()) {
                        let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if let Some(prev) = last_status.get(gid) {
                            if prev != status {
                                crate::db::insert_download_event(gid, "state_change", &format!("{} â†’ {}", prev, status));
                            }
                        } else if !status.is_empty() {
                            crate::db::insert_download_event(gid, "info", &format!("tracking started: {}", status));
                        }
                        last_status.insert(gid.to_string(), status.to_string());
                    }
                }

                // Persist completed/error transfers to the DB. This runs on
                // the backend, so a download that finished (or failed) while
                // the UI is closed still gets a durable terminal row. The
                // poller is the authority here because aria2 only reports a
                // task as complete once it leaves the active list, and that
                // is exactly the event the DB needs to record.
                fn persist_terminal_row(
                    app_handle: &crate::ctx::AppHandle,
                    item: &Value,
                    persisted: &mut std::collections::HashSet<String>,
                    cancelled: &[String],
                ) {
                    let Some(gid) = item.get("gid").and_then(|v| v.as_str()) else { return };
                    if persisted.contains(gid) || cancelled.contains(&gid.to_string()) {
                        return;
                    }
                    let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if status != "complete" && status != "error" && status != "removed" {
                        return;
                    }
                    // "removed" is aria2's cancelled/removed terminal state. Keep
                    // it out of the DB as a live row: persist as error so it does
                    // not resurrect as active, but never as the old "active".
                    let db_status = if status == "complete" { "complete" } else { "error" };

                    let file = item.get("files")
                        .and_then(|f| f.as_array())
                        .and_then(|arr| arr.first());
                    let filename = file
                        .and_then(|f| f.get("path").and_then(|p| p.as_str()))
                        .and_then(|p| std::path::Path::new(p).file_name().and_then(|n| n.to_str()))
                        .unwrap_or(gid)
                        .to_string();
                    let path = file
                        .and_then(|f| f.get("path").and_then(|p| p.as_str()))
                        .map(|p| p.to_string());
                    let first_uri = file
                        .and_then(|f| f.get("uris").and_then(|u| u.as_array()))
                        .and_then(|uris| uris.first())
                        .and_then(|u| u.get("uri").and_then(|v| v.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let total = aria2_u64(item.get("totalLength"));
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;

                    let record = DownloadRecord {
                        gid: gid.to_string(),
                        filename,
                        url: first_uri,
                        size: crate::util::format_bytes(total),
                        status: db_status.to_string(),
                        progress: 100,
                        added_at: now_ms,
                        path,
                        connections: Some(1),
                        source: Some("aria2".to_string()),
                        ytdlp_token: None,
                        proxy: None,
                        size_bytes: Some(total),
                    };
                    if crate::db::upsert_download(app_handle.clone(), record).is_ok() {
                        persisted.insert(gid.to_string());
                    }
                }

                // A transfer that finished in the current `active` snapshot is
                // persisted now, before it drops into stopped history.
                for item in &result {
                    persist_terminal_row(&app_handle, item, &mut persisted_terminal, &cancelled_gids);
                }

                // The stopped list is only refreshed every 5s; use the last
                // cached snapshot so a transfer that finished between refreshes
                // is still persisted on the next tick.
                for item in &last_stopped {
                    persist_terminal_row(&app_handle, item, &mut persisted_terminal, &cancelled_gids);
                }

                // Keep live rows' DB record in sync with aria2's real sizes so
                // that a mid-download kill leaves an accurate `size` (e.g.
                // "128 MB / 1.2 GB") instead of the "0 KB / 0 KB" written at
                // add time (aria2 only discovers Content-Length once it starts
                // fetching). Then the reopened paused card shows real bytes.
                // Throttled to once per 5s â€” 1-2 rows typically, cheap.
                if include_stopped {
                    for item in &result {
                        // Only sync genuinely live tasks. `result` also
                        // contains the whole stopped history on these
                        // ticks; without this guard every completed
                        // download's DB row is rewritten back to `active`
                        // every 5s â€” and after a relaunch the orphan sweep
                        // then flips those fake actives to `error`, because
                        // completion removes the .aria2 control file.
                        let st = item.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        if st != "active" && st != "waiting" && st != "paused" {
                            continue;
                        }
                        let (Some(gid), Some(filename)) = (
                            item.get("gid").and_then(|v| v.as_str()),
                            item.get("files").and_then(|f| f.as_array()).and_then(|a| a.first())
                                .and_then(|f| f.get("path").and_then(|p| p.as_str()))
                                .and_then(|p| std::path::Path::new(p).file_name().and_then(|n| n.to_str())),
                        ) else { continue };
                        let total = aria2_u64(item.get("totalLength"));
                        let completed = aria2_u64(item.get("completedLength"));
                        if total == 0 { continue; }
                        let display = format!("{} / {}", crate::util::format_bytes(completed), crate::util::format_bytes(total));
                        let progress = ((completed as f64 / total as f64) * 100.0).round() as i32;
                        let now_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64;
                        let _ = crate::db::upsert_download(app_handle.clone(), crate::state::DownloadRecord {
                            gid: gid.to_string(),
                            filename: filename.to_string(),
                            url: item.get("files").and_then(|f| f.as_array()).and_then(|a| a.first())
                                .and_then(|f| f.get("uris").and_then(|u| u.as_array()).and_then(|u| u.first()))
                                .and_then(|u| u.get("uri").and_then(|v| v.as_str())).unwrap_or("").to_string(),
                            size: display,
                            status: "active".to_string(),
                            progress,
                            added_at: now_ms,
                            path: item.get("files").and_then(|f| f.as_array()).and_then(|a| a.first())
                                .and_then(|f| f.get("path").and_then(|p| p.as_str())).map(|s| s.to_string()),
                            connections: item.get("connections").and_then(|v| v.as_str()).and_then(|s| s.parse::<i32>().ok()).or_else(|| item.get("connections").and_then(|v| v.as_i64().map(|n| n as i32))),
                            source: Some("aria2".to_string()),
                            ytdlp_token: None,
                            proxy: None,
                            size_bytes: Some(total),
                        });
                    }
                }

                // After a mid-download kill the DB keeps stale `active` rows
                // that aria2 never re-learns (session file is emptied on
                // relaunch). Any row not in the live set (and not cancelled)
                // is an orphan. If the partial + .aria2 control file still
                // exist on disk, re-add it to aria2 as PAUSED so it shows at
                // its real progress and the user can Resume to continue where
                // it died. Only rows with no resumable data are marked error.
                // Runs only on stopped-listing ticks (5s) so a download that
                // JUST completed (still in stopped cache) is never mistaken
                // for an orphan.
                if include_stopped {
                    let live_gid_set: std::collections::HashSet<String> = result
                        .iter()
                        .filter_map(|t| t.get("gid").and_then(|g| g.as_str()).map(String::from))
                        .collect();
                    // Direct yt-dlp downloads never appear in aria2's live
                    // set â€” but while their child process is registered as
                    // running they are NOT orphans. Without this, a just
                    // started direct download was flipped to `error` here
                    // and only recovered on the next progress tick
                    // (stuck â†’ error â†’ started flicker).
                    let live_yt_gids: std::collections::HashSet<String> = app_handle
                        .state()
                        .active_yt_downloads
                        .lock()
                        .map(|g| g.keys().cloned().collect())
                        .unwrap_or_default();
                    let mut orphan_persisted: Vec<String> = Vec::new();
                    let db_rows = orphan_scan_rows.clone().unwrap_or_default();
                    for row in db_rows {
                        let Some(gid) = row.get("gid").and_then(|v| v.as_str()) else { continue };
                        if cancelled_gids.contains(&gid.to_string())
                            || live_gid_set.contains(gid)
                            || live_yt_gids.contains(gid)
                            || orphan_persisted.iter().any(|g| g == gid)
                        {
                            continue;
                        }
                        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                        if status == "active" || status == "paused" || status == "waiting" {
                            let filename = row.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let stored_path = row.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            // `path` is either a full file path (live-sync / terminal
                            // persistence) or a plain directory (older add_download).
                            // The control file + partial live in the PARENT directory.
                            let p = std::path::Path::new(&stored_path);
                            let dir = if stored_path.ends_with(&filename) || p.extension().map(|e| filename.ends_with(&format!(".{}", e.to_string_lossy()))).unwrap_or(false) {
                                p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_else(|| stored_path.clone())
                            } else {
                                stored_path.clone()
                            };
                            // The .aria2 control file marks a resumable partial.
                            let control_file = std::path::Path::new(&dir).join(format!("{}.aria2", filename));
                            let resumable = !filename.is_empty()
                                && !url.is_empty()
                                && control_file.exists();

                            // Heal legacy corruption: the old size-sync block wrote
                            // every stopped-history item back as `active`. A finished
                            // download has no .aria2 control file but its complete
                            // file exists on disk â€” restore its true terminal state
                            // instead of erroring it as a broken partial.
                            if !resumable {
                                let full_file = std::path::Path::new(&dir).join(&filename);
                                if !filename.is_empty() && full_file.exists() {
                                    crate::logln!("[aria2] Orphan {} ({}) has complete file on disk â€” healing to complete", gid, filename);
                                    let healed = DownloadRecord {
                                        gid: gid.to_string(),
                                        filename: filename.clone(),
                                        url,
                                        size: row.get("size").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                                        status: "complete".to_string(),
                                        progress: 100,
                                        added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(0),
                                        path: Some(full_file.to_string_lossy().to_string()),
                                        connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as i32),
                                        source: row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        ytdlp_token: row.get("ytdlp_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        proxy: row.get("proxy").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    size_bytes: None,
                                    };
                                    if crate::db::upsert_download(app_handle.clone(), healed).is_ok() {
                                        orphan_persisted.push(gid.to_string());
                                    }
                                    continue;
                                }
                            }

                            if resumable {
                                crate::logln!("[aria2] Re-adding paused orphan {} ({}) â€” resumable partial found", gid, filename);
                                let client = match crate::aria2::Aria2Client::new() {
                                    Ok(c) => c,
                                    Err(_) => continue,
                                };
                                let mut options = serde_json::Map::new();
                                options.insert("out".to_string(), serde_json::json!(filename));
                                options.insert("dir".to_string(), serde_json::json!(dir));
                                options.insert("pause".to_string(), serde_json::json!("true"));
                                // Re-add under the ORIGINAL gid so the row keeps its identity:
                                // the upsert below overwrites the same DB row, the frontend
                                // "ghost" card and the live card merge on the shared gid (never
                                // two cards), and Resume unpauses the same download instead of
                                // creating a new one. aria2 >=1.23 honors the `gid` option.
                                options.insert("gid".to_string(), serde_json::json!(gid));
                                if let Some(c) = row.get("connections").and_then(|v| v.as_i64()) {
                                    let c = c.max(1).to_string();
                                    options.insert("split".to_string(), serde_json::json!(c));
                                    options.insert("max-connection-per-server".to_string(), serde_json::json!(c));
                                }
                                let add_result = rt.block_on(client.add_uri(&[url.clone()], options));
                                // A failed re-add must NOT rewrite the row as
                                // paused: the common cause is aria2 already
                                // owning the gid (the snapshot just missed it)
                                // or a transient RPC error. Leave the row as-is
                                // and let the next tick re-evaluate — rewriting
                                // it here is what made a fresh, still-running
                                // download show up paused.
                                let Ok(effective_gid) = add_result else {
                                    crate::logln!("[aria2] Could not re-add orphan {} — leaving row untouched", gid);
                                    continue;
                                };
                                if effective_gid != gid {
                                    crate::logln!("[aria2] Paused orphan {} re-added as {} (paused)", gid, effective_gid);
                                } else {
                                    crate::logln!("[aria2] Paused orphan {} re-added under same gid (paused)", gid);
                                }
                                let now_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as i64;
                                // The DB row already carries the accurate sizes: the live-sync
                                // above persisted `totalLength`/`completedLength` on every 5s
                                // poll, so a killed download's row shows e.g. "128 MB / 1.2 GB"
                                // even though aria2 reports 0/0 for a freshly paused task.
                                let record = DownloadRecord {
                                    gid: effective_gid.clone(),
                                    filename: filename.clone(),
                                    url: url.clone(),
                                    size: row.get("size").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                                    status: "paused".to_string(),
                                    progress: row.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                    added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(now_ms),
                                    // Keep `path` as the full file path (UI/fileState expect it).
                                    path: Some(std::path::Path::new(&dir).join(&filename).to_string_lossy().to_string()),
                                    connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as i32),
                                    source: row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    ytdlp_token: row.get("ytdlp_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    proxy: row.get("proxy").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    size_bytes: None,
                                };
                                let _ = crate::db::upsert_download(app_handle.clone(), record);
                                if effective_gid != gid {
                                    // Defensive: only if aria2 ignored the requested gid, drop
                                    // the old row so the UI never shows two cards.
                                    let _ = crate::db::remove_download_from_db(app_handle.clone(), gid.to_string());
                                }
                            } else {
                                crate::logln!("[aria2] Orphaned {} ({}) not resumable â€” marking error", gid, filename);
                                let record = DownloadRecord {
                                    gid: gid.to_string(),
                                    filename: filename.clone(),
                                    url,
                                    size: row.get("size").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
                                    status: "error".to_string(),
                                    progress: row.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                                    added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(0),
                                    path: row.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as i32),
                                    source: row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    ytdlp_token: row.get("ytdlp_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    proxy: row.get("proxy").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    size_bytes: None,
                                };
                                if crate::db::upsert_download(app_handle.clone(), record).is_ok() {
                                    orphan_persisted.push(gid.to_string());
                                }
                            }
                        }
                    }
                }

                // Live stats (connections + remaining time) change often,
                // so push a tiny payload on every change instead of waiting
                // for the full snapshot to differ. Both reuse this poll.
                let live_stats: Vec<(String, u32, Option<u64>)> = result
                    .iter()
                    .filter(|item| {
                        let status = item.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        status == "active" || status == "waiting"
                    })
                    .filter_map(|item| {
                        let gid = item.get("gid").and_then(|g| g.as_str())?.to_string();
                        if cancelled_gids.contains(&gid) {
                            return None;
                        }
                        let count = item
                            .get("connections")
                            .and_then(|v| {
                                v.as_str()
                                    .and_then(|s| s.parse::<u32>().ok())
                                    .or_else(|| v.as_u64().map(|n| n as u32))
                            })
                            .unwrap_or(1);
                        let total = aria2_u64(item.get("totalLength"));
                        let completed = aria2_u64(item.get("completedLength"));
                        let speed = aria2_u64(item.get("downloadSpeed"));
                        let remaining_seconds = if speed > 0 && total > completed {
                            Some((total - completed).div_ceil(speed))
                        } else {
                            None
                        };
                        Some((gid, count, remaining_seconds))
                    })
                    .collect();

                if live_stats != last_live_stats {
                    let payload: Vec<Value> = live_stats
                        .iter()
                        .map(|(gid, count, remaining_seconds)| {
                            serde_json::json!({
                                "gid": gid,
                                "connections": count,
                                "remainingSeconds": remaining_seconds
                            })
                        })
                        .collect();
                    last_live_stats = live_stats;
                    let _ = app_handle.emit("live-stats-changed", &payload);
                }

                // Connection-drop detection: an active download whose byte
                // count stops advancing for STALL_TIMEOUT_SECS is a stalled
                // (timed out) connection. Reported once per stall, with a
                // recovery line when bytes flow again. Replaces the old
                // speed-sample spam â€” this is what actually matters.
                {
                    const STALL_TIMEOUT_SECS: u64 = 20;
                    let mut live_gids: std::collections::HashSet<String> = std::collections::HashSet::new();
                    for item in &result {
                        let st = item.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        if st != "active" { continue; }
                        let Some(gid) = item.get("gid").and_then(|v| v.as_str()) else { continue };
                        if cancelled_gids.contains(&gid.to_string()) { continue; }
                        live_gids.insert(gid.to_string());
                        let completed = aria2_u64(item.get("completedLength"));
                        let total = aria2_u64(item.get("totalLength"));
                        let entry = stall_state.entry(gid.to_string()).or_insert((completed, None, false));
                        if completed != entry.0 {
                            // Bytes moved again: clear the freeze and note
                            // a recovery when the stall was reported.
                            let was_reported = entry.2;
                            let frozen_at = entry.1;
                            *entry = (completed, None, false);
                            if was_reported {
                                let secs = frozen_at
                                    .map(|t| t.elapsed().as_secs())
                                    .unwrap_or(0);
                                crate::db::insert_download_event(
                                    gid,
                                    "retry",
                                    &format!("connection restored after {}s stall", secs),
                                );
                            }
                        } else if entry.2 {
                            // Still frozen; nothing to do.
                        } else {
                            if entry.1.is_none() {
                                entry.1 = Some(std::time::Instant::now());
                            }
                            if let Some(frozen_at) = entry.1 {
                                let elapsed = frozen_at.elapsed().as_secs();
                                if elapsed >= STALL_TIMEOUT_SECS {
                                    entry.2 = true;
                                    let speed = aria2_u64(item.get("downloadSpeed"));
                                    let connections = item.get("connections").and_then(|v| {
                                        v.as_str().and_then(|s| s.parse::<u32>().ok())
                                            .or_else(|| v.as_u64().map(|n| n as u32))
                                    }).unwrap_or(0);
                                    let detail = format!(
                                        "connection drop: no progress for {}s at {}/{} (speed {}/s, connections {})",
                                        elapsed,
                                        crate::util::format_bytes(completed),
                                        crate::util::format_bytes(total),
                                        crate::util::format_bytes(speed),
                                        connections,
                                    );
                                    crate::db::insert_download_event(gid, "timeout", &detail);
                                }
                            }
                        }
                    }
                    // Drop tracking state for downloads that are no longer running.
                    stall_state.retain(|gid, _| live_gids.contains(gid));
                }

                // An empty active-only snapshot is "nothing new" rather than a
                // signal to clear the UI: the last stopped rows (and the DB
                // history merged below) keep the list populated between the
                // slower stopped polls.
                // Emit to desktop UI — filter cancelled GIDs. Owned clones so
                // the DB source enrichment (below) can patch yt-dlp rows' marker
                // headers without fighting the borrow.
                let mut ui_result: Vec<Value> = result.iter()
                    .filter(|t| {
                        let gid = t.get("gid").and_then(|v| v.as_str()).unwrap_or("");
                        !cancelled_gids.contains(&gid.to_string())
                    })
                    .cloned()
                    .collect();
                // Stopped history is only polled every few seconds, but the
                // active-only snapshots between those polls must not make
                // failed/completed rows disappear from the UI.
                if !include_stopped {
                    let mut live_gids = std::collections::HashSet::new();
                    for item in &ui_result {
                        if let Some(gid) = item.get("gid").and_then(|v| v.as_str()) {
                            live_gids.insert(gid.to_string());
                        }
                    }
                    for item in &last_stopped {
                        let gid = item.get("gid").and_then(|v| v.as_str()).unwrap_or("");
                        if !live_gids.contains(gid) && !cancelled_gids.contains(&gid.to_string()) {
                            ui_result.push(item.clone());
                        }
                    }
                }

                // DB source enrichment (old app merged the persisted row's
                // `source`/`ytdlp_token` into the aria2-mapped row, see
                // useDownloads.ts): a direct yt-dlp download lives in the DB
                // only, so once its live emits are exhausted the aria2
                // snapshot would drop the yt-dlp badge. Re-apply the synthetic
                // marker header the app-side detector looks for.
                if !ui_result.is_empty() {
                    let mut yt_tokens: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    // aria2 items carry no `addedAt`, so a finished row still in
                    // aria2's stopped history would sort as `added_at == 0` and
                    // land at the bottom of the list (it is also skipped from
                    // the DB history merge below). Copy the persisted stamp so
                    // newer rows keep sorting to the top.
                    let mut added_at_by_gid: std::collections::HashMap<String, i64> =
                        std::collections::HashMap::new();
                    if let Ok(rows) = crate::db::get_all_downloads_from_db(app_handle.clone()) {
                        for row in rows {
                            let Some(gid) = row.get("gid").and_then(|v| v.as_str()) else { continue };
                            if let Some(t) = row.get("added_at").and_then(|v| v.as_i64()) {
                                if t > 0 {
                                    added_at_by_gid.insert(gid.to_string(), t);
                                }
                            }
                            if row.get("source").and_then(|v| v.as_str()) == Some("yt-dlp") {
                                let tok = row
                                    .get("ytdlp_token")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("1")
                                    .to_string();
                                yt_tokens.insert(gid.to_string(), tok);
                            }
                        }
                    }
                    for item in &mut ui_result {
                        if let Some(gid) = item.get("gid").and_then(|v| v.as_str()) {
                            if let Some(&t) = added_at_by_gid.get(gid) {
                                item["addedAt"] = serde_json::json!(t);
                            }
                        }
                        let Some(token) = item
                            .get("gid")
                            .and_then(|v| v.as_str())
                            .and_then(|g| yt_tokens.get(g))
                            .cloned()
                        else { continue };
                        let header = format!("X-Moonhunt-YTDLP:{}", token);
                        item["requestHeaders"] = serde_json::json!([header]);
                        if let Some(files) = item
                            .get_mut("files")
                            .and_then(|fs| fs.as_array_mut())
                            .and_then(|fs| fs.first_mut())
                        {
                            files["requestHeaders"] = serde_json::json!([header]);
                            if let Some(uris) = files
                                .get_mut("uris")
                                .and_then(|us| us.as_array_mut())
                                .and_then(|us| us.first_mut())
                            {
                                uris["requestHeaders"] = serde_json::json!([header]);
                            }
                        }
                    }
                }

                // `completedNotInLive`: persisted rows the live snapshot no
                // longer carries (finished in an earlier session, or evicted
                // from aria2's in-memory stopped history). `useDownloads`
                // prepends them, so they go first here and the UI's sort
                // reverse (`sortAscending=false`) lands them under the live
                // rows, exactly like the old merged array.
                {
                    let live_gids: std::collections::HashSet<String> = ui_result
                        .iter()
                        .filter_map(|i| i.get("gid").and_then(|v| v.as_str()).map(str::to_owned))
                        .collect();
                    let db_rows = crate::db::get_all_downloads_from_db(app_handle.clone())
                        .unwrap_or_default();
                    let mut history: Vec<Value> = Vec::new();
                    for row in &db_rows {
                        let Some(gid) = row.get("gid").and_then(|v| v.as_str()) else { continue };
                        if live_gids.contains(gid) || cancelled_gids.contains(&gid.to_string()) {
                            continue;
                        }
                        // A row that still claims to be running belongs to the
                        // poller's orphan recovery, not to the history view.
                        let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if matches!(status, "active" | "waiting") {
                            continue;
                        }
                        history.push(serde_json::json!({
                            "gid": gid,
                            "status": status,
                            "filename": row.get("filename").cloned().unwrap_or_default(),
                            "path": row.get("path").cloned().unwrap_or_default(),
                            "url": row.get("url").cloned().unwrap_or_default(),
                            "size": row.get("size").cloned().unwrap_or_default(),
                            "sizeBytes": row.get("size_bytes").cloned().unwrap_or_default(),
                            "progress": row.get("progress").cloned().unwrap_or_default(),
                            "connections": row.get("connections").cloned().unwrap_or_default(),
                            "errorMessage": row.get("error_message").cloned().unwrap_or_default(),
                            "addedAt": row.get("added_at").cloned().unwrap_or_default(),
                        }));
                    }
                    if !history.is_empty() {
                        history.append(&mut ui_result);
                        ui_result = history;
                    }
                }

                let json = serde_json::to_string(&ui_result).unwrap_or_default();
                if json != last_json {
                    last_json = json;
                    // Log error details for failed downloads
                    for item in &ui_result {
                        if let Some(status) = item.get("status").and_then(|s| s.as_str()) {
                            if status == "error" || status == "removed" {
                                let gid = item.get("gid").and_then(|g| g.as_str()).unwrap_or("?");
                                let code = item.get("errorCode").and_then(|e| e.as_str()).unwrap_or("?");
                                let msg = item.get("errorMessage").and_then(|e| e.as_str()).unwrap_or("?");
                                let files = item.get("files").and_then(|f| f.as_array());
                                let total_len = item.get("totalLength").and_then(|t| t.as_str()).unwrap_or("0");
                                let completed = item.get("completedLength").and_then(|c| c.as_str()).unwrap_or("0");
                                println!("[aria2] Download {} failed: status={}, code={}, msg={}, completed={}/{}", gid, status, code, msg, completed, total_len);
                                crate::db::insert_download_event(gid, "error", &format!("[error {}] {}", code, msg));
                                if let Some(files) = files {
                                    for f in files {
                                        if let Some(path) = f.get("path").and_then(|p| p.as_str()) {
                                            println!("[aria2]   file: {}", path);
                                        }
                                        if let Some(uris) = f.get("uris").and_then(|u| u.as_array()) {
                                            for u in uris {
                                                let uri = u.get("uri").and_then(|v| v.as_str()).unwrap_or("?");
                                                let s = u.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                                                println!("[aria2]   uri: {} status={}", uri, s);
                                            }
                                        }
                                    }
                                }
                                // Auto-retry with reqwest for network errors (code 1)
                                if (code == "1" || code == "2") && !retried_gids.contains(gid) {
                                    retried_gids.insert(gid.to_string());
                                    if let Some(files) = files {
                                        if let Some(first_file) = files.first() {
                                            let uri = first_file.get("uris").and_then(|u| u.as_array())
                                                .and_then(|uris| uris.first())
                                                .and_then(|u| u.get("uri").and_then(|v| v.as_str()));
                                            if let Some(url) = uri {
                                                let fname = first_file.get("path").and_then(|p| p.as_str())
                                                    .map(|p| std::path::Path::new(p).file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or("download"))
                                                    .unwrap_or("download")
                                                    .to_string();
                                                let dir = first_file.get("path").and_then(|p| p.as_str())
                                                    .and_then(|p| std::path::Path::new(p).parent())
                                                    .map(|p| p.to_string_lossy().to_string());
                                                // Remove the stale .aria2 control file so a future
                                                // manual retry won't resume from corrupted state.
                                                if let Some(ref d) = dir {
                                                    let ctrl = std::path::PathBuf::from(d).join(format!("{}.aria2", fname));
                                                    if ctrl.exists() {
                                                        let _ = std::fs::remove_file(&ctrl);
                                                    }
                                                }
                                                println!("[aria2] Auto-retry with reqwest: {} â†’ {}", url, fname);
                                                let app_clone = app_handle.clone();
                                                let url_owned = url.to_string();
                                                let retry_headers = crate::aria2::lookup_download_headers(&url_owned);
                                                std::thread::spawn(move || {
                                                    let rt = tokio::runtime::Builder::new_current_thread()
                                                        .enable_all()
                                                        .build()
                                                        .expect("Failed to create tokio runtime for reqwest retry");
                                                    rt.block_on(async {
                                                        match crate::aria2::reqwest_download(
                                                            app_clone.clone(), url_owned, fname, dir, retry_headers,
                                                        ).await {
                                                            Ok(bytes) => println!("[reqwest] Auto-retry succeeded: {} bytes", bytes),
                                                            Err(e) => println!("[reqwest] Auto-retry also failed: {}", e),
                                                        }
                                                    });
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let _ = app_handle.emit("downloads-changed", &ui_result);
                }
            }
    });
}


