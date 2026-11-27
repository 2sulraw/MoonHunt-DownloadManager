//! Backend event → UiState application.
//!
//! The backend threads emit `(name, json)` pairs; the bridge thread forwards
//! them onto a shared inbox and `update()` drains + applies them here.

use crate::state::{AboutCheck, DownloadVm, UiState, UpdateInfo, UpdateProgress};
use moonhunt_core::state::ProxySettings;
use std::time::{Duration, Instant};

pub fn apply(state: &mut UiState, name: &str, payload: &serde_json::Value) {
    match name {
        // `clipboard-url-detected`: payload is the raw URL string.
        "clipboard-url-detected" => {
            if let Some(url) = payload.as_str() {
                state.clip_toast(url);
            }
        }
        // Settings-pane fetches (emitted through `screens::Ctx::emit`).
        "settings-aria2" => {
            state.aria2_fetching = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let ver = aria2_version_of(payload.get("data"));
                state.aria2_status = Some(Ok(ver.clone().unwrap_or_else(|| "aria2".into())));
                state.aria2_version = ver.map(|v| format!("v{v}")).unwrap_or_default();
            } else {
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("aria2c unreachable")
                    .to_string();
                state.aria2_status = Some(Err(err));
                state.aria2_version = String::new();
            }
        }
        "settings-stats" => {
            state.perf_fetching = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                state.perf_error = None;
                state.perf_stats = payload.get("data").cloned();
            } else {
                state.perf_error = Some(
                    payload
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("stats unavailable")
                        .to_string(),
                );
            }
        }
        "settings-detect" => {
            state.st_detecting = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let d = payload.get("data");
                let enabled = d.and_then(|v| v.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);
                let host = d.and_then(|v| v.get("host")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let port = d.and_then(|v| v.get("port")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let msg = d
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                state.st_detect = Some((enabled, msg));
                // The reference fills the form from a live system proxy.
                if enabled && !host.is_empty() && port > 0 {
                    state.settings.proxy_host = host;
                    state.settings.proxy_port = port;
                    state.settings.proxy_type = "http".into();
                    state.settings_dirty = true;
                }
            } else {
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                // Head is a table key; the detail stays raw. The label paints it
                // through `t_of`, which orders the whole sentence once.
                let head = state.raw("Detection failed");
                state.st_detect = Some((false, format!("{head}: {err}")));
            }
        }
        "settings-test-yt" => {
            state.st_testing_yt = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let d = payload.get("data");
                let ok = d
                    .and_then(|v| v.get("ok"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let ms = d
                    .and_then(|v| v.get("latencyMs"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let status = d
                    .and_then(|v| v.get("status"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let msg = if ok {
                    format!("YouTube reachable through proxy ({status} · {ms}ms)")
                } else {
                    let detail = d
                        .and_then(|v| v.get("error"))
                        .and_then(|v| v.as_str())
                        .or_else(|| d.and_then(|v| v.get("message")).and_then(|v| v.as_str()))
                        .unwrap_or("no response");
                    format!("YouTube blocked: {detail}")
                };
                state.toast(ok, msg.clone());
                state.st_test_yt = Some((ok, msg));
            } else {
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                state.toast(false, err.clone());
                state.st_test_yt = Some((false, err));
            }
        }
        "settings-test" => {
            state.st_testing = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let d = payload.get("data");
                let ok = d.and_then(|v| v.get("ok")).and_then(|v| v.as_bool()).unwrap_or(false);
                let mut msg = d
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Connection failed")
                    .to_string();
                if ok {
                    if let Some(ip) = d.and_then(|v| v.get("ip")).and_then(|v| v.as_str()) {
                        msg = format!("{msg} · IP: {ip}");
                    }
                    // Old `notify('Proxy connected (Nms)')` alongside the row.
                    let ms = d
                        .and_then(|v| v.get("latencyMs"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let ok_msg = state.t_of(&format!(
                        "{} ({ms}ms)",
                        state.raw("Proxy connected")
                    ));
                    state.toast(true, ok_msg);
                } else {
                    state.toast(false, msg.clone());
                }
                state.st_test = Some((ok, msg));
            } else {
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                state.toast(false, format!("Proxy test failed: {err}"));
                state.st_test = Some((false, err));
            }
        }
        "settings-db" => {
            state.db_fetching = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let data = payload.get("data");
                state.db_folder = data
                    .and_then(|d| d.get("folder"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                state.db_files = data
                    .and_then(|d| d.get("files"))
                    .and_then(|f| f.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|f| Some((f.get("name")?.as_str()?.to_string(), f.get("size")?.as_u64()?)))
                            .collect()
                    })
                    .unwrap_or_default();
            } else {
                // Old `notify('Failed to list DB folder: …')`.
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                state.toast(false, format!("Failed to list DB folder: {err}"));
            }
        }
        // ── Download details modal (`.details-modal-new`) fetches ────────────
        // `get_download_details(gid)` — aria2 tellStatus with a DB fallback for
        // completed/failed rows aria2 no longer remembers.
        "dm-details" => {
            state.dm_loading = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                state.dm_data = payload.get("data").cloned();
            } else {
                state.dm_data = None;
            }
        }
        // `get_download_events(gid)` — the `.dm-log-list` rows, loaded lazily
        // the first time the Log tab is opened.
        "dm-events" => {
            state.dm_events_loading = false;
            state.dm_events = if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                payload
                    .get("data")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        }
        // `get_yt_download_health(gid)` — `{gid, alive}` for the diagnostics
        // health ring on `yt-dlp-direct-*` rows.
        "dm-yt-health" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                state.dm_yt_health = payload
                    .get("data")
                    .and_then(|d| d.get("alive"))
                    .and_then(|v| v.as_bool());
            } else {
                state.dm_yt_health = None;
            }
        }
        // `get_download_by_gid(gid)` — the persisted row. The aria2 snapshot has
        // no `proxy`, so the Network tab's ProxyField is seeded from here (the
        // old app read `Download.proxy` off its own model).
        "dm-db" => {
            let row = match payload.get("data") {
                Some(v) if !v.is_null() => Some(v.clone()),
                _ => None,
            };
            // `detailProxy.value = parseProxyString(download.proxy)` on every open
            // — absent/empty proxy resets the form to `emptyProxyForm()`.
            let proxy = row
                .as_ref()
                .and_then(|r| r.get("proxy"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let (enabled, typ, host, port, user, pass) = crate::modals::dm_parse_proxy(proxy);
            state.dm_proxy_enabled = enabled;
            state.dm_proxy_type = typ;
            state.dm_proxy_host = host;
            state.dm_proxy_port = port;
            state.dm_proxy_user = user;
            state.dm_proxy_pass = pass;
            state.dm_db_row = row;
        }
        // `set_download_proxy(gid, …)` — Network tab Apply result line
        // (`.dm-proxy-message.ok` / error). The toggle-off auto-apply passes
        // `silent = true` and must stay quiet.
        "dm-proxy-apply" => {
            state.dm_proxy_applying = false;
            let silent = std::mem::take(&mut state.dm_proxy_silent);
            state.dm_proxy_msg = if silent {
                None
            } else if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                // The old handler ignores the backend string on success.
                Some((true, state.t("Settings saved successfully!").to_owned()))
            } else {
                // `` `${t('Failed to save settings')}: ${e}` ``.
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or(state.t("Failed to save settings"));
                let head = state.raw("Failed to save settings");
                Some((false, format!("{head}: {err}")))
            };
        }
        // `detect_system_proxy()` — ProxyField's local detect button result.
        // A hit seeds the form (enabled + host + port) like the Vue handler.
        "dm-detect" => {
            state.dm_detecting = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let d = payload.get("data");
                let enabled = d
                    .and_then(|x| x.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let host = d
                    .and_then(|x| x.get("host"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let port = d
                    .and_then(|x| x.get("port"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let msg = d
                    .and_then(|x| x.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if enabled && !host.is_empty() && port > 0 {
                    state.dm_proxy_enabled = true;
                    state.dm_proxy_type = "http".to_owned();
                    state.dm_proxy_host = host;
                    state.dm_proxy_port = port.to_string();
                }
                state.dm_detect_msg = if msg.is_empty() { None } else { Some((enabled, msg)) };
            } else {
                state.dm_detect_msg = Some((
                    false,
                    payload
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Detection failed")
                        .to_string(),
                ));
            }
        }

        // ── Update flow (`UpdateModal.vue` + `MainLayout`'s launch check) ────
        // Launch `check_for_update` result: only the titlebar badge.
        // About-modal check result: same info, then `onUpdateAvailable` closes
        // the About dialog and opens the update modal.
        "update-checked" | "update-available" => {
            let from_about = name == "update-available";
            let d = payload.get("data").unwrap_or(payload);
            let status = d.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "available" {
                // `AboutModal.vue` renders these inline (`.ab-update-msg`); the
                // launch check only lights the titlebar badge.
                if from_about {
                    state.about_check = match status {
                        "upToDate" => AboutCheck::UpToDate,
                        "noReleases" => AboutCheck::NoReleases,
                        _ => AboutCheck::Error,
                    };
                    state.about_check_error = d
                        .get("message")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .or_else(|| {
                            payload
                                .get("error")
                                .and_then(|v| v.as_str())
                                .map(str::to_string)
                        })
                        .unwrap_or_default();
                }
                return;
            }
            let text = |k: &str| {
                d.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };
            state.update_available = true;
            state.update_info = Some(UpdateInfo {
                version: text("version"),
                url: text("url"),
                asset_url: text("assetUrl"),
                body: text("body"),
            });
            if name == "update-available" {
                state.show_about = false;
                state.show_update_modal = true;
            }
        }
        // `update-download-progress` from `install_update`'s byte stream.
        "update-download-progress" => {
            let downloaded = payload.get("downloaded").and_then(|v| v.as_u64()).unwrap_or(0);
            let total = payload.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
            let percent = payload.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            state.upd_progress = Some(UpdateProgress {
                downloaded,
                total,
                percent,
            });
        }
        // `install_update` settled. Success parks the button for 2.5s (the
        // reference's `setTimeout`); failure surfaces `.um-error`.
        "update-install-result" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                state.upd_progress = Some(UpdateProgress {
                    downloaded: 0,
                    total: 0,
                    percent: 100.0,
                });
                state.upd_reset_at = Some(Instant::now() + Duration::from_millis(2500));
            } else {
                state.upd_installing = false;
                state.upd_progress = None;
                let e = payload.get("error").and_then(|v| v.as_str()).unwrap_or("");
                state.upd_error = if e.is_empty() {
                    state.t("Could not install update").to_string()
                } else {
                    e.to_string()
                };
            }
        }

        // BatchUrlDialog submit: close + summary toast, or keep the dialog
        // open with the first backend failure in `.nd-category-error`.
        "batch-add-done" => {
            state.batch_submitting = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                let n = payload
                    .get("data")
                    .and_then(|d| d.get("count"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                state.toast(
                    true,
                    format!("Added {n} download{}", if n == 1 { "" } else { "s" }),
                );
                state.show_batch_dialog = false;
                crate::download_dialogs::batch_reset(state);
            } else {
                state.batch_error = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Failed to add downloads")
                    .to_owned();
            }
        }

        // HashVerifyDialog `compute_file_hash` result.
        "hash-computed" => {
            state.hash_busy = false;
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                state.hash_computed = payload
                    .get("data")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                state.hash_error.clear();
            } else {
                state.hash_computed.clear();
                state.hash_error = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Failed to compute hash")
                    .to_owned();
            }
        }

        // Full aria2 snapshot (active + waiting + stopped). Replace the list;
        // sort afterwards. Keep rows alive across polls by gid replacement.
        "downloads-changed" => {
            let Some(items) = payload.as_array() else { return };
            // Parity harness: keep the seeded rows even while the backend
            // polls an empty aria2 list (and now also emits DB history).
            if std::env::var("MOONHUNT_FAKE_DOWNLOADS").is_ok() {
                return;
            }
            let parsed: Vec<DownloadVm> = items.iter().map(DownloadVm::from_value).flatten().collect();
            // `useDownloads` merges each poll over the previous row, so the
            // `fileState` badge and the resolved `path` survive snapshots that
            // don't carry them (aria2 items never do). Indexed by gid: a linear
            // scan per item made every poll O(n²) on a long list.
            let prev_by_gid: std::collections::HashMap<&str, &DownloadVm> =
                state.downloads.iter().map(|p| (p.gid.as_str(), p)).collect();
            let mut merged: Vec<DownloadVm> = Vec::with_capacity(parsed.len());
            for mut d in parsed {
                if let Some(prev) = prev_by_gid.get(d.gid.as_str()) {
                    if d.file_state.is_empty() {
                        d.file_state = prev.file_state.clone();
                    }
                    if d.path.is_empty() {
                        d.path = prev.path.clone();
                    }
                }
                merged.push(d);
            }
            // Stable-merge: preserve previous ordering where possible so rows
            // don't jump around between polls (aria2 order is stable anyway).
            state.note_first_seen(&merged);
            // Live aria2 items (and anything not yet persisted) carry no
            // `addedAt`, so they'd sort as 0 and drop to the bottom of the
            // "Date Added/Completed" list. Fall back to the first-seen stamp so
            // a newly started download shows at the top where it was added.
            for d in &mut merged {
                if d.added_at == 0 {
                    if let Some(&t) = state.first_seen.get(&d.gid) {
                        d.added_at = t;
                    }
                }
            }
            state.record_speed_samples(&merged);
            state.downloads = merged;
            state.sort_downloads();
            // `useDownloads.processAria2Items` → `seedFromLiveItems` /
            // `trackBytes` / `checkLimit` on every snapshot the poller pushes.
            state.limit_track(items);
        }

        // `check_download_file_states` results (one batch of ≤50 completed
        // rows): stamp the per-gid check time, then apply the badge/path.
        "ui-file-states" => {
            let at = payload.get("at").and_then(|v| v.as_f64()).unwrap_or(0.0);
            state.fs_in_flight = false;
            let Some(rows) = payload.get("rows").and_then(|v| v.as_array()) else { return };
            for r in rows {
                let Some(gid) = r.get("gid").and_then(|v| v.as_str()) else { continue };
                state.fs_checked_at.insert(gid.to_owned(), at);
                let Some(row) = state.downloads.iter_mut().find(|d| d.gid == gid) else { continue };
                if let Some(fs) = r.get("fileState").and_then(|v| v.as_str()) {
                    row.file_state = fs.to_owned();
                }
                if let Some(p) = r.get("path").and_then(|v| v.as_str()) {
                    if !p.is_empty() {
                        row.path = p.to_owned();
                    }
                }
            }
            // `lastFileStateCheckByGid` prunes gids that left the list.
            let downloads = &state.downloads;
            state
                .fs_checked_at
                .retain(|gid, _| downloads.iter().any(|d| &d.gid == gid));
        }

        // Light per-poll diff: connections + ETA (emitted only when it changed).
        "live-stats-changed" => {
            let Some(items) = payload.as_array() else { return };
            for upd in items {
                let Some(gid) = upd.get("gid").and_then(|v| v.as_str()) else { continue };
                let Some(row) = state.downloads.iter_mut().find(|d| d.gid == gid) else { continue };
                row.connections = upd
                    .get("connections")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);
                row.remaining_seconds = upd
                    .get("remainingSeconds")
                    .and_then(|v| v.as_u64());
                if let Some(s) = row.remaining_seconds {
                    if s == 0 {
                        row.remaining_seconds = None;
                    }
                }
            }
        }

        // yt-dlp / backend console chatter (shown in a diagnostics drawer).
        "yt-log" => {
            let line = payload.as_str().unwrap_or("");
            push_log(state, line);
            if line.starts_with("[yt-search]") || line.starts_with("[spotify]") {
                state.spotify.log(line);
            }
        }
        "clipboard-url-detected" => {
            state.toast(true, format!("URL detected: {}", payload.as_str().unwrap_or("")));
        }
        "pending-download" => {
            // `listen('pending-download', () => checkPendingDownloads())` — the
            // queue itself is fetched by `extension_pending::tick`.
            state.pending_dirty = true;
        }
        // Data fetch feedback loop: downloads screen fetched the pending list
        // and re-emitted it as a UI event; parse it back out here.
        "ui-pending" => {
            let list = payload.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        Some(crate::state::PendingView {
                            idx: p.get("idx").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                            url: p.get("url").and_then(|v| v.as_str())?.to_string(),
                            filename: p.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            // Cookies/UA captured by the extension, if any.
                            headers: p.get("headers").and_then(|h| h.as_object()).map(|o| {
                                o.iter()
                                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                    .collect()
                            }),
                        })
                    })
                    .collect()
            });
            if let Some(list) = list {
                state.pendings = list;
            }
        }

        // ProxyUrlCheckBadge result (async `check_aria2_proxy` /
        // `test_proxy_url` round-trip started by the new-download dialog).
        // Stale responses (the URL changed mid-flight) only release the busy
        // flag — mirroring the old `requestSequence` guard.
        "nd-proxy-check" => {
            state.nd_pcheck_busy = false;
            let key = payload.get("key").and_then(|v| v.as_str()).unwrap_or("");
            if key != state.nd_pcheck_key {
                return;
            }
            state.nd_pcheck_text = payload
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            state.nd_pcheck_ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            state.nd_pcheck_direct = payload.get("direct").and_then(|v| v.as_bool()).unwrap_or(false);
        }

        // Auto-detect result (`detect_system_proxy`) for the ProxyField card.
        "nd-proxy-detect" => {
            state.nd_proxy_detecting = false;
            let enabled = payload.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            state.nd_proxy_msg_ok = enabled;
            state.nd_proxy_msg = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let host = payload
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let port = payload.get("port").and_then(|v| v.as_u64()).unwrap_or(0);
            if enabled && !host.is_empty() && port > 0 {
                state.nd_proxy_enabled = true;
                state.nd_proxy_host = host;
                state.nd_proxy_port = port.to_string();
            }
        }

        // Parsed .torrent contents (async `parse_torrent_files`).
        "nd-torrent-files" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let files: Vec<(String, u64)> = payload
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            Some((
                                f.get("path").and_then(|v| v.as_str())?.to_owned(),
                                f.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(t) = state.nd_torrents.iter_mut().find(|t| t.path == path) {
                t.loading = false;
                t.error.clear();
                t.files = files;
                // Old app: every file selected by default.
                t.selected = vec![true; t.files.len()];
            }
        }

        // Torrent parse failure — keep the entry and surface the error line.
        "nd-torrent-error" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let parse_failed = state.t("Failed to read torrent contents");
            if let Some(t) = state.nd_torrents.iter_mut().find(|t| t.path == path) {
                t.loading = false;
                t.files.clear();
                t.selected.clear();
                t.error = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or(parse_failed)
                    .to_owned();
            }
        }

        // add_download / addTorrent settled: drop the footer spinner and, on
        // success, run the old dialog's post-add reset + close.
        // `submitPendingGroupDownloads`: one gid per queued URL joins the
        // group the dialogs just created/updated.
        "grp-pending-add" => {
            let ok = payload.get("ok").and_then(|v| v.as_bool()) == Some(true);
            if ok {
                let d = payload.get("data");
                let group = d
                    .and_then(|v| v.get("group"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let gid = d
                    .and_then(|v| v.get("gid"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !group.is_empty() && !gid.is_empty() {
                    state.group_add_member(&group, &gid);
                }
            } else {
                let err = payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                state.toast(false, format!("Failed to add download: {err}"));
            }
        }
        "nd-add-done" => {
            state.nd_is_adding = false;
            let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(true);
            if ok {
                if payload.get("tab").and_then(|v| v.as_str()) == Some("torrent") {
                    // `addTorrent` only clears the torrent tab's own fields.
                    state.nd_torrents.clear();
                    state.nd_magnet.clear();
                    state.nd_path.clear();
                    state.nd_torrent_tab = false;
                } else {
                    state.nd_reset();
                }
                state.show_new_download = false;
            }
        }

        _ => {
            // Screen-scoped events (yt-dlp stream/installers, addon jobs) are
            // handled by their own modules; only unconsumed ones hit the log.
            if !crate::screens::media::apply_event(state, name, payload)
                && !crate::screens::addon::apply_event(state, name, payload)
                && !crate::screens::spotify::apply_event(state, name, payload)
            {
                push_log(state, &format!("[event] {name}: {payload}"));
            }
        }
    }
}

fn push_log(state: &mut UiState, line: &str) {
    state.log.push_back(line.to_string());
    while state.log.len() > 200 {
        let _ = state.log.pop_front();
    }
}

/// Pull a version string out of the shapes `get_aria2_version` can return:
/// `{version}`, a JSON string of that object, or the RPC envelope `{result}`.
fn aria2_version_of(data: Option<&serde_json::Value>) -> Option<String> {
    fn from_value(v: &serde_json::Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                return from_value(&parsed);
            }
            return Some(s.to_string());
        }
        v.get("version").and_then(|x| x.as_str()).map(str::to_string)
    }
    let d = data?;
    from_value(d).or_else(|| d.get("result").and_then(from_value))
}

/// Initial settings mirror (called once at backend boot from the app side).
pub fn load_initial_settings(state: &mut UiState) {
    state.settings = moonhunt_core::db::load_settings_from_db().unwrap_or_else(|_| ProxySettings::default());
    // Parity harness: `MOONHUNT_LANG` must survive this DB load, so it is applied
    // here rather than at boot.
    if let Ok(l) = std::env::var("MOONHUNT_LANG") {
        state.settings.language = l;
    }
    // Parity harness: `MOONHUNT_PROXY` keeps the UI copy in step with the
    // engine copy that `start_backend` applies.
    moonhunt_core::state::apply_env_proxy(&mut state.settings);
}
