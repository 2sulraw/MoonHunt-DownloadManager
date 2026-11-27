#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! MoonHunt â€” egui frontend.
//!
//! Boot order mirrors the old Tauri `setup()`: DB â†’ settings mirror â†’
//! extension bridge â†’ aria2 poller (all inside `backend::start_backend`).
//! Backend threads emit `(name, json)` events onto a channel; a bridge thread
//! forwards them into a shared inbox that `update()` drains into `UiState`.

mod download_dialogs;
mod events;
mod extension_pending;
mod file_state;
mod group_dialogs;
mod groups;
mod i18n;
mod modals;
mod overlays;
mod rtl;
mod screens;
mod state;
mod title_bar;
mod ui;

use eframe::egui;
use egui::pos2;
use state::{ToastQueue, UiState};
use state::{Screen, SettingsCat};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Backend â†’ UI event inbox: (event name, JSON payload).
pub type Inbox = Arc<Mutex<VecDeque<(String, serde_json::Value)>>>;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("MoonHunt")
        // Old app window geometry (tauri.conf.json).
        .with_inner_size([1067.0, 678.0])
        .with_min_inner_size([740.0, 480.0])
        // Custom MoonKit frame: title bar + resize strips are drawn by us.
        .with_decorations(false);
    // The taskbar button only follows the *window* icon here (the exe's .ico
    // alone renders as Windows' generic placeholder), so set it explicitly.
    if let Some(icon) = ui::icons::logo_icon() {
        viewport = viewport.with_icon(icon);
    }
    let native_opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "MoonHunt",
        native_opts,
        Box::new(|cc| {
            // MoonKit fonts + default (dark, coral) look before the first frame.
            ui::widgets::set_mode(ui::tokens::Mode::Dark);
            ui::widgets::set_accent_preset("coral");
            ui::theme::apply(&cc.egui_ctx, ui::tokens::Mode::Dark);
            Ok(Box::new(App::new()))
        }),
    )
}

pub struct App {
    core: Option<moonhunt_core::AppHandle>,
    rt: Arc<tokio::runtime::Runtime>,
    inbox: Inbox,
    toasts: ToastQueue,
    ui: UiState,
    error: Option<String>,
    /// Theme mode currently applied to egui (re-applied when settings change).
    applied_theme: ui::tokens::Mode,
    /// Accent preset (`settings.theme`) currently applied.
    applied_accent: String,
    /// Parity harness override (`MOONHUNT_THEME`), applied but never persisted.
    theme_force: Option<ui::tokens::Mode>,
    /// Captured on the first frame so async results can wake the loop.
    egui_ctx: egui::Context,
}

