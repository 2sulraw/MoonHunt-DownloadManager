//! `useDownloads.scheduleFileStateCheck`: completed rows get a per-file badge
//! (`present` / `moved` / `removed`) by re-statting them every 30s — removed
//! rows only every 5 minutes, since a deleted file rarely comes back. Work is
//! rotated 50 rows at a time so a huge history never blocks on one batch.

use crate::screens::Ctx;
use crate::state::{Status, UiState};

const CHECK_EVERY: f64 = 30.0;
const RECHECK_REMOVED: f64 = 300.0;
const BATCH_MAX: usize = 50;

pub fn tick(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    // Parity harness: `MOONHUNT_FAKE_FILE_STATE` seeds badges for captures;
    // a real check would immediately overwrite them with `removed`.
    if std::env::var("MOONHUNT_FAKE_FILE_STATE").is_ok() {
        return;
    }
    if state.fs_in_flight {
        return;
    }
    // The reference schedules the first check from a populated refresh; here
    // the tick runs every frame, so don't burn the 30s window on an empty
    // list (the first aria2/DB snapshot lands a beat after boot).
    if state.downloads.is_empty() {
        return;
    }
    let now = ctx.input(|i| i.time);
    if now - state.fs_last_check < CHECK_EVERY {
        return;
    }
    // The reference stamps the batch time before filtering, so an empty
    // eligible set still throttles the next attempt by 30s.
    state.fs_last_check = now;

    let eligible: Vec<String> = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Complete)
        .filter(|d| {
            let last = state
                .fs_checked_at
                .get(&d.gid)
                .copied()
                .unwrap_or(f64::NEG_INFINITY);
            let gap = if d.file_state == "removed" {
                RECHECK_REMOVED
            } else {
                CHECK_EVERY
            };
            now - last >= gap
        })
        .map(|d| d.gid.clone())
        .collect();
    if eligible.is_empty() {
        return;
    }

    // Rotate through the list 50 rows at a time.
    let n = eligible.len();
    let take = n.min(BATCH_MAX);
    let gids: Vec<String> = (0..take)
        .map(|i| eligible[(state.fs_cursor + i) % n].clone())
        .collect();
    state.fs_cursor = (state.fs_cursor + take) % n;

    state.fs_in_flight = true;
    let at = now;
    let core = cctx.core.clone();
    let inbox = cctx.inbox.clone();
    cctx.rt.spawn(async move {
        // Disk stat + sqlite writes: keep them off the backend's async workers.
        let rows = tokio::task::spawn_blocking(move || {
            moonhunt_core::db::check_download_file_states(core, gids)
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default();
        let rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "gid": r.get("gid").and_then(|v| v.as_str()).unwrap_or(""),
                    "fileState": r.get("fileState").and_then(|v| v.as_str()).unwrap_or(""),
                    "path": r.get("path").and_then(|v| v.as_str()).unwrap_or(""),
                })
            })
            .collect();
        let payload = serde_json::json!({ "at": at, "rows": rows });
        if let Ok(mut q) = inbox.lock() {
            q.push_back(("ui-file-states".to_owned(), payload));
        }
    });
}
