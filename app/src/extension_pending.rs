//! `checkPendingDownloads` from `MainLayout.vue`: the browser extension queues
//! downloads in the backend; the app opens its download dialog prefilled with
//! the queued URL and drops that queue entry once the dialog closes — accepted
//! or not. The Rust port reuses the New Download modal (`show_new_download`)
//! as that dialog, and carries the extension's `headers` through to
//! `add_download` (cookies/UA of the source site).

use crate::screens::Ctx;
use crate::state::UiState;

/// `pendingCheckInterval = setInterval(checkPendingDownloads, 5000)`.
const CHECK_EVERY: f64 = 5.0;

pub fn tick(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let t = ctx.input(|i| i.time);

    // After `remove_pending_download` the list needs a moment to settle before
    // the next fetch, or the same entry comes back.
    if state.pending_refetch_at > 0.0 && t >= state.pending_refetch_at {
        state.pending_refetch_at = 0.0;
        state.pending_dirty = true;
    }

    // The dialog owns the flow until it closes: `checkPendingDownloads` bails
    // out while `isDialogOpen()`.
    if state.nd_pending {
        if !state.show_new_download {
            // `await invoke('remove_pending_download', { index: 0 })` runs on
            // the accept path *and* the cancel path.
            state.nd_pending = false;
            state.nd_headers = None;
            if !state.pendings.is_empty() {
                state.pendings.remove(0);
            }
            state.pending_refetch_at = t + 0.6;
            cctx.run(move |app| async move {
                moonhunt_core::aria2::remove_pending_download(0, app).await?;
                Ok::<String, String>(String::new())
            });
        }
        return;
    }
    if state.show_new_download {
        return;
    }

    // A queued request opens the dialog (`.openDialog(url, filename, '', …)`).
    if let Some(p) = state.pendings.first().cloned() {
        state.add_url = p.url;
        state.nd_filename = p.filename;
        // Extension cookies/UA follow the dialog to `add_download`.
        state.nd_headers = p.headers;
        state.nd_pasted = false;
        state.nd_autofill.clear();
        state.nd_pending = true;
        state.show_new_download = true;
        state.nd_focus_next = true;
        return;
    }

    // Event-driven (`pending-download`) or the 5s poll.
    if state.pending_dirty || t - state.pending_last_check > CHECK_EVERY {
        state.pending_dirty = false;
        state.pending_last_check = t;
        cctx.run(move |app| async move {
            let list = moonhunt_core::aria2::get_pending_downloads(app.clone())
                .await
                .unwrap_or_default();
            let payload: Vec<serde_json::Value> = list
                .iter()
                .enumerate()
                .map(|(idx, p)| serde_json::json!({"idx": idx, "url": p.url, "filename": p.filename, "headers": p.headers}))
                .collect();
            app.emit("ui-pending", &payload);
            Ok::<String, String>(String::new())
        });
    }
}