/// Windows keeps a minimized window's pages resident even though egui paints
/// nothing. Give them back to the OS on the way down (they fault back in on
/// restore).
#[cfg(windows)]
fn trim_working_set() {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetProcessWorkingSetSize(h: isize, min: usize, max: usize) -> i32;
    }
    // -1/-1 == "trim as much as you like", the same call Task Manager makes.
    unsafe {
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

#[cfg(not(windows))]
fn trim_working_set() {}

impl App {
    fn new() -> Self {
        let mut s = Self {
            core: None,
            rt: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    // The UI thread only awaits short backend calls; 2 workers
                    // beat parking one per core.
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("backend runtime"),
            ),
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            toasts: Arc::new(Mutex::new(VecDeque::new())),
            ui: UiState::default(),
            error: None,
            applied_theme: ui::tokens::Mode::Dark,
            applied_accent: "coral".to_string(),
            // Parity harness: `MOONHUNT_THEME=light|linear-dark|dark` pins the
            // rendered mode without touching the persisted settings.
            theme_force: std::env::var("MOONHUNT_THEME")
                .ok()
                .map(|v| ui::tokens::Mode::parse(&v)),
            egui_ctx: egui::Context::default(),
        };
        // Parity harness: force the initial screen / settings section from the
        // environment so screenshot captures of both apps can be reproduced
        // deterministically (MOONHUNT_SCREEN=Home|Downloads|Media|Addon|Settings,
        // MOONHUNT_SETTINGS_SECTION=general|aria2c|url|torrent|network|
        // extension|appearance|performance).
        if let Ok(v) = std::env::var("MOONHUNT_SCREEN") {
            s.ui.screen = match v.as_str() {
                "Home" => Screen::Home,
                "Downloads" => Screen::Downloads,
                "Media" => Screen::Media,
                "Addon" => Screen::Addon,
                "Settings" => Screen::Settings,
                _ => s.ui.screen,
            };
        }
        if let Ok(v) = std::env::var("MOONHUNT_SETTINGS_SECTION") {
            s.ui.settings_cat = match v.as_str() {
                "aria2c" => SettingsCat::Aria2c,
                "url" => SettingsCat::Url,
                "torrent" => SettingsCat::Torrent,
                "network" => SettingsCat::Network,
                "extension" => SettingsCat::Extension,
                "appearance" => SettingsCat::Appearance,
                "performance" => SettingsCat::Performance,
                _ => SettingsCat::General,
            };
        }
        if let Ok(v) = std::env::var("MOONHUNT_OPEN_MODALS") {
            if v == "nd" {
                s.ui.show_new_download = true;
                s.ui.nd_focus_next = false;
            }
            if v == "dla" {
                // Download Limit alert, step 1 (before "I Understand" / step 2).
                // Falls back to a "limit just hit" state when the DB settings
                // have no limit configured yet (fresh capture profile).
                s.ui.dla_used = s.ui.settings.download_limit_used_bytes;
                s.ui.dla_limit = s.ui.settings.download_limit_bytes;
                if s.ui.dla_limit == 0 {
                    s.ui.dla_limit = 1024 * 1024;
                    s.ui.dla_used = s.ui.dla_limit;
                }
                s.ui.dla_step = 0;
                s.ui.dla_open = true;
            }
            if v == "batch" {
                // Batch URL dialog (`.batch-nd-modal`), empty default state.
                s.ui.show_batch_dialog = true;
            }
            if v == "err" {
                // Error-details dialog with a representative multi-line body.
                s.ui.err_open = true;
                s.ui.err_filename = "ubuntu-24.04-desktop-amd64.iso".to_owned();
                s.ui.err_message =
                    "HTTP 403 Forbidden\n\naria2 reported: The requested resource is not available.\nserver: cloudflare"
                        .to_owned();
            }
            if v == "hash" {
                // Hash-verify dialog seeded with a finished digest.
                s.ui.hash_open = true;
                s.ui.hash_path = "C:\\Downloads\\ubuntu-24.04-desktop-amd64.iso".to_owned();
                s.ui.hash_name = "ubuntu-24.04-desktop-amd64.iso".to_owned();
                s.ui.hash_computed =
                    "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90".to_owned();
                s.ui.hash_expected =
                    "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90".to_owned();
            }
            if v == "group-create" {
                // Create-group dialog, empty form (Options collapsed).
                s.ui.g_create_open = true;
            }
            if v == "group-edit" || v == "group-dl" {
                // Edit-group dialog seeded from `MOONHUNT_FAKE_GROUPS`, with the
                // Options section open and both limits configured.
                s.ui.g_edit = Some("g-1760000000000-fake1".to_owned());
                s.ui.g_edit_name = "Season 3".to_owned();
                s.ui.g_edit_color = 5; // #42a5f5
                s.ui.g_edit_on_complete = 1; // Close app
                s.ui.g_edit_speed = 500;
                s.ui.g_edit_dl_on = true;
                s.ui.g_edit_dl_value = 2;
                s.ui.g_edit_dl_unit = 2; // GB
                s.ui.g_edit_dl_used = 1_500_000_000;
                s.ui.g_edit_options = true;
                s.ui.g_edit_members = vec![
                    "0123456789abcdef".to_owned(),
                    "fedcba9876543210".to_owned(),
                    "pending-1".to_owned(),
                ];
                s.ui.g_speed_text = "500".to_owned();
                s.ui.g_dl_value_text = "2".to_owned();
                if v == "group-dl" {
                    // The Downloads sub-modal, opened by the edit dialog.
                    s.ui.g_dl_open = true;
                    s.ui.g_dl_for_edit = true;
                    s.ui.g_pending = vec![crate::state::GroupPendingDl {
                        seq: 1,
                        url: "https://example.com/season-3/episode-04.mkv".to_owned(),
                        filename: String::new(),
                    }];
                    s.ui.g_pending_seq = 1;
                }
            }
            if v == "about" || v == "aboutup" || v == "aboutcheck" {
                // AboutModal parity harness: idle / up-to-date / checking.
                s.ui.show_about = true;
                if v == "aboutup" {
                    s.ui.about_check = crate::state::AboutCheck::UpToDate;
                }
                if v == "aboutcheck" {
                    s.ui.about_check = crate::state::AboutCheck::Checking;
                }
            }
            if v == "update" || v == "updprog" || v == "upderr" {
                // Update modal parity harness (no live release in dev).
                s.ui.update_info = Some(crate::state::UpdateInfo {
                    version: "v0.0.7".to_owned(),
                    url: "https://github.com/2sulraw/MoonHunt-DownloadManager/releases/tag/v0.0.7"
                        .to_owned(),
                    asset_url: "https://example.com/moonhunt_0.0.7_x64-setup.exe".to_owned(),
                    body: "## What's new\n\n- Faster aria2 reconnect and resume\n- Addon picker remembers the last tab\n- Download-limit alert handles multi-file torrents\n\n## Fixes\n\n- Clipboard toast no longer stacks on the same link\n- Window drag region stops at the titlebar controls\n- Group progress uses weighted byte totals".to_owned(),
                });
                s.ui.show_update_modal = true;
                if v == "updprog" {
                    s.ui.upd_installing = true;
                    s.ui.upd_progress = Some(crate::state::UpdateProgress {
                        downloaded: 5_400_000,
                        total: 12_000_000,
                        percent: 45.0,
                    });
                }
                if v == "upderr" {
                    s.ui.upd_error =
                        "Update download error: 404 Not Found".to_owned();
                }
            }
        }
        // Parity harness: `MOONHUNT_FAKE_ADDON=1` seeds one enabled addon so the
        // dialog's `matchedAddon` hint ("Handled by …") can be exercised without
        // installing a real one; `MOONHUNT_ND_URL` prefills the URL field.
        if std::env::var("MOONHUNT_FAKE_ADDON").is_ok() {
            s.ui.addon.addons.push(crate::screens::addon::AddonRow {
                id: "a-fake".to_owned(),
                name: "YouTube Downloader".to_owned(),
                version: "1.0.0".to_owned(),
                enabled: true,
                patterns: vec!["youtube.com".to_owned()],
                ..Default::default()
            });
        }
        if let Ok(u) = std::env::var("MOONHUNT_ND_URL") {
            s.ui.add_url = u;
        }
        // Parity harness: deterministic card rows for screenshots that need a
        // populated Downloads page (right-click menu, error chip, ...).
        if std::env::var("MOONHUNT_FAKE_DOWNLOADS").is_ok() {
            let row = |gid: &str, name: &str, st: crate::state::Status, done: u64, total: u64| {
                crate::state::DownloadVm {
                    gid: gid.to_owned(),
                    filename: name.to_owned(),
                    path: format!("C:\\Downloads\\{name}"),
                    url: format!("https://example.com/{name}"),
                    status: st,
                    total,
                    completed: done,
                    size_text: String::new(),
                    added_at: 0,
                    speed: 4_200_000,
                    connections: Some(8),
                    remaining_seconds: Some(42),
                    error_message: String::new(),
                    error_code: String::new(),
                    is_yt: false,
                    progress: (done as f64 / total.max(1) as f64 * 100.0) as f32,
                    file_state: String::new(),
                }
            };
            s.ui.downloads = vec![
                row("0123456789abcdef", "ubuntu-24.04-desktop-amd64.iso", crate::state::Status::Active, 3_100_000_000, 6_200_000_000),
                row("fedcba9876543210", "moonhunt-setup.exe", crate::state::Status::Paused, 12_000_000, 48_000_000),
                row("aaaabbbbccccdddd", "holiday-video.mp4", crate::state::Status::Complete, 720_000_000, 720_000_000),
                row("ddddccccbbbbaaaa", "archive-backup.zip", crate::state::Status::Error, 0, 90_000_000),
            ];
            // `MOONHUNT_FAKE_DOWNLOADS=24` pads the list so long-scroll chrome
            // (sticky toolbar past 160px, scroll-to-top past 400px) can be
            // exercised without a live aria2 queue.
            let extra = std::env::var("MOONHUNT_FAKE_DOWNLOADS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            for i in 4..extra {
                let gid = format!("{:016x}", 0x1000_0000_u64 + i as u64);
                let name = format!("sample-file-{i}.bin");
                s.ui.downloads.push(row(
                    &gid,
                    &name,
                    crate::state::Status::Complete,
                    1_000_000,
                    1_000_000,
                ));
            }
            s.ui.downloads[3].error_message =
                "HTTP 403 Forbidden\n\nThe server refused the request.".to_owned();
            // `MOONHUNT_GRAB=1` — grab mode on at boot (drag-reorder captures
            // without hunting the 28px toolbar toggle).
            if std::env::var("MOONHUNT_GRAB").is_ok() {
                s.ui.grab_enabled = true;
                eprintln!("[harness] grab_enabled=true");
            }
            // `MOONHUNT_DRAG_TEST="gid>target"` — run the manual reorder once at
            // boot so the swap can be checked without synthesising a pointer.
            if let Ok(v) = std::env::var("MOONHUNT_DRAG_TEST") {
                if let Some((g, t)) = v.split_once('>') {
                    s.ui.manual_move(g, t);
                    eprintln!("[harness] manual_move {g} -> {t}");
                }
            }
            // `MOONHUNT_FAKE_FILE_STATE=1` seeds both badges: the completed
            // row is "moved", the first padded row is "removed". `file_state`
            // skips the real check so captures stay deterministic.
            if std::env::var("MOONHUNT_FAKE_FILE_STATE").is_ok() {
                s.ui.downloads[2].file_state = "moved".to_owned();
                if let Some(d) = s.ui.downloads.get_mut(4) {
                    d.file_state = "removed".to_owned();
                }
            }
            // `MOONHUNT_OPEN_MODALS=cm|cmgroup` — the right-click menu staged on
            // the first row (no pointer needed); `cmgroup` opens the submenu.
            if let Ok(v) = std::env::var("MOONHUNT_OPEN_MODALS") {
                if v == "cm" || v == "cmgroup" {
                    s.ui.cm_open = true;
                    s.ui.cm_gid = s.ui.downloads.first().map(|d| d.gid.clone());
                    s.ui.cm_pos = egui::pos2(430.0, 210.0);
                    s.ui.cm_group_menu = v == "cmgroup";
                }
            }
        }
        s
    }

    /// Boots the backend exactly once, on the first frame.
    fn ensure_backend(&mut self, ctx: &egui::Context) {
        if self.core.is_some() || self.error.is_some() {
            return;
        }

        let (app, mut rx) = moonhunt_core::AppHandle::new();

        if let Err(e) = moonhunt_core::db::init_db(&app) {
            self.error = Some(format!("Database init failed: {e}"));
            return;
        }
        // `useGroups` reads `localStorage` at module init; the SQLite-backed
        // equivalent can only be read once the global connection exists.
        self.ui.groups_reload();
        // Parity harness: `MOONHUNT_FAKE_GROUPS=1` stages a container over the
        // fake downloads (in memory only — the stored `moonhunt_groups` blob is
        // left untouched).
        if std::env::var("MOONHUNT_FAKE_GROUPS").is_ok() {
            self.ui.groups = vec![crate::groups::DownloadGroup {
                id: "g-1760000000000-fake1".to_owned(),
                name: "Season 3".to_owned(),
                color: "#42a5f5".to_owned(),
                member_gids: vec![
                    "0123456789abcdef".to_owned(),
                    "fedcba9876543210".to_owned(),
                    "aaaabbbbccccdddd".to_owned(),
                ],
                on_complete: "none".to_owned(),
                collapsed: false,
                speed_limit: 0,
                download_limit_enabled: false,
                download_limit_bytes: 0,
                download_limit_used_bytes: 0,
                added_at: 1_762_000_000_000,
            }];
        }
        if let Err(e) = moonhunt_core::backend::start_backend(&app) {
            self.error = Some(format!("Backend startup failed: {e}"));
            return;
        }
        // Boot the aria2 daemon (previously triggered by the web UI on mount).
        let a2 = app.clone();
        self.rt.spawn(async move {
            match moonhunt_core::aria2::start_aria2(a2).await {
                Ok(msg) => println!("[aria2] {msg}"),
                Err(e) => eprintln!("[aria2] startup failed: {e}"),
            }
        });
        // `startClipboardListener()` — polls the clipboard and emits
        // `clipboard-url-detected`; the UI stacks ClipboardToast rows.
        let cb = app.clone();
        self.rt.spawn(async move {
            if let Err(e) = moonhunt_core::clipboard::start_clipboard_listener(cb).await {
                eprintln!("[clipboard] listener failed: {e}");
            }
        });
        self.core = Some(app);
        self.ui.core_ready = true;
        events::load_initial_settings(&mut self.ui);
        // Parity harness: `MOONHUNT_FAKE_PENDING=1` queues one
        // browser-extension request so the confirm-dialog handoff can be
        // captured without the extension server (port 6802 is taken by the
        // real app).
        if std::env::var("MOONHUNT_FAKE_PENDING").is_ok() {
            if let Some(core) = &self.core {
                core.state_arc().pending_downloads.lock().unwrap().push(
                    moonhunt_core::state::PendingDownload {
                        url: "https://example.com/releases/moonhunt-0.0.7-x64-setup.exe"
                            .to_owned(),
                        filename: "moonhunt-0.0.7-x64-setup.exe".to_owned(),
                        headers: None,
                    },
                );
            }
            self.ui.pending_dirty = true;
        }

        // `MainLayout` one-shot launch check: a newer release only lights the
        // titlebar badge (unlike the About check, it does not open the modal).
        let chk = self.core.clone().expect("core initialized");
        let chk_inbox = self.inbox.clone();
        self.rt.spawn(async move {
            let payload = match moonhunt_core::update::check_for_update(chk).await {
                Ok(s) => serde_json::json!({ "ok": true, "data": s }),
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            if let Ok(mut q) = chk_inbox.lock() {
                q.push_back(("update-checked".to_string(), payload));
            }
        });

        // Bridge the async event channel into the egui loop.
        let inbox = self.inbox.clone();
        let ui_ctx = ctx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("event bridge runtime");
            rt.block_on(async move {
                while let Some((event, payload)) = rx.recv().await {
                    if event == moonhunt_core::EV_WAKE {
                        ui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    if let Ok(mut q) = inbox.lock() {
                        q.push_back((event, payload));
                    }
                    ui_ctx.request_repaint();
                }
            });
        });
    }

    fn ctx(&self) -> screens::Ctx {
        screens::Ctx {
            core: self.core.clone().expect("core initialized"),
            rt: self.rt.clone(),
            toasts: self.toasts.clone(),
            inbox: self.inbox.clone(),
            egui_ctx: self.egui_ctx.clone(),
        }
    }

    fn drain_events(&mut self) {
        let events: Vec<(String, serde_json::Value)> = match self.inbox.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        for (name, payload) in events {
            events::apply(&mut self.ui, &name, &payload);
        }
        // Action toasts (completed backend calls).
        let toast_batch: Vec<(bool, String)> = match self.toasts.lock() {
            Ok(mut q) => q.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        for (ok, text) in toast_batch {
            self.ui.toast(ok, text);
        }
    }
}

/// ponytail: temporary frame probe — `MOONHUNT_PERF=1` logs pass/frame cost so
/// the list slowness can be measured instead of guessed at. Delete with the
/// `perf_probe(ctx)` call once the list is fast.
fn perf_probe() -> Option<std::time::Instant> {
    std::env::var_os("MOONHUNT_PERF")
        .is_some()
        .then(std::time::Instant::now)
}

/// Logs how long one `ui()` pass took, plus the wall time since the previous
/// pass: `dur` is our CPU work, `dt` is the frame interval the user feels.
fn perf_probe_end(ctx: &egui::Context, t0: Option<std::time::Instant>) {
    use std::sync::{Mutex, OnceLock};
    static LAST: OnceLock<Mutex<Option<std::time::Instant>>> = OnceLock::new();
    let Some(t0) = t0 else { return };
    let now = std::time::Instant::now();
    let dt = LAST
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut l| l.replace(now))
        .map(|prev| now.duration_since(prev).as_secs_f64() * 1000.0);
    eprintln!(
        "[perf] pass={} frame={} dur={:.2}ms dt={}",
        ctx.cumulative_pass_nr(),
        ctx.cumulative_frame_nr(),
        t0.elapsed().as_secs_f64() * 1000.0,
        dt.map(|d| format!("{d:.1}")).unwrap_or_else(|| "-".into()),
    );
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        ui::widgets::pal().bg_surface.to_normalized_gamma_f32()
    }

    /// eframe calls `logic` and skips `ui` entirely while the window is hidden.
    /// Nothing is painted then, so the working set is dead weight — give it
    /// back; the pages fault in again on restore.
    fn logic(&mut self, _ctx: &egui::Context, frame: &mut eframe::Frame) {
        let minimized = frame
            .winit_window()
            .and_then(|w| w.is_minimized())
            .unwrap_or(false);
        if minimized {
            trim_working_set();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 0.35: App::ui receives the root Ui; panels show inside it. The
        // backend/bridge still wants a Context, so clone it once per frame.
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let perf_t0 = perf_probe();

        if self.egui_ctx != *ctx {
            self.egui_ctx = ctx.clone();
        }
        self.ensure_backend(ctx);
        self.sync_theme(ctx);
        self.drain_events();
        self.ui.prune_toasts();
        self.ui.prune_clip_toasts();

        // OS close request (Alt+F4 / taskbar close): the reference intercepts
        // it and shows `CloseDialog` instead of quitting.
        if ctx.input(|i| i.viewport().close_requested()) {
            self.ui.show_close_dialog = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // Old layout: full-height 72px sidebar on the left (starts at the very
        // top of the window; same color as the titlebar makes it look merged),
        // with the 50px titlebar only spanning the content area (MainLayout
        // renders TitleBar inside .main-content, right of the rail).
        self.rail(ui);

        // Termination requested from the custom close button.
        if self.ui.request_exit {
            // `quit_app` in the reference: aria2c is spawned detached, so a
            // plain exit leaves it running (it then locks aria2c.exe and
            // breaks in-app updates/uninstall).
            moonhunt_core::aria2::kill_aria2c();
            std::process::exit(0);
        }

        title_bar::show(&mut self.ui, ui);

        // Build the backend ctx before borrowing ui mutably inside the panel.
        let cctx = self.ctx();
        // `.main-content`/`.content-area`: bg-surface with 18/20/24 padding.
        // Settings and Addon opt out: their sidebar is `position: fixed`
        // flush against the rail below the titlebar, and the padding is
        // re-applied by the screen itself around its scroll box (ported in
        // `screens::settings` / `screens::addon`), so the panel must expose
        // the bare area for those screens.
        let settings_screen = matches!(self.ui.screen, Screen::Settings | Screen::Addon);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ui::widgets::pal().bg_surface)
                    .inner_margin(if settings_screen {
                        egui::Margin {
                            left: 0,
                            right: 20,
                            top: 0,
                            bottom: 0,
                        }
                    } else {
                        egui::Margin {
                            left: 20,
                            right: 20,
                            top: 18,
                            bottom: 24,
                        }
                    }),
            )
            .show(ui, |ui| {
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
                return;
            }
            screens::show(&mut self.ui, &cctx, ui);
        });

        draw_toasts(ctx, &self.ui.toasts, &self.ui.settings.language);
        modals::show(&mut self.ui, ctx, &cctx);
        self.speed_popover(ctx);

        // Keep repainting while downloads pump so bars/speeds update; toasts
        // drain a progress bar over 3s, so they need ~30fps frames too.
        let pumping = self
            .ui
            .downloads
            .iter()
            .any(|d| d.status == state::Status::Active);
        ctx.request_repaint_after(std::time::Duration::from_millis(
            if !self.ui.toasts.is_empty() {
                33
            } else if pumping {
                500
            } else {
                1500
            },
        ));
        // `egui_taffy` measures each node from the previous frame and asks for a
        // re-pass; the details-modal chain (root → body → panel leaf) needs a
        // couple of passes to settle (egui_taffy README `max_passes = 2`+).
        ctx.options_mut(|o| o.max_passes = 4.try_into().unwrap());
        perf_probe_end(ctx, perf_t0);
    }
}

