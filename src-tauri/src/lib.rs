//! # MoonHunt — Tauri Backend
//!
//! Core library for the MoonHunt download manager.
//! Handles aria2 RPC, yt-dlp, browser extension,
//! download scheduling, and SQLite persistence.

mod aria2;
mod clipboard;
mod db;
mod extension;
mod fs_ops;
mod hash;
mod install_cancel;
mod js_runtime;
mod logfile;
mod merger;
mod protocol;
mod state;
mod update;

use serde_json::Value;
mod util;
mod ytdlp;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use state::{lock, set_current_rpc_port, AppState, DownloadRecord, ExtensionSettings, ProxySettings};
use std::sync::{Arc, Mutex};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri_plugin_deep_link::DeepLinkExt;
use tauri::Emitter;
use tauri::Listener;
use tauri::Manager;
use tauri_plugin_single_instance; // ensure the plugin crate is linked

/// aria2 reports numeric fields as strings; accept either form.
fn aria2_u64(value: Option<&Value>) -> u64 {
    value
        .and_then(|v| v.as_str().and_then(|s| s.parse().ok()))
        .or_else(|| value.and_then(|v| v.as_u64()))
        .unwrap_or(0)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Main entry point. Called by main.rs.
pub fn run() {
    let pending_downloads = Arc::new(Mutex::new(Vec::new()));
    let extension_settings = Arc::new(Mutex::new(ExtensionSettings {
        extension_port: 6802,
        extension_auth_enabled: false,
        extension_auth_token: String::new(),
        open_in_browser_protocol_enabled: false,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch or a moonhunt:// deep-link woke the app. If the
            // window was hidden to the tray, reveal it.
            show_main_window(app);
        }))
        .manage(AppState {
            proxy_settings: Mutex::new(ProxySettings::default()),
            pending_downloads: pending_downloads.clone(),
            extension_settings: extension_settings.clone(),
            extension_cookies: Arc::new(Mutex::new(None)),
            active_yt_scan: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cancelled_yt_scans: Arc::new(Mutex::new(std::collections::HashSet::new())),
            active_yt_downloads: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.app_handle().emit("close-requested", ());
            }
        })
        .setup(move |app| {
            db::init_db(app.handle())?;

            let settings = db::load_settings_from_db().unwrap_or_else(|_| ProxySettings::default());
            set_current_rpc_port(settings.rpc_port);

            // Configure file logging from the saved setting (off by default).
            let app_data = app.path().app_data_dir().unwrap_or_default();
            logfile::configure_logging(settings.debug_logging, app_data.clone());
            if settings.debug_logging {
                println!("[log] debug logging enabled → {}/moonhunt.log", app_data.display());
            }

            let state = app.state::<AppState>();
            let mut guard = lock(&state.proxy_settings)?;
            *guard = settings.clone();

            // Mirror saved extension bridge settings (port + auth) so the HTTP
            // server enforces auth from the first boot, not after a re-save.
            {
                let mut ext_guard = lock(&state.extension_settings)?;
                *ext_guard = ExtensionSettings {
                    extension_port: settings.extension_port,
                    extension_auth_enabled: settings.extension_auth_enabled,
                    extension_auth_token: settings.extension_auth_token,
                    open_in_browser_protocol_enabled: settings.open_in_browser_protocol_enabled,
                };
            }

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval(
                    r#"
                    (function() {
                        document.addEventListener('contextmenu', function(e) {
                            e.preventDefault();
                        }, true);
                    })();
                    "#,
                );
            }

            // System tray: lets MoonHunt keep running hidden after the user closes the window.
            let tray_icon = app
                .default_window_icon()
                .cloned()
                .unwrap_or_else(|| {
                    tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                        .expect("failed to load tray icon")
                });
            let open_item = MenuItemBuilder::with_id("tray-open", "Open MoonHunt").build(app)?;
            let quit_item = MenuItemBuilder::with_id("tray-quit", "Exit").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .items(&[&open_item, &quit_item])
                .build()?;
            let tray = TrayIconBuilder::with_id("moonhunt-tray")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("MoonHunt")
                .icon(tray_icon)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "tray-open" => show_main_window(app),
                    "tray-quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                    | TrayIconEvent::DoubleClick { .. } => show_main_window(tray.app_handle()),
                    _ => {}
                })
                .build(app)?;
            app.manage(tray);

            let app_handle = app.handle().clone();

            // Register the moonhunt:// deep-link scheme so the browser extension can
            // wake/reveal the app. Best-effort — registration is not fatal.
            protocol::register_all(&app_handle);

            // On macOS, runtime deep links arrive via the plugin's on_open_url event.
            #[cfg(target_os = "macos")]
            {
                let wake_handle = app_handle.clone();
                app_handle.deep_link().on_open_url(move |_event| {
                    protocol::focus_main_window(&wake_handle);
                });
            }

            // On Windows/Linux, deep links to an already-running instance are
            // forwarded by single-instance → deep-link plugin, which emits a
            // `deep-link://new-url` event. Listen for it and surface the window.
            #[cfg(not(target_os = "macos"))]
            {
                let wake_handle = app_handle.clone();
                app_handle.listen("deep-link://new-url", move |event| {
                    println!("[deep-link] new-url event received: {:?}", event.payload());
                    protocol::focus_main_window(&wake_handle);
                });
            }

            // Start HTTP server for extension bridge now that app handle is available
            extension::start_http_server(pending_downloads.clone(), extension_settings.clone(), app_handle.clone());

            // Spawn background thread that polls aria2 and emits download state events.
            // Filters out cancelled GIDs so they don't re-appear as "complete".
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for aria2 poller");
                let mut last_json = String::new();
                let mut last_stopped: Vec<Value> = Vec::new();
                let mut last_live_stats: Vec<(String, u32, Option<u64>)> = Vec::new();
                let mut retried_gids: std::collections::HashSet<String> = std::collections::HashSet::new();
                // Gids whose terminal state has already been persisted this run,
                // so a stopped transfer is written to the DB exactly once. The DB —
                // not the frontend — is the durable source for "was this completed".
                let mut persisted_terminal: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut poll_tick: u32 = 0;
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    poll_tick = poll_tick.wrapping_add(1);
                    // Completed downloads are comparatively expensive to
                    // fetch/serialize, and they rarely change second-to-second.
                    // Poll active/waiting every second but only refresh the
                    // full stopped history on the first tick and every 5s.
                    let include_stopped = poll_tick == 1 || poll_tick % 5 == 0;
                    let client = match aria2::Aria2Client::new() {
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

                    let cancelled_gids: Vec<String> = extension::cancelled_gids();

                    // Persist completed/error transfers to the DB. This runs on
                    // the backend, so a download that finished (or failed) while
                    // the UI is closed still gets a durable terminal row. The
                    // poller is the authority here because aria2 only reports a
                    // task as complete once it leaves the active list, and that
                    // is exactly the event the DB needs to record.
                    fn persist_terminal_row(
                        app_handle: &tauri::AppHandle,
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
                    // Throttled to once per 5s — 1-2 rows typically, cheap.
                    if include_stopped {
                        for item in &result {
                            // Only sync genuinely live tasks. `result` also
                            // contains the whole stopped history on these
                            // ticks; without this guard every completed
                            // download's DB row is rewritten back to `active`
                            // every 5s — and after a relaunch the orphan sweep
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
                        let mut orphan_persisted: Vec<String> = Vec::new();
                        let db_rows = crate::db::get_all_downloads_from_db(app_handle.clone()).unwrap_or_default();
                        for row in db_rows {
                            let Some(gid) = row.get("gid").and_then(|v| v.as_str()) else { continue };
                            if cancelled_gids.contains(&gid.to_string())
                                || live_gid_set.contains(gid)
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
                                // file exists on disk — restore its true terminal state
                                // instead of erroring it as a broken partial.
                                if !resumable {
                                    let full_file = std::path::Path::new(&dir).join(&filename);
                                    if !filename.is_empty() && full_file.exists() {
                                        crate::logln!("[aria2] Orphan {} ({}) has complete file on disk — healing to complete", gid, filename);
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
                                        };
                                        if crate::db::upsert_download(app_handle.clone(), healed).is_ok() {
                                            orphan_persisted.push(gid.to_string());
                                        }
                                        continue;
                                    }
                                }

                                if resumable {
                                    crate::logln!("[aria2] Re-adding paused orphan {} ({}) — resumable partial found", gid, filename);
                                    let client = match aria2::Aria2Client::new() {
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
                                    // On failure keep the row paused under the original gid so the
                                    // next tick retries the re-add (aria2 may not be up yet); never
                                    // flip a resumable download to error just because this add failed.
                                    let effective_gid = add_result.unwrap_or_else(|_| gid.to_string());
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
                                    };
                                    let _ = crate::db::upsert_download(app_handle.clone(), record);
                                    if effective_gid != gid {
                                        // Defensive: only if aria2 ignored the requested gid, drop
                                        // the old row so the UI never shows two cards.
                                        let _ = crate::db::remove_download_from_db(app_handle.clone(), gid.to_string());
                                    }
                                } else {
                                    crate::logln!("[aria2] Orphaned {} ({}) not resumable — marking error", gid, filename);
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

                    // Skipping the stopped snapshot also means an empty result
                    // is just "nothing new" rather than a signal to clear the
                    // UI while completed downloads are temporarily unpolled.
                    if !include_stopped && result.is_empty() {
                        continue;
                    }

                    // Emit to desktop UI — filter cancelled GIDs
                    let mut ui_result: Vec<&Value> = result.iter()
                        .filter(|t| {
                            let gid = t.get("gid").and_then(|v| v.as_str()).unwrap_or("");
                            !cancelled_gids.contains(&gid.to_string())
                        })
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
                                ui_result.push(item);
                            }
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
                                                    println!("[aria2] Auto-retry with reqwest: {} → {}", url, fname);
                                                    let app_clone = app_handle.clone();
                                                    let url_owned = url.to_string();
                                                    let retry_headers = crate::aria2::lookup_download_headers(&url_owned);
                                                    std::thread::spawn(move || {
                                                        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for reqwest retry");
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            aria2::start_aria2_with_settings,
            aria2::get_aria2_version,
            aria2::add_download,
            aria2::add_torrent,
            aria2::parse_torrent_files,
            ytdlp::scan_yt_formats,
            ytdlp::cancel_yt_scan,
            ytdlp::cancel_yt_download,
            ytdlp::resume_yt_download,
            ytdlp::add_yt_download,
            ytdlp::check_yt_dlp_exists,
            ytdlp::get_yt_dlp_path,
            ytdlp::open_app_bin_folder,
            ytdlp::open_yt_dlp_location,
            ytdlp::install_yt_dlp,
            ytdlp::cancel_yt_dlp_install,
            ytdlp::uninstall_yt_dlp,
            ytdlp::check_ejs_exists,
            ytdlp::get_ejs_path,
            ytdlp::open_ejs_location,
            ytdlp::install_ejs,
            ytdlp::cancel_ejs_install,
            ytdlp::uninstall_ejs,
            js_runtime::get_js_runtimes,
            js_runtime::select_js_runtime,
            js_runtime::install_js_runtime,
            js_runtime::cancel_js_runtime_install,
            js_runtime::remove_js_runtime,
            merger::get_mergers,
            merger::select_merger,
            merger::install_merger,
            merger::cancel_merger_install,
            merger::remove_merger,
            fs_ops::open_external,
            update::get_app_version,
            update::check_for_update,
            update::install_update,
            aria2::get_all_downloads,
            aria2::get_download_details,
            aria2::pause_download,
            aria2::resume_download,
            aria2::pause_all_downloads,
            aria2::retry_download,
            aria2::remove_download,
            fs_ops::show_download_in_folder,
            fs_ops::open_download_file,
            aria2::set_proxy,
            aria2::apply_global_speed_limit,
            aria2::apply_download_speed_limit,
            aria2::set_download_proxy,
            aria2::get_pending_downloads,
            aria2::remove_pending_download,
            aria2::confirm_download_action,
            aria2::check_aria2_proxy,
            aria2::test_proxy,
            aria2::test_proxy_url,
            aria2::detect_system_proxy,
            aria2::reqwest_download,
            extension::set_extension_settings,
            extension::open_chrome_extensions,
            db::save_all_settings,
            db::get_all_settings,
            db::get_download_preferences,
            db::save_download_preferences,
            db::get_download_categories,
            db::save_download_categories,
            db::get_download_by_gid,
            db::upsert_download,
            db::remove_download_from_db,
            db::get_all_downloads_from_db,
            db::check_download_file_states,
            db::create_schedule,
            db::execute_schedule,
            db::get_due_schedules,
            fs_ops::get_system_stats,
            fs_ops::open_cache_folder,
            db::open_app_db_folder,
            db::list_app_db_folder,
            fs_ops::set_hardware_acceleration,
            hash::compute_file_hash,
            clipboard::start_clipboard_listener,
            clipboard::read_clipboard_text,
            state::get_default_download_dir,
            protocol::is_default_protocol_client,
            protocol::set_default_protocol_client,
            protocol::remove_as_default_protocol_client,
            logfile::set_debug_logging,
            logfile::append_log_line,
            logfile::get_log_file_path,
            logfile::open_log_folder,
            hide_to_tray,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