impl App {
    /// `openSpeedPopover` — seed the stepper from the stored limit and pin the
    /// popover's bottom-left corner 8px above the click (`.sl-popover`).
    fn open_speed_popover(&mut self, pos: egui::Pos2) {
        let kb = self.ui.settings.max_download_speed;
        self.ui.speed_saved_kb = kb;
        let (unit, value) = if kb >= 1024 * 1024 {
            (2u8, kb / (1024 * 1024))
        } else if kb >= 1024 {
            (1, kb / 1024)
        } else {
            (0, if kb > 0 { kb } else { 200 })
        };
        self.ui.speed_unit = unit;
        self.ui.speed_input = value.to_string();
        self.ui.speed_pop_pos = pos;
        self.ui.speed_pop_open = true;
    }

    /// `closeSpeedPopover` — an empty or zero input falls back to the saved
    /// value instead of leaving the field blank.
    fn close_speed_popover(&mut self) {
        let ok = self
            .ui
            .speed_input
            .trim()
            .parse::<f64>()
            .is_ok_and(|v| v > 0.0);
        if !ok {
            self.ui.speed_input = self.ui.speed_saved_kb.max(200).to_string();
        }
        self.ui.speed_pop_open = false;
    }

    /// `setGlobalSpeedLimit` — persist, push to aria2, then re-apply the
    /// per-group caps: a new global cap clears every per-download limit, so
    /// group caps have to go back on afterwards (`reapplyAllLimits`).
    fn apply_speed_limit(&mut self, enabled: bool, kb: u32) {
        self.ui.settings.max_download_speed = kb;
        self.ui.settings.global_speed_limit_enabled = Some(enabled);
        self.ui.settings_dirty = true;
        let settings = self.ui.settings.clone();
        let group_caps: Vec<(String, u64)> = self
            .ui
            .groups
            .iter()
            .filter(|g| g.speed_limit > 0)
            .flat_map(|g| {
                g.member_gids
                    .iter()
                    .map(move |gid| (gid.clone(), g.speed_limit * 1024))
            })
            .collect();
        if let Some(core) = self.core.clone() {
            let rt = self.rt.clone();
            rt.spawn(async move {
                let _ = moonhunt_core::db::save_all_settings(settings.clone(), core.clone()).await;
                let _ = moonhunt_core::aria2::apply_global_speed_limit(settings).await;
                for (gid, bytes) in group_caps {
                    let _ = moonhunt_core::aria2::apply_download_speed_limit(gid, bytes).await;
                }
            });
        }
    }

    /// `.sl-popover` — 12r card at the cursor: header (title + toggle) and,
    /// while the limit is on, the stepper plus the KB/MB/GB switch.
    fn speed_popover(&mut self, ctx: &egui::Context) {
        if !self.ui.speed_pop_open {
            return;
        }
        let p = ui::widgets::pal();
        let screen = ctx.content_rect();
        let closed = egui::Area::new(egui::Id::new("speed-pop-overlay"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.allocate_exact_size(screen.size(), egui::Sense::click())
                    .1
                    .clicked()
            })
            .inner;
        if closed {
            self.close_speed_popover();
            return;
        }

        // `globalSpeedLimitEnabled` in the composable: the toggle only reads as
        // on when a positive cap is stored, matching the rail's label.
        let mut en = self.ui.settings.global_speed_limit_enabled.unwrap_or(false)
            && self.ui.settings.max_download_speed > 0;
        let mut input = self.ui.speed_input.clone();
        let mut unit = self.ui.speed_unit;
        let title = self.ui.t("Speed Limit");
        let mut action: Option<SpeedAction> = None;

        let pos = egui::pos2(self.ui.speed_pop_pos.x, self.ui.speed_pop_pos.y - 8.0);
        egui::Area::new(egui::Id::new("speed-pop"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(p.bg_card)
                    .stroke(egui::Stroke::new(1.0, p.border))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(16, 14))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(title).size(13.0).strong().color(p.text),
                            );
                            ui.add_space(12.0);
                            if draw_switch(ui, en).clicked() {
                                action = Some(SpeedAction::Toggle);
                            }
                        });
                        if en {
                            ui.horizontal(|ui| {
                                let step = if unit == 0 { 100.0 } else { 1.0 };
                                if stepper_button(ui, "remove").clicked() {
                                    action = Some(SpeedAction::Step(-step));
                                }
                                let field = egui::TextEdit::singleline(&mut input)
                                    .desired_width(54.0)
                                    .horizontal_align(egui::Align::Center)
                                    .margin(egui::Margin::symmetric(4, 9));
                                let resp = ui.add(field);
                                if resp.changed() {
                                    action = Some(SpeedAction::Input);
                                }
                                if stepper_button(ui, "add").clicked() {
                                    action = Some(SpeedAction::Step(step));
                                }
                                for (i, name) in ["KB", "MB", "GB"].iter().enumerate() {
                                    if unit_button(ui, name, unit as usize == i).clicked() {
                                        action = Some(SpeedAction::Unit(i as u8));
                                    }
                                }
                            });
                        }
                    });
            });

        // Act after the closure so `self` is free again.
        let value = input.trim().parse::<f64>().unwrap_or(0.0);
        let mult = [1.0, 1024.0, 1024.0 * 1024.0][unit as usize];
        let to_kb = |v: f64, m: f64| (v * m).round().max(1.0) as u32;
        match action {
            Some(SpeedAction::Toggle) => {
                self.ui.speed_input = input;
                self.ui.speed_unit = unit;
                if en {
                    let kb = self.ui.settings.max_download_speed;
                    self.apply_speed_limit(false, kb);
                } else {
                    // Enabling applies whatever the stepper shows (`kb || 200`).
                    self.apply_speed_limit(true, to_kb(value.max(0.01), mult).max(200));
                }
            }
            Some(SpeedAction::Step(d)) => {
                let next = (value + d).max(0.01);
                self.ui.speed_input = fmt_num(next);
                self.ui.speed_unit = unit;
                self.apply_speed_limit(en, to_kb(next, mult));
            }
            Some(SpeedAction::Unit(u)) => {
                let m2 = [1.0, 1024.0, 1024.0 * 1024.0][u as usize];
                self.ui.speed_unit = u;
                self.ui.speed_input = fmt_num((value * mult / m2 * 100.0).round() / 100.0);
            }
            Some(SpeedAction::Input) => {
                self.ui.speed_input = input;
                self.ui.speed_unit = unit;
                if value > 0.0 {
                    self.apply_speed_limit(en, to_kb(value, mult));
                }
            }
            None => {
                self.ui.speed_input = input;
                self.ui.speed_unit = unit;
            }
        }
    }

    /// The ~25 MB of system CJK fonts are only worth holding while Japanese or
    /// Chinese text is on screen: a ja/zh locale (whole UI) or the Settings
    /// language cards. Leaving Settings gives the memory back.
    fn sync_fonts(&self, ctx: &egui::Context) {
        if matches!(self.ui.settings.language.as_str(), "ja" | "zh")
            || self.ui.screen == Screen::Settings
        {
            ui::theme::install_cjk_fonts(ctx);
        } else {
            ui::theme::release_cjk_fonts(ctx);
        }
    }

    /// Re-apply MoonKit when the user flips the theme mode in settings.
    fn sync_theme(&mut self, ctx: &egui::Context) {
        self.sync_fonts(ctx);
        let want = match self.theme_force {
            Some(m) => m,
            None => ui::tokens::Mode::parse(&self.ui.settings.theme_mode),
        };
        let accent = self.ui.settings.theme.clone();
        if want != self.applied_theme || accent != self.applied_accent {
            ui::widgets::set_mode(want);
            ui::widgets::set_accent_preset(&accent);
            ui::theme::apply(ctx, want);
            self.applied_theme = want;
            self.applied_accent = accent;
        }
    }

    /// Sidebar rail â€” pixel port of `SideBar.vue`: 72px bg-card column with
    /// border-right, `nightlight_round` 32px logo, vertical menu items
    /// (24px icon over 11px label), About item in nav, speed-limit toggle +
    /// version pinned to the bottom.
    #[allow(clippy::too_many_lines)]
    fn rail(&mut self, ui: &mut egui::Ui) {
        let p = ui::widgets::pal();
        // 0.35: SidePanel was removed; the unified Panel replaces it and
        // shows inside the root Ui instead of the Context.
        let rail_resp = egui::Panel::left("rail")
            // resizable panels draw their own resize line on top of the CSS
            // `border-right` below, doubling its alpha.
            .resizable(false)
            .show_separator_line(false)
            .exact_size(ui::theme::RAIL_W)
            .frame(
                egui::Frame::new()
                    .fill(p.bg_card)
                    // `.sidebar { padding: 16px 0 }` — the ref renders glyphs
                    // ~0.5px lower than our centered painting, so give the top
                    // margin 17 and take it back from the bottom to keep the
                    // block ending at the same y as the old 16/16.
                    .inner_margin(egui::Margin {
                        left: 0,
                        right: 0,
                        top: 17,
                        bottom: 15,
                    }),
            )
            .show(ui, |ui| {
                // `.sidebar-nav { gap: 0 }` — items must not get egui's 8px
                // default spacing or the rail drifts ~6px per item.
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                let width = ui.available_width();

                // `.sidebar-logo`: nightlight_round 32px accent-strong,
                // margin-bottom 32px.
                let (lrect, _) = ui.allocate_exact_size(egui::vec2(width, 32.0), egui::Sense::hover());
                ui::icons::paint_centered(ui.painter(), lrect.center(), "nightlight_round", 32.0, p.accent_strong);
                ui.add_space(32.0); // `.sidebar-logo` margin-bottom

                // `.menu-item`s: column, 24px icon + 11px label, padding 12 0.
                let items: [(state::Screen, &str, &str); 5] = [
                    (state::Screen::Home, "home", "Home"),
                    (state::Screen::Downloads, "cloud_download", "Downloads"),
                    (state::Screen::Media, "movie", "Media"),
                    (state::Screen::Addon, "extension", "Addon"),
                    (state::Screen::Settings, "settings", "Settings"),
                ];
                for (screen, icon, label) in items {
                    // Dispatch on the Screen, not the label: the label is
                    // translated («خانه», «رسانه»…), so matching on it sent every
                    // non-English locale to the fallback arm (Settings).
                    if self
                        .sidebar_menu_item(
                            ui,
                            width,
                            self.ui.screen == screen,
                            icon,
                            self.ui.t(label),
                            11.0,
                            47.5,
                        )
                        .clicked()
                    {
                        self.ui.screen = screen;
                    }
                }
                // About opens the modal (footer â„¹ in old app is in main nav).
                if self
                    .sidebar_menu_item(ui, width, false, "info", self.ui.t("About"), 11.0, 47.5)
                    .clicked()
                {
                    self.ui.show_about = true;
                }

                // Flex spacer pushes speed toggle + version to the bottom: the
                // remaining block is the 67px `.speed-limit-toggle` row plus a
                // ~20px version line (3px slack keeps the version's baseline
                // 8px below the toggle, matching `.sidebar-version`).
                ui.allocate_exact_size(
                    egui::vec2(width, (ui.available_height() - 91.0).max(0.0)),
                    egui::Sense::hover(),
                );

                // `.speed-limit-toggle`: click toggles the global limit, right
                // click opens `.sl-popover` (toggle + stepper + unit switch).
                let en = self.ui.settings.global_speed_limit_enabled.unwrap_or(false)
                    && self.ui.settings.max_download_speed > 0;
                let speed_active = self.sidebar_menu_item(
                    ui,
                    width,
                    en,
                    "speed",
                    &speed_limit_label(&self.ui.settings, &self.ui.settings.language),
                    9.0,
                    42.5,
                );
                if speed_active.secondary_clicked() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        self.open_speed_popover(pos);
                    }
                } else if speed_active.clicked() {
                    // `toggleGlobalSpeedLimit`: `nextKb = kb || 200`.
                    let kb = self.ui.settings.max_download_speed;
                    self.apply_speed_limit(!en, kb.max(200));
                }
                // `.speed-limit-toggle` title.
                let tip = if en {
                    self.ui.t("Speed limit is on")
                } else {
                    self.ui.t("Speed limit is off")
                };
                speed_active.on_hover_text(tip);

                // `.sidebar-version` — centered in the rail, its 8px
                // `padding-top` below the speed toggle row.
                let (vrect, _) =
                    ui.allocate_exact_size(egui::vec2(width, 20.0), egui::Sense::hover());
                ui.painter().text(
                    pos2(vrect.center().x, vrect.min.y + 9.5),
                    egui::Align2::CENTER_CENTER,
                    format!("v{}", moonhunt_core::APP_VERSION),
                    egui::FontId::proportional(10.0),
                    p.text_dim,
                );
            });
        let rr = rail_resp.response.rect;
        ui.painter()
            .vline(rr.max.x - 0.5, rr.y_range(), egui::Stroke::new(1.0, p.border));
    }

    /// One `.menu-item` (vertical icon-over-label row, full rail width).
    /// `label_size`/`label_dy` differ for the speed toggle: `.speed-limit-value`
    /// is a 9px/600 line with `margin-top: 3px`, so its ink sits ~5px higher
    /// inside the same 67px row than `.menu-label` (11px) does.
    fn sidebar_menu_item(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        active: bool,
        icon: &str,
        label: &str,
        label_size: f32,
        label_dy: f32,
    ) -> egui::Response {
        let p = ui::widgets::pal();
        // Item pitch: ref nav items repeat every 67px (padding 12+12 around a
        // 24px icon line and a ~19px label line) with `gap: 0` between them.
        let h = 67.0;
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::click());
        // hover = accent 8%; active = accent-strong text (no fill in old CSS).
        if resp.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(0),
                p.accent.gamma_multiply(0.08),
            );
        }
        let color = if active || resp.hovered() { p.accent_strong } else { p.text_dim };
        ui::icons::paint_centered(
            ui.painter(),
            pos2(rect.center().x, rect.min.y + 12.0 + 12.0),
            icon,
            24.0,
            color,
        );
        ui.painter().text(
            pos2(rect.center().x, rect.min.y + label_dy),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(label_size),
            color,
        );
        // Click handling is the caller's job — it knows the `Screen`; the label
        // here is translated and can't be matched against English names.
        resp
    }
}

/// `speedLimitText` port: Unlimited / KB/s / MB/s / GB/s formatting.
fn speed_limit_label(settings: &moonhunt_core::state::ProxySettings, lang: &str) -> String {
    let on = settings.global_speed_limit_enabled.unwrap_or(false);
    let kb = settings.max_download_speed as f64;
    if !on || kb <= 0.0 {
        return crate::i18n::t(lang, "Unlimited").into();
    }
    if kb >= 1_000_000.0 {
        let gb = kb / 1_000_000.0;
        return format!("{} GB/s", if gb == gb.floor() { format!("{gb}") } else { format!("{gb:.2}") });
    }
    if kb >= 1000.0 {
        let mb = kb / 1000.0;
        return format!("{} MB/s", if mb == mb.floor() { format!("{mb}") } else { format!("{mb:.2}") });
    }
    format!("{} KB/s", kb as u32)
}

/// What the speed popover asked for this frame; applied after the UI closure
/// so the borrow on `self.ui` is released.
enum SpeedAction {
    Toggle,
    Step(f64),
    Unit(u8),
    Input,
}

/// Drop a trailing `.00` so the stepper reads `200`, not `200.00`.
fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 0.005 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// `.speed-toggle` — 42×24 track, 18px knob that slides 18px when on.
fn draw_switch(ui: &mut egui::Ui, on: bool) -> egui::Response {
    let p = ui::widgets::pal();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::click());
    let track = if on {
        ui::widgets::blend(p.bg_card, p.accent, 0.25)
    } else {
        ui::widgets::blend(p.bg_card, egui::Color32::WHITE, 0.08)
    };
    let edge = if on {
        ui::widgets::blend(p.bg_card, p.accent, 0.5)
    } else {
        p.border
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(12), track);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(12),
        egui::Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let cx = rect.min.x + 12.0 + if on { 18.0 } else { 0.0 };
    ui.painter().circle_filled(
        egui::pos2(cx, rect.center().y),
        9.0,
        if on { p.accent } else { p.text_dim },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// `.speed-stepper-btn` — 36×38 outlined square with a centred 18px icon.
fn stepper_button(ui: &mut egui::Ui, icon: &str) -> egui::Response {
    let p = ui::widgets::pal();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 38.0), egui::Sense::click());
    let bg = if resp.hovered() {
        ui::widgets::blend(p.bg_card, egui::Color32::WHITE, 0.06)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(8), bg);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    ui::icons::paint_centered(ui.painter(), rect.center(), icon, 18.0, p.text);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// `.speed-unit-btn` — 44×38; the active one takes an accent wash.
fn unit_button(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let p = ui::widgets::pal();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(44.0, 38.0), egui::Sense::click());
    if active {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(8),
            ui::widgets::blend(p.bg_card, p.accent, 0.18),
        );
    }
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(8),
        egui::Stroke::new(
            1.0,
            if active {
                ui::widgets::blend(p.bg_card, p.accent, 0.45)
            } else {
                p.border
            },
        ),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        if active { p.text } else { p.text_dim },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Toasts: the old app's `ToastContainer` — a 320px card centered above the
/// bottom bar (container bottom 142px + 16px padding + 8px tail margin), with
/// a 4px progress bar pinned to the card's bottom-left that drains from 100%
/// to 0% linearly over the toast's 3s lifetime.
fn draw_toasts(ctx: &egui::Context, toasts: &[state::Toast], lang: &str) {
    if toasts.is_empty() {
        return;
    }    let p = ui::widgets::pal();
    let font = egui::FontId::new(14.0, egui::FontFamily::default());
    // `.toast { width: 320px; padding: 12px 20px; border: 1px }` → the message
    // wraps at 278px and each card is text height + 26px of padding/border.
    // The card is a flex column and the progress bar is `position: absolute`,
    // so the bar adds no height.
    let items: Vec<(std::sync::Arc<egui::Galley>, f32)> = toasts
        .iter()
        .map(|t| {
            let galley = ctx
                .debug_painter()
                // One place to translate toasts: the backend hands over plain
                // English keys, and `t()` is a no-op for anything that is not one.
                .layout(crate::i18n::t(lang, &t.text).to_owned(), font.clone(), p.text, 278.0);
            let h = galley.rect.height() + 26.0;
            (galley, h)
        })
        .collect();
    egui::Area::new(egui::Id::new("toasts"))
        // Newest toast is pushed last and sits at the bottom of the stack.
        // `.toast-container { bottom: 142px; padding: 16px }` + the last card's
        // `margin-bottom: 8px` → the stack's bottom edge is 166px up.
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -166.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 8.0);
            // Allocate the stack so the Area has a real size: an Area whose
            // content is only painted (never allocated) stays 0×0, which both
            // mis-anchors CENTER_BOTTOM (left edge lands on the center line) and
            // gives the painter a zero-width clip rect.
            let total_h: f32 = items.iter().map(|(_, h)| *h).sum::<f32>()
                + 8.0 * items.len().saturating_sub(1) as f32;
            let (stack, _) =
                ui.allocate_exact_size(egui::vec2(320.0, total_h), egui::Sense::hover());
            let mut y = stack.top();
            for (t, (galley, h)) in toasts.iter().zip(items.iter()) {
                let rect = egui::Rect::from_min_size(
                    pos2(stack.left(), y),
                    egui::vec2(320.0, *h),
                );
                // `box-shadow: var(--shadow-md)` — approximated with a soft
                // dark underlay, since egui paints no shadows.
                ui.painter().rect_filled(
                    rect.translate(egui::vec2(0.0, 2.0)),
                    egui::CornerRadius::same(8),
                    egui::Color32::from_black_alpha(60),
                );
                ui.painter().rect_filled(rect, egui::CornerRadius::same(8), p.bg_card);
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(8),
                    egui::Stroke::new(1.0, p.border_strong),
                    egui::StrokeKind::Inside,
                );
                ui.painter().galley(
                    rect.min + egui::vec2(20.0, 13.0),
                    galley.clone(),
                    p.text,
                );
                // `.toast-progress` — 4px tall, drains left-anchored: the
                // right edge slides in toward the left over 3s linear.
                let frac = (1.0 - t.born.elapsed().as_secs_f32() / 3.0).clamp(0.0, 1.0);
                let bw = (320.0 - 2.0) * frac;
                if bw > 0.0 {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            pos2(rect.min.x + 1.0, rect.max.y - 5.0),
                            egui::vec2(bw, 4.0),
                        ),
                        egui::CornerRadius::same(0),
                        if t.ok { p.accent } else { egui::Color32::from_rgb(0xef, 0x53, 0x50) },
                    );
                }
                y += h + 8.0;
            }
        });
}
