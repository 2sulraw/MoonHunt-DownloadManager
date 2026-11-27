//! UI-side state: view models mirroring `useDownloads.ts` mapping semantics,
//! filters/sort, toasts and the mirrored settings form.

use moonhunt_core::state::ProxySettings;
use std::collections::VecDeque;
use std::time::Instant;

/// A queued user-visible result from a backend action.
pub type ToastQueue = std::sync::Arc<std::sync::Mutex<VecDeque<(bool, String)>>>;

/// One top-level row of the Downloads list: a group container or a lone card.
#[derive(Clone, Debug)]
pub enum DlEntry {
    Group {
        id: String,
        gids: Vec<String>,
        date_added: i64,
    },
    Download {
        gid: String,
    },
}

/// One `pendingGroupDownloads` row of the group Downloads sub-modal: a URL
/// queued from the dialog and only turned into a real gid on confirm.
#[derive(Clone, Debug)]
pub struct GroupPendingDl {
    pub seq: u64,
    pub url: String,
    pub filename: String,
}

impl GroupPendingDl {
    /// The `pending-<n>` placeholder gid the member lists carry.
    pub fn gid(&self) -> String {
        format!("pending-{}", self.seq)
    }
}

/// Download status (aria2's vocabulary, normalized).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Active,
    Waiting,
    Paused,
    Error,
    Complete,
    Removed,
}

impl Status {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Status::Active,
            "waiting" => Status::Waiting,
            "paused" => Status::Paused,
            "error" => Status::Error,
            "complete" => Status::Complete,
            // `normalizeStatus`: the DB stores finished transfers as `stopped`.
            "stopped" => Status::Complete,
            "removed" => Status::Removed,
            _ => Status::Waiting,
        }
    }

    /// Raw `st-*` token — the web search preview prints `{{ item.status }}`
    /// verbatim, so the port shows the same lowercase words.
    pub fn token(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Waiting => "waiting",
            Status::Paused => "paused",
            Status::Error => "error",
            Status::Complete => "complete",
            Status::Removed => "removed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Active => "Active",
            Status::Waiting => "Waiting",
            Status::Paused => "Paused",
            Status::Error => "Error",
            Status::Complete => "Complete",
            Status::Removed => "Removed",
        }
    }

    /// Status color from the MoonKit token palette (`--status-*`).
    pub fn color(self) -> egui::Color32 {
        let p = crate::ui::widgets::pal();
        match self {
            Status::Active => p.status_active,
            Status::Waiting => p.status_waiting,
            Status::Paused => p.status_paused,
            Status::Error => p.status_error,
            Status::Complete => p.status_complete,
            Status::Removed => p.status_cancelled,
        }
    }
}

/// One row of the Downloads screen (mirrors the `Download` type in
/// `useDownloads.ts` — same fields, same derivation rules).
#[derive(Clone, Debug)]
pub struct DownloadVm {
    pub gid: String,
    pub filename: String,
    pub path: String,
    /// Source URL (`files[0].uris[0].uri`), used by title-bar search.
    pub url: String,
    pub status: Status,
    pub total: u64,
    pub completed: u64,
    /// Raw `size` string of a persisted row ("976.6 KB"); empty for live
    /// aria2 rows, which format `completed / total` instead.
    pub size_text: String,
    /// Raw bytes/s (parsed from aria2, unformatted).
    pub speed: u64,
    pub connections: Option<u32>,
    pub remaining_seconds: Option<u64>,
    pub error_message: String,
    pub error_code: String,
    /// Set when the download was handed off by yt-dlp (header token).
    pub is_yt: bool,
    pub progress: f32,
    /// Epoch ms the row was added (`added_at` in the DB); 0 for live aria2
    /// rows, which have no such field.
    pub added_at: i64,
    /// `check_download_file_states` badge: `""` (unknown), `present`,
    /// `moved` or `removed`. Only completed rows ever carry a value.
    pub file_state: String,
}

impl DownloadVm {
    /// Build from a raw aria2 tellStatus/tellActive object — field names match
    /// the emitted `downloads-changed` payload 1:1 (camelCase aria2 keys).
    pub fn from_value(item: &serde_json::Value) -> Option<Self> {
        let gid = item.get("gid")?.as_str()?.to_string();

        // DB history rows (`completedNotInLive`) arrive in the aria2 item
        // shape plus these explicit keys; live aria2 rows never carry them.
        let explicit = |key: &str| -> Option<String> {
            item.get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };

        let total = num_u64(item, "totalLength").max(
            item.get("sizeBytes")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| parse_size_str(&explicit("size").unwrap_or_default())),
        );
        let completed = num_u64(item, "completedLength");
        let speed = num_u64(item, "downloadSpeed");
        let progress = if total > 0 {
            ((completed as f64 / total as f64) * 100.0).min(100.0) as f32
        } else {
            item.get("progress")
                .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                .unwrap_or(0) as f32
        };
        let status_str = item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("waiting");
        let status = Status::parse(status_str);

        let (error_message, error_code) = {
            let code = item.get("errorCode").and_then(|v| v.as_str()).unwrap_or("");
            let msg = item
                .get("errorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if status == Status::Error && (!code.is_empty() || !msg.is_empty()) {
                (
                    if msg.is_empty() {
                        format!("Error code: {code}")
                    } else {
                        msg.to_string()
                    },
                    code.to_string(),
                )
            } else {
                (String::new(), String::new())
            }
        };

        let files = item.get("files").and_then(|v| v.as_array());
        let path = explicit("path")
            .or_else(|| {
                files
                    .and_then(|fs| fs.first())
                    .and_then(|f| f.get("path"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_default();
        let filename = explicit("filename").unwrap_or_else(|| {
            if path.is_empty() {
                item.get("bittorrent")
                    .and_then(|bt| bt.get("info"))
                    .and_then(|i| i.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&gid)
                    .to_string()
            } else {
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&gid)
                    .to_string()
            }
        });

        let url = explicit("url")
            .or_else(|| {
                files
                    .and_then(|fs| fs.first())
                    .and_then(|f| f.get("uris"))
                    .and_then(|us| us.as_array())
                    .and_then(|us| us.first())
                    .and_then(|u| u.get("uri"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_default();

        let is_yt = contains_yt_header(item).unwrap_or(false);

        // Terminal rows pinned at full progress, like the old UI did.
        let progress = if status == Status::Error || status == Status::Complete {
            100.0
        } else {
            progress
        };

        let connections = item
            .get("connections")
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<u32>().ok())
                    .or_else(|| v.as_u64().map(|n| n as u32))
            })
            .or_else(|| item.get("split").and_then(|v| v.as_str().and_then(|s| s.parse().ok())));

        let is_live = matches!(status, Status::Active | Status::Waiting);
        let remaining_seconds = if is_live && speed > 0 && total > completed {
            Some((total - completed) / speed.max(1))
        } else {
            None
        };

        Some(Self {
            gid,
            filename,
            path,
            url,
            status,
            total,
            completed,
            size_text: explicit("size").unwrap_or_default(),
            speed,
            connections,
            remaining_seconds,
            error_message,
            error_code,
            is_yt,
            progress,
            added_at: item
                .get("addedAt")
                .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                .unwrap_or(0),
            file_state: item
                .get("fileState")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    pub fn is_live(&self) -> bool {
        matches!(self.status, Status::Active | Status::Waiting)
    }

    pub fn speed_display(&self) -> String {
        if self.is_live() && self.speed > 0 {
            format!("{}/s", moonhunt_core::util::format_bytes(self.speed))
        } else {
            String::new()
        }
    }

    pub fn eta_display(&self) -> String {
        match (self.is_live(), self.remaining_seconds, self.speed) {
            (true, Some(s), sp) if sp > 0 => format_eta(s),
            _ => String::new(),
        }
    }
}

fn format_eta(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Numeric aria2 fields arrive as decimal strings; accept either form.
fn num_u64(item: &serde_json::Value, key: &str) -> u64 {
    item.get(key)
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| v.as_u64())
        })
        .unwrap_or(0)
}

/// `format_bytes` output back to bytes ("976.6 KB" → 1_000_038). The DB only
/// stores the formatted size string (`size_bytes` is never populated), so a
/// history row's total has to be recovered from it.
fn parse_size_str(s: &str) -> u64 {
    let s = s.trim();
    let (num, unit) = match s.split_once(' ') {
        Some((n, u)) => (n, u),
        None => return 0,
    };
    let Ok(value) = num.parse::<f64>() else { return 0 };
    let mult = match unit {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024f64.powi(2),
        "GB" => 1024f64.powi(3),
        "TB" => 1024f64.powi(4),
        _ => return 0,
    };
    (value * mult).round() as u64
}

/// Detects the `x-moonhunt-ytdlp` marker header on the item or its first file.
fn contains_yt_header(item: &serde_json::Value) -> Option<bool> {
    const MARKER: &str = "x-moonhunt-ytdlp";
    for holder in [
        item.get("requestHeaders"),
        item.get("files")
            .and_then(|fs| fs.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("requestHeaders")),
        item.get("files")
            .and_then(|fs| fs.as_array())
            .and_then(|f| f.first())
            .and_then(|f| f.get("uris"))
            .and_then(|us| us.as_array())
            .and_then(|us| us.first())
            .and_then(|u| u.get("requestHeaders")),
    ] {
        if let Some(v) = holder {
            if let Some(s) = v.as_str() {
                let lower = s.to_lowercase();
                if lower.starts_with(MARKER) || lower.contains(&format!("{}:", MARKER)) {
                    return Some(true);
                }
            } else if let Some(list) = v.as_array() {
                // The core emits `requestHeaders: ["X-Moonhunt-YTDLP:<token>"]`
                // (same shape the old app's aria2 snapshots carried; the old
                // detector accepted array entries and {name,value} objects).
                for entry in list {
                    if let Some(s) = entry.as_str() {
                        if s.to_lowercase().contains(&format!("{}:", MARKER))
                            || s.to_lowercase().starts_with(MARKER)
                        {
                            return Some(true);
                        }
                    } else if let Some(map) = entry.as_object() {
                        if map.get("name").and_then(|n| n.as_str()).map(|n| n.to_lowercase())
                            == Some(MARKER.to_string())
                        {
                            return Some(true);
                        }
                    }
                }
            } else if let Some(map) = v.as_object() {
                for key in map.keys() {
                    if key.to_lowercase() == MARKER {
                        return Some(true);
                    }
                }
            }
        }
    }
    None
}

/// Hero filter-bar of the Downloads page (old app has exactly 3 filters:
/// `all` / `downloading` = active+waiting+paused / `completed` = complete+error,
/// see `filteredDownloads` in `DownloadsPage.vue`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Downloading,
    Completed,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Downloading => "Downloading",
            Filter::Completed => "Completed",
        }
    }
}

/// Sort options of the old sort menu (`sortOptions` in `DownloadsPage.vue`:
/// added / name / size / completed / manual; no speed option in the old UI).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
    Added,
    Name,
    Size,
    Completed,
    Manual,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Added => "Date Added",
            SortKey::Name => "Name",
            SortKey::Size => "Size",
            SortKey::Completed => "Date Completed",
            SortKey::Manual => "Manual",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub ok: bool,
    pub text: String,
    pub born: Instant,
}

/// One "Download link detected" toast (`ClipboardToast.vue`): 10s lifetime.
#[derive(Clone, Debug)]
pub struct ClipToast {
    pub id: u64,
    pub url: String,
    pub born: Instant,
}

/// `UpdateModal.vue`'s `UpdateInfo` prop.
#[derive(Clone, Debug, Default)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub asset_url: String,
    pub body: String,
}

/// Payload of the backend's `update-download-progress` event.
#[derive(Clone, Copy, Debug)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f32,
}

/// `AboutModal.vue`'s `check.status` — the inline update-check row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AboutCheck {
    #[default]
    Idle,
    Checking,
    UpToDate,
    NoReleases,
    Error,
}

/// Everything the UI renders. Owned by `App`, mutated from `update()` only.
pub struct UiState {
    pub screen: Screen,
    pub downloads: Vec<DownloadVm>,
    pub filter: Filter,
    pub search: String,
    /// Keyboard selection in the title-bar search preview (`activeIndex`).
    pub search_active: Option<usize>,
    /// Title-bar search field owns keyboard focus this frame
    /// (`searchFocused`; the preview dropdown is gated on it).
    pub search_focused: bool,
    /// The preview dropdown was open last frame. Enter inside the TextEdit
    /// surrenders focus *before* the dropdown code runs, so the key would
    /// otherwise never reach the selection handlers.
    pub search_open: bool,
    pub sort: SortKey,
    pub sort_ascending: bool,
    pub add_url: String,
    pub settings: ProxySettings,
    pub settings_dirty: bool,
    pub proxy_test_result: Option<String>,
    /// Settings editor category (left nav on the Settings screen).
    pub settings_cat: SettingsCat,
    /// Aria2 engine status snapshot for the aria2c settings pane.
    pub aria2_status: Option<Result<String, String>>,
    /// Cached `aria2c` bin path shown in the aria2c settings pane.
    pub aria2_bin_path: String,
    /// Version chip of the last successful aria2 status poll.
    pub aria2_version: String,
    /// Status poll in flight (keeps the 5s refetch from double-spawning).
    pub aria2_fetching: bool,
    /// Performance pane snapshots (`get_system_stats` / db folder listing).
    pub perf_stats: Option<serde_json::Value>,
    pub perf_error: Option<String>,
    pub perf_fetching: bool,
    pub db_folder: String,
    pub db_files: Vec<(String, u64)>,
    pub db_fetching: bool,
    /// `.speed-unit-switch` selections (0=KB, 1=MB, 2=GB) — session-only,
    /// mirroring the old `useGlobalSpeedLimit` unit refs.
    pub st_speed_unit: usize,
    pub st_upload_unit: usize,
    /// `.dl-limit-unit-custom` selection (0=KB, 1=MB, 2=GB) — session-only.
    pub st_dl_unit: usize,
    /// Settings → Network transient results (cleared when leaving the tab):
    /// `(ok, message)` of the last auto-detect / connection test.
    pub st_detect: Option<(bool, String)>,
    pub st_test: Option<(bool, String)>,
    pub st_detecting: bool,
    pub st_testing: bool,
    /// `Test YouTube`: does the configured proxy actually reach YouTube?
    pub st_test_yt: Option<(bool, String)>,
    pub st_testing_yt: bool,
    /// Extension token field shows the secret in plain text (eye toggle).
    pub st_token_visible: bool,
    /// Performance cache card help note visibility.
    pub st_cache_help: bool,
    /// Settings-pane refresh clocks: aria2 status polls every 5s, perf stats
    /// every 1.5s, the DB listing once (re-armed by the refresh button).
    pub aria2_next_poll: Instant,
    pub perf_next_poll: Instant,
    pub db_next_fetch: Instant,
    pub toasts: Vec<Toast>,
    /// "Download link detected" toasts (`ClipboardToast.vue`, 10s lifetime).
    pub clip_toasts: Vec<ClipToast>,
    pub clip_next_id: u64,
    /// Close confirmation modal (`CloseDialog.vue`) — opened by the title-bar
    /// close button and by OS close requests.
    pub show_close_dialog: bool,
    pub log: VecDeque<String>,
    /// Download requests handed off by the browser extension (unconfirmed).
    pub pendings: Vec<PendingView>,
    pub pending_dirty: bool,
    /// `checkPendingDownloads` throttle (egui frame seconds) and the delay
    /// before re-fetching after `remove_pending_download(0)`.
    pub pending_last_check: f64,
    pub pending_refetch_at: f64,
    /// The open modal was opened *for* a queued extension request, so closing
    /// it must drop that entry (0 = none).
    pub nd_pending: bool,
    /// Headers of the queued extension request that opened the dialog; sent to
    /// `add_download` on submit (cookies of the source site). `None` for a
    /// hand-typed URL.
    pub nd_headers: Option<std::collections::HashMap<String, String>>,
    /// `useDownloads.scheduleFileStateCheck`: last batch time, in-flight flag,
    /// round-robin cursor and the per-gid check timestamps.
    pub fs_last_check: f64,
    pub fs_in_flight: bool,
    pub fs_cursor: usize,
    pub fs_checked_at: std::collections::HashMap<String, f64>,
    /// Backends that completed startup so dormant UI doesn't double-spawn.
    pub core_ready: bool,
    /// About modal (sidebar ℹ item) — ported from `AboutModal.vue`.
    pub show_about: bool,
    /// `speedLimitPopover` in `SideBar.vue` — right-click on the rail's speed
    /// row opens a small popover at the cursor with a toggle, a stepper and a
    /// KB/MB/GB switch.
    pub speed_pop_open: bool,
    /// Where the popover's bottom-left corner is pinned (the click position).
    pub speed_pop_pos: egui::Pos2,
    /// `speedInputValue` — kept as text so a half-typed number survives a frame.
    pub speed_input: String,
    /// `speedUnit`: 0 = KB, 1 = MB, 2 = GB.
    pub speed_unit: u8,
    /// `savedSpeedKb` — restored when the popover closes on an empty input.
    pub speed_saved_kb: u32,
    /// `MainLayout.updateAvailable` — the launch-time `check_for_update` found
    /// a newer release; shows the titlebar badge.
    pub update_available: bool,
    /// `updateInfo` handed to `UpdateModal.vue`.
    pub update_info: Option<UpdateInfo>,
    /// `showUpdateModal`.
    pub show_update_modal: bool,
    /// `installing` — the Install button is disabled while the download runs.
    pub upd_installing: bool,
    /// `progress` from the `update-download-progress` event.
    pub upd_progress: Option<UpdateProgress>,
    /// `installError`.
    pub upd_error: String,
    /// The reference re-enables the button 2.5s after a successful install
    /// (the app usually exits first; a cached relaunch stays alive).
    pub upd_reset_at: Option<Instant>,
    /// `AboutModal.vue`'s inline check state (the dialog's own `check` ref).
    pub about_check: AboutCheck,
    /// `check.error` — shown in `.ab-update-msg.error`.
    pub about_check_error: String,
    /// Screen rect of the title-bar search input (preview dropdown anchor).
    pub search_rect: Option<egui::Rect>,
    /// Preview row picked → scroll to/highlight this gid on the Downloads
    /// screen (ported `preview-select` / `moonhunt:focus-download` event).
    pub focus_gid: Option<String>,
    /// `flash-highlight`: the card rings in the accent colour for 1.6s.
    pub flash_gid: Option<String>,
    pub flash_at: Option<std::time::Instant>,
    /// Window controls want the app to terminate (close-button parity).
    pub request_exit: bool,
    /// Home-screen dialogs (ported next milestone).
    pub show_new_download: bool,
    /// New-download modal state (rework of the old `DownloadsPage` add
    /// dialog): torrent-source tab, magnet, save-as override, save folder
    /// override, connection count, and the focus-request flag for the URL
    /// field's first frame after opening.
    pub nd_torrent_tab: bool,
    pub nd_magnet: String,
    pub nd_filename: String,
    pub nd_path: String,
    pub nd_connections: u32,
    pub nd_focus_next: bool,
    /// Frames left to keep re-requesting the URL field's focus. The pointer-up
    /// of the click that opened the dialog lands a pass after the request and
    /// clears it again, so one request is not enough.
    pub nd_focus_frames: u8,
    /// The URL field was prefilled from the clipboard on open
    /// (`wasPastedFromClipboard`).
    pub nd_pasted: bool,
    /// Last Save As value *we* filled from the URL (`lastAutoFilledFilename`).
    pub nd_autofill: String,
    /// Duplicate-URL override (`allowDuplicate`).
    pub nd_allow_duplicate: bool,
    /// A submit is in flight (footer spinner + disabled buttons).
    pub nd_is_adding: bool,
    /// `.torrent` files queued on the Torrent tab.
    pub nd_torrents: Vec<NdTorrentFile>,
    /// CategoryPicker state (`categoriesEnabled` / `categoryMode` /
    /// `selectedCategory` + the inline `.nd-category-error` row).
    pub nd_cat_enabled: bool,
    /// 0 = formatted, 1 = packaged, 2 = manual.
    pub nd_cat_mode: u8,
    pub nd_cat_selected: String,
    pub nd_categories: Vec<String>,
    pub nd_cat_error: String,
    /// SchedulePicker state (`scheduleEnabled` / `pendingSchedule`) plus the
    /// open ScheduleModal's form (`nd_sched_*`).
    pub nd_sched_enabled: bool,
    pub nd_sched_open: bool,
    /// A schedule has been configured (drives the preview row).
    pub nd_sched_set: bool,
    /// 0 = start, 1 = pause, 2 = resume.
    pub nd_sched_action: u8,
    pub nd_sched_date: String,
    pub nd_sched_time: String,
    /// 0 = once, 1 = hourly, 2 = daily, 3 = weekly.
    pub nd_sched_repeat: u8,
    pub nd_sched_error: String,
    /// ProxyField state (`downloadProxy`).
    pub nd_proxy_enabled: bool,
    pub nd_proxy_host: String,
    pub nd_proxy_port: String,
    pub nd_proxy_user: String,
    pub nd_proxy_pass: String,
    pub nd_proxy_detecting: bool,
    /// Auto-detect result line (`.pf-detect-msg`).
    pub nd_proxy_msg: String,
    pub nd_proxy_msg_ok: bool,
    /// ProxyUrlCheckBadge: host currently being checked / debounce start /
    /// in-flight flag / last result (`None` = still unknown).
    pub nd_pcheck_key: String,
    pub nd_pcheck_at: Instant,
    pub nd_pcheck_done: String,
    pub nd_pcheck_busy: bool,
    pub nd_pcheck_text: Option<String>,
    pub nd_pcheck_ok: bool,
    /// Badge shows the "direct connection" state (no proxy on aria2).
    pub nd_pcheck_direct: bool,
    /// Whether `show_new_download` was up last frame (open-edge detection so
    /// the dialog's `watch(showAddDialog)` open work runs exactly once).
    pub nd_was_open: bool,
    /// CategoryManagerModal (`.cm-*`) state.
    pub nd_catmgr_open: bool,
    pub nd_catmgr_new: String,
    /// `Some(index)` while that category row is being renamed.
    pub nd_catmgr_edit: Option<usize>,
    pub nd_catmgr_edit_value: String,
    /// Due-schedule poller (`MainLayout.checkDueSchedules`).
    pub nd_due_poll: Instant,
    /// Open dropdown popup (`CustomDropdown`) id — cleared when the dialog
    /// reopens so a stale list can't reappear.
    pub nd_dd_open: Option<String>,
    /// `rememberRequestedConnections` — session map of URL → requested
    /// connection count (`requestedConnectionsByUrl` in the old app).
    pub nd_conn_memory: Vec<(String, u32)>,
    pub show_batch_dialog: bool,
    /// ── BatchUrlDialog (`.batch-*`) state ──────────────────────────────────
    pub batch_url_text: String,
    pub batch_dir: String,
    /// Kept as text so the numeric field can hold partial input (`number`).
    pub batch_connections: String,
    pub batch_error: String,
    pub batch_submitting: bool,
    /// ── ErrorDetailsDialog (`.error-*`) state ──────────────────────────────
    pub err_open: bool,
    pub err_filename: String,
    pub err_message: String,
    /// ── HashVerifyDialog (`.hash-*`) state ─────────────────────────────────
    pub hash_open: bool,
    pub hash_path: String,
    pub hash_name: String,
    pub hash_algo: String,
    pub hash_expected: String,
    pub hash_computed: String,
    pub hash_busy: bool,
    pub hash_error: String,
    /// ── GlobalContextMenu (`.context-menu-*`) state ────────────────────────
    pub cm_open: bool,
    pub cm_pos: egui::Pos2,
    pub cm_gid: Option<String>,
    /// The context menu's "Add to Group" submenu is showing.
    pub cm_group_menu: bool,
    /// ── Download details modal (`.details-modal-new`) ───────────────────────
    /// Open flag (`showDetailsModal`).
    pub dm_open: bool,
    /// Gid of the download being inspected (`selectedDownload.gid`). The live
    /// row is re-resolved from `downloads` every frame, mirroring the old
    /// `watch(downloads)` refresh of `selectedDownload`.
    pub dm_gid: Option<String>,
    /// Active tab: 0 = General, 1 = Network, 2 = Log.
    pub dm_tab: u8,
    /// `get_download_details` result (aria2 tellStatus / DB fallback).
    pub dm_data: Option<serde_json::Value>,
    /// True while the details fetch is in flight (`.dm-loading` skeleton).
    pub dm_loading: bool,
    /// `get_download_events` rows (`{id,gid,timestamp,eventType,detail}`),
    /// fetched when the Log tab is first opened.
    pub dm_events: Vec<serde_json::Value>,
    pub dm_events_loading: bool,
    /// yt-dlp child liveness for `yt-dlp-direct-*` gids (diagnostics ring).
    pub dm_yt_health: Option<bool>,
    /// Per-download proxy form on the Network tab (`detailProxy`).
    pub dm_proxy_enabled: bool,
    pub dm_proxy_type: String,
    pub dm_proxy_host: String,
    pub dm_proxy_port: String,
    pub dm_proxy_user: String,
    pub dm_proxy_pass: String,
    /// Apply in flight + the `.dm-proxy-save` result line.
    pub dm_proxy_applying: bool,
    pub dm_proxy_msg: Option<(bool, String)>,
    /// Next `applyDetailProxy` result is the toggle-off auto-apply (`silent`),
    /// which must not surface a `.dm-proxy-message`.
    pub dm_proxy_silent: bool,
    /// `ProxyField`'s local `detecting` / `detectMsg` / `detectOk` state.
    pub dm_detecting: bool,
    pub dm_detect_msg: Option<(bool, String)>,
    /// The persisted row (`get_download_by_gid`) for the open download — the
    /// aria2 snapshot has no `proxy`, so the Network tab's ProxyField is seeded
    /// from this DB row (as the old app did via `Download.proxy`).
    pub dm_db_row: Option<serde_json::Value>,
    /// `show_details_modal` was up last frame (open-edge detection so the
    /// dialog's `showDownloadDetails` setup runs exactly once).
    pub dm_was_open: bool,
    /// Downloads page view mode (`moonhunt_view_mode`: 'list' | 'compact').
    pub view_compact: bool,
    /// Old `sortMenuOpen` — the hero toolbar's sort popup.
    pub sort_menu_open: bool,
    /// Old `grabEnabled` — drag-reorder toggle.
    pub grab_enabled: bool,
    /// Card being dragged (`dragGid`) and the group it came out of.
    pub drag_gid: Option<String>,
    pub drag_from_group: Option<String>,
    /// Group container being dragged by its header.
    pub drag_group: Option<String>,
    /// Old `customOrder` — manual gid slots (session-scoped, like the app).
    pub custom_order: Vec<String>,
    /// ── Groups (ported `useGroups.ts`) ───────────────────────────────────────
    pub groups: Vec<crate::groups::DownloadGroup>,
    /// Create-group dialog (`showCreateGroupDialog`).
    pub g_create_open: bool,
    pub g_create_name: String,
    pub g_create_color: usize,
    /// `showGroupOptions` — the tune toggle reveals the options section.
    pub g_create_options: bool,
    pub g_create_on_complete: usize,
    /// `newGroupSpeedLimit` in KB/s; 0 = off (the toggle writes 200).
    pub g_create_speed: u64,
    pub g_create_dl_on: bool,
    pub g_create_dl_value: u64,
    pub g_create_dl_unit: usize,
    /// `newGroupMembers` — staged member gids (`pending-<seq>` placeholders
    /// resolve through `g_pending`).
    pub g_create_members: Vec<String>,
    /// Edit-group dialog (`renameGroupId` + its fields).
    pub g_edit: Option<String>,
    pub g_edit_name: String,
    pub g_edit_color: usize,
    pub g_edit_options: bool,
    pub g_edit_on_complete: usize,
    pub g_edit_speed: u64,
    pub g_edit_dl_on: bool,
    pub g_edit_dl_value: u64,
    pub g_edit_dl_unit: usize,
    pub g_edit_dl_used: u64,
    /// `editingGroupMembers`.
    pub g_edit_members: Vec<String>,
    /// Gid whose card-level "Group" dropdown is open.
    pub g_menu_gid: Option<String>,
    /// `showGroupDownloads` — the group dialogs' Downloads sub-modal, plus
    /// which dialog opened it (`editingTarget`).
    pub g_dl_open: bool,
    pub g_dl_for_edit: bool,
    /// `groupDownloadUrl` input.
    pub g_dl_url: String,
    /// `pendingGroupDownloads` — queued URLs (stable `seq` so removing a row
    /// can't shift the remaining placeholders).
    pub g_pending: Vec<GroupPendingDl>,
    pub g_pending_seq: u64,
    /// Open on-complete dropdown of either dialog (`g_dd_open`).
    pub g_dd_open: Option<String>,
    /// Edit buffers of the two number inputs (the old `v-model.number`
    /// fields): the parsed value lives in `g_*_speed` / `g_*_dl_value`, the
    /// raw text keeps partial input while the user types.
    pub g_speed_text: String,
    pub g_dl_value_text: String,
    /// Windows-style maximize state — tracked by our custom frame controls.
    pub maximized: bool,
    /// First time (ms epoch) we saw each gid — "Today" chip + Recent Activity
    /// ordering in the old app used `added_at`, which aria2 doesn't provide.
    pub first_seen: std::collections::HashMap<String, i64>,
    /// Rolling per-gid speed samples (bytes/s) for the Home speed-bar visual.
    pub speed_history: std::collections::HashMap<String, std::collections::VecDeque<u64>>,
    /// ── Download-limit tracker (ported `useDownloadLimit.ts`) ───────────────
    /// Per-gid last-seen `completedLength` (delta accounting).
    pub limit_prev: std::collections::HashMap<String, u64>,
    /// Gids that have reported a non-zero `completedLength` — only their
    /// deltas count toward the limit (`verifiedGids`).
    pub limit_verified: std::collections::HashSet<String>,
    /// Snapshots consumed by `seedFromLiveItems` (the first 4 only record
    /// baselines so an already-mid-flight transfer doesn't count retroactively).
    pub limit_seed: u32,
    /// `downloadLimitReached` — latched until the counter is reset.
    pub limit_reached: bool,
    /// A pause-all + alert open is owed; drained by `modals::limit_tick`.
    pub limit_pause_pending: bool,
    /// Persist `downloadLimitUsedBytes` on the next tick (alert's Reset).
    pub limit_persist_pending: bool,
    /// Clock of the next 10s periodic used-bytes persist.
    pub limit_next_persist: Instant,
    /// ── Download-limit alert (ported `DownloadLimitAlert.vue`) ──────────────
    /// Modal open flag.
    pub dla_open: bool,
    /// 0 = acknowledge step, 1 = reset prompt (`step: 'alert' | 'reset'`).
    pub dla_step: u8,
    /// used/limit bytes snapshotted when the alert opened (`open(used, limit)`).
    pub dla_used: u64,
    pub dla_limit: u64,
    /// Media screen (`Media.vue`) — downloader card + helper-card lists.
    pub media: crate::screens::media::MediaUi,
    /// Addon screen (`AddonPage.vue`) — addon list + manager state.
    pub addon: crate::screens::addon::AddonUi,
    pub spotify: crate::screens::spotify::SpotifyUi,
}

/// Wall clock in ms (local UTC epoch — matches JS Date.now() semantics).
pub fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Local midnight (ms epoch), as the old "Today" chip used.
pub fn local_midnight_ms() -> i64 {
    // Rough local-day derivation via the system's offset-free local time read.
    let now = chrono_now_ms();
    let secs = now / 1000;
    // Convert to local struct via libc-free approach: Windows/Unix localtime is
    // unavailable without a tz crate; approximate using UTC + system offset.
    let local_secs = secs + local_utc_offset_secs() as i64;
    (local_secs - (local_secs % 86_400)) * 1000
}

fn local_utc_offset_secs() -> i32 {
    // std has no tz API; compute offset from local day-of-day via PATH trick.
    // Cheap stable approximation: read env TZ or default to system zone via
    // `GetTimeZoneInformation`; for parity purposes UTC fallback is acceptable
    // and the chip is decorative.
    tz_offset_seconds()
}

fn tz_offset_seconds() -> i32 {
    #[cfg(target_os = "windows")]
    {
        // DYNAMIC-TIME-WINDOWS: use PowerShell-free FFI? Keep it simple: rely on
        // the TZ env or fall back to (UTC). The chip only needs day granularity.
        if let Ok(tz) = std::env::var("TZ") {
            if let Some(sign) = tz.rfind(['+', '-']) {
                if let Ok(h) = tz[sign + 1..sign + 3].parse::<i32>() {
                    let val = h * if tz.as_bytes()[sign] == b'+' { 3600 } else { -3600 };
                    return -val;
                }
            }
        }
        0 // lazily default to UTC when TZ is unset
    }
    #[cfg(not(target_os = "windows"))]
    0
}

/// One unconfirmed extension handoff (index into the backend's own list is
/// required for dismissal because removal is positional there).
#[derive(Clone, Debug)]
pub struct PendingView {
    pub idx: usize,
    pub url: String,
    pub filename: String,
    /// Cookies/UA captured by the browser extension for this queued URL.
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// One `.torrent` file queued on the new-download dialog's Torrent tab
/// (port of the old `TorrentFileEntry`: parsed contents, per-file selection
/// and the loading/error states of the preview).
#[derive(Clone, Debug, Default)]
pub struct NdTorrentFile {
    /// Absolute path handed to `add_torrent`.
    pub path: String,
    /// File name shown in the card header.
    pub name: String,
    /// Parsed `(path, size)` entries — empty while loading or after an error.
    pub files: Vec<(String, u64)>,
    /// Per-entry selection, parallel to `files`.
    pub selected: Vec<bool>,
    pub loading: bool,
    pub error: String,
}

impl NdTorrentFile {
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    pub fn all_selected(&self) -> bool {
        !self.files.is_empty() && self.selected.iter().all(|s| *s)
    }

    pub fn selected_bytes(&self) -> u64 {
        self.files
            .iter()
            .zip(self.selected.iter())
            .filter(|(_, sel)| **sel)
            .map(|((_, size), _)| *size)
            .sum()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen { Home, Downloads, Media, Addon, Settings }

/// Settings categories (8 panes of the old settings sidebar).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsCat {
    #[default]
    General,
    Aria2c,
    Url,
    Torrent,
    Network,
    Extension,
    Appearance,
    Performance,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            filter: Filter::All,
            search: String::new(),
            search_active: None,
            search_focused: false,
            search_open: false,
            sort: SortKey::Added,
            sort_ascending: false,
            add_url: String::new(),
            settings: ProxySettings::default(),
            settings_dirty: false,
            proxy_test_result: None,
            settings_cat: SettingsCat::General,
            aria2_status: None,
            aria2_bin_path: String::new(),
            aria2_version: String::new(),
            aria2_fetching: false,
            perf_stats: None,
            perf_error: None,
            perf_fetching: false,
            db_folder: String::new(),
            db_files: Vec::new(),
            db_fetching: false,
            st_speed_unit: 0,
            st_upload_unit: 0,
            st_dl_unit: 1,
            st_detect: None,
            st_test: None,
            st_detecting: false,
            st_testing: false,
            st_test_yt: None,
            st_testing_yt: false,
            st_token_visible: false,
            st_cache_help: false,
            aria2_next_poll: Instant::now(),
            perf_next_poll: Instant::now(),
            db_next_fetch: Instant::now(),
            downloads: Vec::new(),
            toasts: Vec::new(),
            clip_toasts: Vec::new(),
            clip_next_id: 0,
            show_close_dialog: false,
            log: VecDeque::with_capacity(64),
            pendings: Vec::new(),
            pending_dirty: false,
            pending_last_check: 0.0,
            pending_refetch_at: 0.0,
            nd_pending: false,
            nd_headers: None,
            fs_last_check: f64::NEG_INFINITY,
            fs_in_flight: false,
            fs_cursor: 0,
            fs_checked_at: std::collections::HashMap::new(),
            core_ready: false,
            show_about: false,
            speed_pop_open: false,
            speed_pop_pos: egui::Pos2::ZERO,
            speed_input: "200".to_owned(),
            speed_unit: 0,
            speed_saved_kb: 0,
            update_available: false,
            update_info: None,
            show_update_modal: false,
            upd_installing: false,
            upd_progress: None,
            upd_error: String::new(),
            upd_reset_at: None,
            about_check: AboutCheck::Idle,
            about_check_error: String::new(),
            search_rect: None,
            focus_gid: None,
            flash_gid: None,
            flash_at: None,
            request_exit: false,
            show_new_download: false,
            nd_torrent_tab: false,
            nd_magnet: String::new(),
            nd_filename: String::new(),
            nd_path: String::new(),
            // The old dialog opened with `connections = ref(16)`.
            nd_connections: 16,
            nd_focus_next: false,
            nd_focus_frames: 0,
            nd_pasted: false,
            nd_autofill: String::new(),
            nd_allow_duplicate: false,
            nd_is_adding: false,
            nd_torrents: Vec::new(),
            nd_cat_enabled: false,
            nd_cat_mode: 0,
            nd_cat_selected: String::new(),
            nd_categories: Vec::new(),
            nd_cat_error: String::new(),
            nd_sched_enabled: false,
            nd_sched_open: false,
            nd_sched_set: false,
            nd_sched_action: 0,
            nd_sched_date: String::new(),
            nd_sched_time: String::new(),
            nd_sched_repeat: 0,
            nd_sched_error: String::new(),
            nd_proxy_enabled: false,
            nd_proxy_host: String::new(),
            nd_proxy_port: "1080".to_owned(),
            nd_proxy_user: String::new(),
            nd_proxy_pass: String::new(),
            nd_proxy_detecting: false,
            nd_proxy_msg: String::new(),
            nd_proxy_msg_ok: false,
            // Impossible key so the badge's first check fires on open.
            nd_pcheck_key: String::new(),
            nd_pcheck_at: Instant::now(),
            nd_pcheck_done: "\u{0}".to_owned(),
            nd_pcheck_busy: false,
            nd_pcheck_text: None,
            nd_pcheck_ok: false,
            nd_pcheck_direct: false,
            nd_was_open: false,
            nd_catmgr_open: false,
            nd_catmgr_new: String::new(),
            nd_catmgr_edit: None,
            nd_catmgr_edit_value: String::new(),
            nd_dd_open: None,
            nd_conn_memory: Vec::new(),
            nd_due_poll: Instant::now(),
            show_batch_dialog: false,
            batch_url_text: String::new(),
            batch_dir: String::new(),
            batch_connections: "1".to_owned(),
            batch_error: String::new(),
            batch_submitting: false,
            err_open: false,
            err_filename: String::new(),
            err_message: String::new(),
            hash_open: false,
            hash_path: String::new(),
            hash_name: String::new(),
            hash_algo: "sha256".to_owned(),
            hash_expected: String::new(),
            hash_computed: String::new(),
            hash_busy: false,
            hash_error: String::new(),
            cm_open: false,
            cm_pos: egui::pos2(0.0, 0.0),
            cm_gid: None,
            cm_group_menu: false,
            dm_open: false,
            dm_gid: None,
            dm_tab: 0,
            dm_data: None,
            dm_loading: false,
            dm_events: Vec::new(),
            dm_events_loading: false,
            dm_yt_health: None,
            dm_proxy_enabled: false,
            dm_proxy_type: "http".to_owned(),
            dm_proxy_host: String::new(),
            dm_proxy_port: "1080".to_owned(),
            dm_proxy_user: String::new(),
            dm_proxy_pass: String::new(),
            dm_proxy_applying: false,
            dm_proxy_msg: None,
            dm_proxy_silent: false,
            dm_detecting: false,
            dm_detect_msg: None,
            dm_db_row: None,
            dm_was_open: false,
            view_compact: false,
            sort_menu_open: false,
            grab_enabled: false,
            drag_gid: None,
            drag_from_group: None,
            drag_group: None,
            custom_order: Vec::new(),
            groups: crate::groups::load_groups(),
            g_create_open: false,
            g_create_name: String::new(),
            g_create_color: 0,
            g_create_options: false,
            g_create_on_complete: 0,
            g_create_speed: 0,
            g_create_dl_on: false,
            g_create_dl_value: 1,
            g_create_dl_unit: 1,
            g_create_members: Vec::new(),
            g_edit: None,
            g_edit_name: String::new(),
            g_edit_color: 0,
            g_edit_options: false,
            g_edit_on_complete: 0,
            g_edit_speed: 0,
            g_edit_dl_on: false,
            g_edit_dl_value: 1,
            g_edit_dl_unit: 1,
            g_edit_dl_used: 0,
            g_edit_members: Vec::new(),
            g_menu_gid: None,
            g_dl_open: false,
            g_dl_for_edit: false,
            g_dl_url: String::new(),
            g_pending: Vec::new(),
            g_pending_seq: 0,
            g_dd_open: None,
            g_speed_text: String::new(),
            g_dl_value_text: String::new(),
            maximized: false,
            first_seen: std::collections::HashMap::new(),
            speed_history: std::collections::HashMap::new(),
            limit_prev: std::collections::HashMap::new(),
            limit_verified: std::collections::HashSet::new(),
            limit_seed: 0,
            limit_reached: false,
            limit_pause_pending: false,
            limit_persist_pending: false,
            // First periodic persist lands 10s after boot (old
            // `startPeriodicPersist` interval).
            limit_next_persist: Instant::now() + std::time::Duration::from_secs(10),
            dla_open: false,
            dla_step: 0,
            dla_used: 0,
            dla_limit: 0,
            media: Default::default(),
            addon: Default::default(),
            spotify: Default::default(),
        }
    }
}

impl UiState {
    /// Stamp arrival times for new gids (mirrors the old `added_at` usage).
    pub fn note_first_seen(&mut self, rows: &[DownloadVm]) {
        let now = chrono_now_ms();
        for d in rows {
            // History rows carry their real `added_at`; live ones only have
            // the first poll we saw them in.
            let stamp = if d.added_at > 0 { d.added_at } else { now };
            // Check before cloning: `entry(gid.clone())` allocated a key for
            // every row on every poll, and only new rows need one.
            if !self.first_seen.contains_key(&d.gid) {
                self.first_seen.insert(d.gid.clone(), stamp);
            }
        }
    }

    /// Push this poll's speed into each active row's rolling history
    /// (mirrors `speedHistoryMap` in `useDownloads.ts`).
    pub fn record_speed_samples(&mut self, rows: &[DownloadVm]) {
        const MAX_SAMPLES: usize = 32;
        for d in rows {
            match self.speed_history.get_mut(&d.gid) {
                Some(e) => {
                    if d.status == Status::Active {
                        e.push_back(d.speed);
                        while e.len() > MAX_SAMPLES {
                            e.pop_front();
                        }
                    } else if !e.is_empty() {
                        e.clear();
                    }
                }
                // Key only allocated for a row we haven't seen yet; the old
                // `entry(gid.clone())` cloned every gid on every poll.
                None if d.status == Status::Active => {
                    self.speed_history
                        .insert(d.gid.clone(), VecDeque::from([d.speed]));
                }
                None => {}
            }
        }
        // Prune histories for gids no longer present. Set lookup, not a scan
        // per history entry (that was O(history × rows) every poll).
        let live: std::collections::HashSet<&str> = rows
            .iter()
            .chain(self.downloads.iter())
            .map(|r| r.gid.as_str())
            .collect();
        self.speed_history.retain(|gid, _| live.contains(gid.as_str()));
    }

    /// One `downloads-changed` snapshot through the old
    /// `useDownloadLimit` pipeline: `seedFromLiveItems` for the first 4
    /// snapshots, then `trackBytes` + `checkLimit`. On a hit it latches
    /// `limit_reached`, snapshots the numbers the alert renders and queues
    /// the pause-all + modal open for `modals::limit_tick`.
    pub fn limit_track(&mut self, items: &[serde_json::Value]) {
        if self.limit_seed < 4 {
            for item in items {
                let Some(gid) = item.get("gid").and_then(|v| v.as_str()) else { continue };
                let completed = num_u64(item, "completedLength");
                // Only move the baseline forward (a restart may regress it).
                if self.limit_prev.get(gid).is_none_or(|prev| completed >= *prev) {
                    self.limit_prev.insert(gid.to_owned(), completed);
                }
                if completed > 0 {
                    self.limit_verified.insert(gid.to_owned());
                }
            }
            self.limit_seed += 1;
            return;
        }

        let enabled = self.settings.download_limit_enabled;
        for item in items {
            let Some(gid) = item.get("gid").and_then(|v| v.as_str()) else { continue };
            let completed = num_u64(item, "completedLength");
            let prev = self.limit_prev.get(gid).copied().unwrap_or(0);
            self.limit_prev.insert(gid.to_owned(), completed);
            if !self.limit_verified.contains(gid) {
                // First sighting of real progress: establish the baseline,
                // don't count it (`verifiedGids` guard in the old tracker).
                if completed > 0 {
                    self.limit_verified.insert(gid.to_owned());
                }
                continue;
            }
            if enabled && !self.limit_reached && completed > prev {
                self.settings.download_limit_used_bytes += completed - prev;
            }
        }

        // `checkLimit()` — once per snapshot, latched until `resetLimit()`.
        let limit = self.settings.download_limit_bytes;
        if enabled && !self.limit_reached && limit > 0 && self.settings.download_limit_used_bytes >= limit {
            self.limit_reached = true;
            self.dla_used = self.settings.download_limit_used_bytes;
            self.dla_limit = limit;
            self.dla_step = 0;
            self.dla_open = true;
            self.limit_pause_pending = true;
        }
    }

    /// Port of `useDownloadLimit.resetLimit`: counter and per-gid baselines
    /// clear, downloads stay paused. Persistence is queued for the next
    /// `modals::limit_tick`.
    pub fn limit_reset(&mut self) {
        self.settings.download_limit_used_bytes = 0;
        self.limit_reached = false;
        self.limit_prev.clear();
        self.limit_verified.clear();
        self.limit_seed = 0;
        self.limit_persist_pending = true;
    }

    pub fn visible_downloads(&self) -> Vec<&DownloadVm> {
        let search = self.search.to_lowercase();
        self.downloads
            .iter()
            .filter(|d| match self.filter {
                // Old `filteredDownloads`: downloading = active/waiting/paused,
                // completed = complete/error.
                Filter::All => true,
                Filter::Downloading => matches!(
                    d.status,
                    Status::Active | Status::Waiting | Status::Paused
                ),
                Filter::Completed => matches!(d.status, Status::Complete | Status::Error),
            })
            .filter(|d| search.is_empty() || d.filename.to_lowercase().contains(&search))
            .collect()
    }

    pub fn sort_downloads(&mut self) {
        // `sortedDownloads`: name/size are plain ascending sorts, added and
        // completed both use `added_at` (live aria2 rows have none → 0, which
        // sorts them before every persisted row and, after the descending
        // reverse, leaves them at the bottom like the old merged array).
        match self.sort {
            SortKey::Added | SortKey::Completed => {
                self.downloads.sort_by_key(|d| d.added_at);
            }
            SortKey::Name => {
                // Cached key: `to_lowercase()` twice per comparison allocated
                // ~2n·log n strings every poll.
                self.downloads
                    .sort_by_cached_key(|d| d.filename.to_lowercase());
            }
            SortKey::Size => {
                self.downloads.sort_by_key(|d| d.total);
            }
            // `customOrder`: manual slots, unknown gids stay last (stable).
            SortKey::Manual => {
                // Slot index per gid: `order.iter().position()` per key made
                // this O(n^2) on every poll.
                let slot: std::collections::HashMap<&str, usize> = self
                    .custom_order
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (g.as_str(), i))
                    .collect();
                self.downloads.sort_by_key(|d| {
                    slot.get(d.gid.as_str()).copied().unwrap_or(usize::MAX)
                });
            }
        }
        // Old: `if (!asc && sort !== 'manual') list.reverse()`.
        if !self.sort_ascending && self.sort != SortKey::Manual {
            self.downloads.reverse();
        }
    }

    /// `reorderDownloads` (dropped on a sibling): swap the two manual slots so
    /// the dragged card lands where the target sat.
    pub fn manual_move(&mut self, gid: &str, target: &str) {
        if self.custom_order.is_empty() {
            self.custom_order = self.downloads.iter().map(|d| d.gid.clone()).collect();
        }
        if !swap_in(&mut self.custom_order, gid, target) {
            return;
        }
        self.sort = SortKey::Manual;
        self.sort_downloads();
    }

    /// `reorderGroups`: move the dragged group into the target's index.
    pub fn group_reorder(&mut self, dragged: &str, target: &str) {
        let Some(from) = self.groups.iter().position(|g| g.id == dragged) else {
            return;
        };
        let group = self.groups.remove(from);
        let to = self
            .groups
            .iter()
            .position(|g| g.id == target)
            .unwrap_or(self.groups.len());
        self.groups.insert(to, group);
        crate::groups::save_groups(&self.groups);
    }

    /// `groupedDownloads`: groups render as containers around their members;
    /// ungrouped rows follow. Group membership is resolved against the
    /// filtered/sorted list, and groups survive an empty body only on the
    /// "all" filter.
    pub fn grouped_entries(&self) -> Vec<DlEntry> {
        let visible = self.visible_downloads();
        let member_gids: std::collections::HashSet<&str> = self
            .groups
            .iter()
            .flat_map(|g| g.member_gids.iter().map(|s| s.as_str()))
            .collect();

        let mut items: Vec<DlEntry> = Vec::new();
        for g in &self.groups {
            let gids: Vec<String> = visible
                .iter()
                .filter(|d| g.has_member(&d.gid))
                .map(|d| d.gid.clone())
                .collect();
            if gids.is_empty() && self.filter != Filter::All {
                continue;
            }
            // `dateAdded`: the group's creation stamp, else its earliest
            // member's `added_at`.
            let earliest = visible
                .iter()
                .filter(|d| g.has_member(&d.gid))
                .map(|d| d.added_at)
                .filter(|t| *t > 0)
                .min()
                .unwrap_or(0);
            let date_added = if g.added_at > 0 { g.added_at } else { earliest };
            items.push(DlEntry::Group {
                id: g.id.clone(),
                gids,
                date_added,
            });
        }
        for d in &visible {
            if !member_gids.contains(d.gid.as_str()) {
                items.push(DlEntry::Download {
                    gid: d.gid.clone(),
                });
            }
        }

        // Sort keys are computed once per entry: the old closures rescanned
        // `downloads` for every comparison, which is O(n² log n) and cost 47ms
        // a frame at 3k rows. `to_lowercase` now runs once per entry, too.
        let by_gid: std::collections::HashMap<&str, &DownloadVm> =
            self.downloads.iter().map(|d| (d.gid.as_str(), d)).collect();
        let key = |e: &DlEntry| -> (String, i64, u64) {
            match e {
                DlEntry::Group {
                    id,
                    gids,
                    date_added,
                } => {
                    let name = self
                        .group_by_id(id)
                        .map(|g| g.name.to_lowercase())
                        .unwrap_or_default();
                    let size = gids
                        .iter()
                        .filter_map(|gid| by_gid.get(gid.as_str()))
                        .map(|d| d.total)
                        .sum();
                    (name, *date_added, size)
                }
                DlEntry::Download { gid } => by_gid
                    .get(gid.as_str())
                    .map(|d| (d.filename.to_lowercase(), d.added_at, d.total))
                    .unwrap_or_default(),
            }
        };
        let mut keyed: Vec<((String, i64, u64), DlEntry)> =
            items.into_iter().map(|e| (key(&e), e)).collect();

        match self.sort {
            SortKey::Manual => {}
            SortKey::Name => keyed.sort_by(|a, b| a.0.0.cmp(&b.0.0)),
            SortKey::Size => keyed.sort_by_key(|a| a.0.2),
            SortKey::Added | SortKey::Completed => keyed.sort_by_key(|a| a.0.1),
        }
        let mut items: Vec<DlEntry> = keyed.into_iter().map(|(_, e)| e).collect();
        if !self.sort_ascending && self.sort != SortKey::Manual {
            items.reverse();
        }
        items
    }

    /// `parseGroupSize` / `parseGroupProgress`: sum the members' completed and
    /// total bytes. Persisted rows only carry their formatted `size` string, so
    /// a completed history row contributes its whole size as done.
    pub fn group_totals(&self, gids: &[String]) -> (u64, u64) {
        let mut done = 0u64;
        let mut total = 0u64;
        // Index once: a linear scan per member made a big group O(n²) a frame.
        let by_gid: std::collections::HashMap<&str, &DownloadVm> =
            self.downloads.iter().map(|d| (d.gid.as_str(), d)).collect();
        for gid in gids {
            let Some(d) = by_gid.get(gid.as_str()) else {
                continue;
            };
            done += if d.size_text.is_empty() {
                d.completed
            } else {
                d.total
            };
            total += d.total;
        }
        (done, total)
    }

    /// Push a toast and trim old ones.
    pub fn toast(&mut self, ok: bool, text: impl Into<String>) {
        // Toast payloads are English keys ("Download cancelled"); translate
        // them here so each call site keeps its literal. Payloads that carry an
        // error look like `Failed to pause: <detail>`: translate the head — it is
        // a table key — keep the detail raw, and order the whole sentence once.
        let text = {
            let s: String = text.into();
            crate::i18n::show_detail(&self.settings.language, &s)
        };
        self.toasts.push(Toast {
            ok,
            text: text.into(),
            born: Instant::now(),
        });
        while self.toasts.len() > 3 {
            let _ = self.toasts.remove(0);
        }
    }

    pub fn prune_toasts(&mut self) {
        // Old app: auto-dismiss after exactly 3000ms (`useNotification.ts`),
        // matching the 3s progress-bar animation.
        self.toasts.retain(|t| t.born.elapsed().as_secs_f32() < 3.0);
    }

    /// `ClipboardToast.show(url)` — stack one row per detected link.
    pub fn clip_toast(&mut self, url: &str) {
        let id = self.clip_next_id;
        self.clip_next_id += 1;
        self.clip_toasts.push(ClipToast {
            id,
            url: url.to_owned(),
            born: Instant::now(),
        });
    }

    pub fn dismiss_clip_toast(&mut self, id: u64) {
        self.clip_toasts.retain(|t| t.id != id);
    }

    pub fn prune_clip_toasts(&mut self) {
        // `setTimeout(() => dismiss(id), 10000)`.
        self.clip_toasts.retain(|t| t.born.elapsed().as_secs_f32() < 10.0);
    }

    // ─── New-download dialog helpers (parity with `DownloadsPage.vue`) ───────

    /// Full post-add reset — the old dialog cleared every field it owned
    /// (`newUrl`/`newFilename`/`newPath`/`connections`/category/schedule/
    /// proxy/tab/duplicate/clipboard flags) once `addDownload` resolved.
    pub fn nd_reset(&mut self) {
        self.add_url.clear();
        self.nd_filename.clear();
        self.nd_autofill.clear();
        self.nd_path.clear();
        self.nd_magnet.clear();
        self.nd_connections = 16;
        self.nd_torrent_tab = false;
        self.nd_focus_next = false;
        self.nd_pasted = false;
        self.nd_allow_duplicate = false;
        self.nd_torrents.clear();
        self.nd_cat_enabled = false;
        self.nd_cat_mode = 0;
        self.nd_cat_selected.clear();
        self.nd_cat_error.clear();
        self.nd_sched_enabled = false;
        self.nd_sched_open = false;
        self.nd_sched_set = false;
        self.nd_sched_action = 0;
        self.nd_sched_date.clear();
        self.nd_sched_time.clear();
        self.nd_sched_repeat = 0;
        self.nd_sched_error.clear();
        self.nd_proxy_enabled = false;
        self.nd_proxy_host.clear();
        self.nd_proxy_port = "1080".to_owned();
        self.nd_proxy_user.clear();
        self.nd_proxy_pass.clear();
        self.nd_proxy_msg.clear();
    }

    /// Cancel/close: the fields the old dialog reset on the close path too
    /// (the URL field is re-read from the clipboard on the next open).
    pub fn nd_clear_source(&mut self) {
        self.add_url.clear();
        self.nd_filename.clear();
        self.nd_autofill.clear();
        self.nd_magnet.clear();
        self.nd_pasted = false;
        self.nd_focus_next = false;
    }

    /// Host part of an absolute URL — `new URL(u).hostname` without the
    /// user/port and a leading `www.` (`detectedHost`, also used by the addon
    /// URL matcher, hence the qualified `UiState::host_of`).
    pub fn host_of(u: &str) -> Option<String> {
        let after = u.trim().split_once("://").map(|(_, r)| r)?;
        let authority = after.split(['/', '?', '#']).next().unwrap_or("");
        let host = authority.rsplit('@').next().unwrap_or("");
        let host = match host.rfind(':') {
            Some(i) if !host[i + 1..].chars().any(|c| !c.is_ascii_digit()) => &host[..i],
            _ => host,
        };
        let host = host.strip_prefix("www.").unwrap_or(host);
        (!host.is_empty()).then(|| host.to_owned())
    }

    /// `t(key)` — the old app translates by English key (`i18n.t`).
    /// `preview-select` → scroll the card into view once and ring it for 1.6s.
    pub fn start_flash(&mut self, gid: impl Into<String>) {
        let gid = gid.into();
        self.focus_gid = Some(gid.clone());
        self.flash_gid = Some(gid);
        self.flash_at = Some(std::time::Instant::now());
    }

    /// Milliseconds left of the `.flash-highlight` ring, if this card has it.
    /// `MOONHUNT_FLASH_MS` stretches 1.6s for parity screenshots.
    pub fn flash_ms_left(&self, gid: &str) -> Option<u64> {
        if self.flash_gid.as_deref() != Some(gid) {
            return None;
        }
        let total = std::env::var("MOONHUNT_FLASH_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1600u64);
        let ms = (total as u128).saturating_sub(self.flash_at?.elapsed().as_millis());
        (ms > 0).then_some(ms as u64)
    }

    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        crate::i18n::t(&self.settings.language, key)
    }

    /// Logical text for `key` — splice numbers into this, then paint the whole
    /// sentence with `t_of`; see `i18n::raw`.
    pub fn raw<'a>(&self, key: &'a str) -> &'a str {
        crate::i18n::raw(&self.settings.language, key)
    }

    /// Paint a sentence built from `raw` pieces in logical order.
    pub fn t_of(&self, text: &str) -> String {
        crate::i18n::show(&self.settings.language, text)
    }

    /// `detectedHost` — `new URL(url).hostname` minus a leading `www.`.
    pub fn nd_detected_host(&self) -> Option<String> {
        Self::host_of(&self.add_url)
    }

    /// `detectedFilename` — the URL path's last segment (percent-decoded).
    pub fn nd_detected_filename(&self) -> String {
        let u = self.add_url.trim();
        let Some(after) = u.split_once("://").map(|(_, r)| r) else {
            return String::new();
        };
        let path = after.split(['?', '#']).next().unwrap_or("");
        let last = path.rsplit('/').next().unwrap_or("");
        if last.is_empty() {
            return String::new();
        }
        percent_decode(last)
    }

    /// `watch(newUrl)` autofill — mirror the old rule: fill the Save As field
    /// from the URL, but only replace a value we filled ourselves.
    pub fn nd_autofill_filename(&mut self) {
        if self.add_url.trim().is_empty() {
            self.nd_filename.clear();
            self.nd_autofill.clear();
            return;
        }
        let detected = self.nd_detected_filename();
        if detected.is_empty() {
            return;
        }
        if self.nd_filename.is_empty() || self.nd_filename == self.nd_autofill {
            self.nd_autofill = detected.clone();
            self.nd_filename = detected;
        }
    }

    /// `duplicateDownload` — an existing list row with the same URL.
    pub fn nd_duplicate_gid(&self) -> Option<String> {
        let url = self.add_url.trim().to_lowercase();
        if url.is_empty() {
            return None;
        }
        self.downloads
            .iter()
            .find(|d| !d.url.is_empty() && d.url.to_lowercase() == url)
            .map(|d| d.gid.clone())
    }

    /// Repeat interval in ms for the schedule builder (`getRepeatMs`).
    pub fn nd_repeat_ms(&self) -> i64 {
        match self.nd_sched_repeat {
            1 => 60 * 60 * 1000,
            2 => 24 * 60 * 60 * 1000,
            3 => 7 * 24 * 60 * 60 * 1000,
            _ => 0,
        }
    }

    pub fn nd_action_label(&self) -> &'static str {
        ["Start", "Pause", "Resume"][self.nd_sched_action.min(2) as usize]
    }

    pub fn nd_repeat_label(&self) -> &'static str {
        ["Once", "Hourly", "Daily", "Weekly"][self.nd_sched_repeat.min(3) as usize]
    }

    /// Epoch ms of the configured local date/time (the browser's
    /// `datetime-local` value is local wall-clock, so subtract the OS offset).
    pub fn nd_schedule_epoch_ms(&self) -> Option<i64> {
        let (y, m, d) = split_date(&self.nd_sched_date)?;
        let (hh, mm) = split_time(&self.nd_sched_time)?;
        let local = days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60;
        let utc = local - local_utc_offset_ms() / 1000;
        Some(utc * 1000)
    }

    /// `schedulePreview` — "Start - Oct 25, 2026 at 14:30 - Once".
    pub fn nd_schedule_preview(&self) -> String {
        if !self.nd_sched_set {
            return String::new();
        }
        let date = match (split_date(&self.nd_sched_date), split_time(&self.nd_sched_time)) {
            (Some((y, m, d)), Some((hh, mm))) => {
                format!(
                    "{} {}, {} at {:02}:{:02}",
                    month_abbr(m),
                    d,
                    y,
                    hh,
                    mm
                )
            }
            _ => format!("{} {}", self.nd_sched_date, self.nd_sched_time),
        };
        format!(
            "{} - {} - {}",
            self.t(self.nd_action_label()),
            date,
            self.t(self.nd_repeat_label())
        )
    }

    /// Per-download proxy form → `PerDownloadProxy` (the old `serializeProxy`:
    /// disabled or missing a host means "inherit global"). HTTP only.
    pub fn nd_proxy_value(&self) -> Option<moonhunt_core::state::PerDownloadProxy> {
        if !self.nd_proxy_enabled {
            return None;
        }
        let host = self.nd_proxy_host.trim();
        if host.is_empty() {
            return None;
        }
        let port = self
            .nd_proxy_port
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|p| *p > 0)
            .unwrap_or(1080);
        Some(moonhunt_core::state::PerDownloadProxy {
            enabled: true,
            proxy_type: "http".to_owned(),
            proxy_host: host.to_owned(),
            proxy_port: port,
            proxy_user: self.nd_proxy_user.clone(),
            proxy_pass: self.nd_proxy_pass.clone(),
        })
    }

    /// Schedule form → `DownloadSchedule` for `add_download` (the old
    /// `scheduleEnabled ? pendingSchedule : null`).
    pub fn nd_schedule_value(&self) -> Option<moonhunt_core::state::DownloadSchedule> {
        if !self.nd_sched_enabled || !self.nd_sched_set {
            return None;
        }
        let scheduled_time = self.nd_schedule_epoch_ms()?;
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        Some(moonhunt_core::state::DownloadSchedule {
            id: None,
            download_gid: None,
            url: self.add_url.trim().to_owned(),
            filename: self.nd_filename.trim().to_owned(),
            action: ["start", "pause", "resume"][self.nd_sched_action.min(2) as usize].to_owned(),
            scheduled_time,
            repeat_interval: Some(self.nd_repeat_ms()),
            enabled: true,
            created_at,
        })
    }
}

/// `withDuplicateSuffix` — `name (1).ext` before the extension.
pub fn with_duplicate_suffix(name: &str, count: usize) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => format!("{} ({}){}", &name[..i], count, &name[i..]),
        _ => format!("{} ({})", name, count),
    }
}

/// `sanitizeCategoryFolder` — replace path-hostile chars, collapse whitespace,
/// guard dot-only names, cap at 80 chars (the old `sanitizeCategoryFolder`).
pub fn sanitize_category_folder(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    // JS `replace(/\s+/g, ' ')` — collapse runs, no trim.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut prev_ws = false;
    for c in replaced.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                collapsed.push(' ');
            }
            prev_ws = true;
        } else {
            collapsed.push(c);
            prev_ws = false;
        }
    }
    if !collapsed.is_empty() && collapsed.chars().all(|c| c == '.') {
        return "_".to_owned();
    }
    let clipped: String = collapsed.chars().take(80).collect();
    if clipped.is_empty() {
        "uncategorized".to_owned()
    } else {
        clipped
    }
}

/// `extractFileExtension` — last path segment, `?query` stripped, trailing
/// `[a-zA-Z0-9]{1,10}` run after a dot.
pub fn extract_file_extension(value: &str) -> String {
    let name = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let clean = name.split('?').next().unwrap_or(name);
    if let Some(dot) = clean.rfind('.') {
        let ext = &clean[dot + 1..];
        if !ext.is_empty()
            && ext.len() <= 10
            && ext.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return ext.to_ascii_lowercase();
        }
    }
    String::new()
}

/// Extension → folder tables of `utils/downloadCategories.ts`.
const FORMAT_GROUPS: [(&str, &[&str]); 8] = [
    ("video", &["mkv", "webm", "mov", "avi", "flv", "wmv", "m4v"]),
    ("audio", &["mp3", "flac", "wav", "m4a", "ogg", "aac", "aiff", "aif", "wma"]),
    ("image", &["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp", "ico"]),
    ("document", &["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "ppt", "pptx"]),
    ("code", &["js", "ts", "py", "rs", "go", "cpp", "c", "h", "java", "cs", "php", "rb"]),
    ("archive", &["rar", "7z", "tar", "gz", "bz2", "xz"]),
    ("installer", &["msi", "dmg", "pkg", "apk", "appx"]),
    ("disk-image", &["iso", "img", "vhd", "vdi"]),
];

const PACKAGED_GROUPS: [(&str, &[&str]); 8] = [
    ("video", &["mp4", "mkv", "webm", "mov", "avi", "flv", "wmv", "m4v"]),
    ("audio", &["mp3", "aiff", "aif", "flac", "wav", "m4a", "ogg", "aac"]),
    ("image", &["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp", "ico"]),
    ("document", &["pdf", "doc", "docx", "txt", "md", "rtf", "odt", "xls", "xlsx", "ppt", "pptx"]),
    ("code", &["js", "ts", "py", "rs", "go", "cpp", "c", "h", "java", "cs", "php", "rb"]),
    ("archive", &["zip", "rar", "7z", "tar", "gz", "bz2", "xz"]),
    ("installer", &["exe", "msi", "dmg", "pkg", "apk", "appx"]),
    ("disk-image", &["iso", "img", "vhd", "vdi"]),
];

/// `formattedCategoryFolder` — exact mp4/zip/exe passthrough, else a group.
pub fn formatted_category_folder(value: &str) -> String {
    let ext = extract_file_extension(value);
    if ext.is_empty() {
        return "other".to_owned();
    }
    if matches!(ext.as_str(), "mp4" | "zip" | "exe") {
        return ext;
    }
    FORMAT_GROUPS
        .iter()
        .find(|(_, exts)| exts.contains(&ext.as_str()))
        .map(|(f, _)| (*f).to_owned())
        .unwrap_or_else(|| "other".to_owned())
}

/// `packagedCategoryFolder` — group lookup only (mp4 lands in `video`).
pub fn packaged_category_folder(value: &str) -> String {
    let ext = extract_file_extension(value);
    if ext.is_empty() {
        return "other".to_owned();
    }
    PACKAGED_GROUPS
        .iter()
        .find(|(_, exts)| exts.contains(&ext.as_str()))
        .map(|(f, _)| (*f).to_owned())
        .unwrap_or_else(|| "other".to_owned())
}

/// `joinDownloadPath` — `base\folder` (always a backslash, like the old app).
pub fn join_download_path(base: &str, child: &str) -> String {
    let folder = sanitize_category_folder(child);
    let trimmed = base.trim().trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        folder
    } else {
        format!("{}\\{}", trimmed, folder)
    }
}

/// `resolveCategoryDir` — target dir of a download for the category mode.
/// 0 = formatted, 1 = packaged, 2 = manual.
pub fn resolve_category_dir(
    enabled: bool,
    mode: u8,
    selected: &str,
    filename: &str,
    base: &str,
) -> Option<String> {
    let base = base.trim();
    if !enabled {
        return (!base.is_empty()).then(|| base.to_owned());
    }
    if base.is_empty() {
        return None;
    }
    match mode {
        1 => Some(join_download_path(base, &packaged_category_folder(filename))),
        2 => {
            if selected.trim().is_empty() {
                return None;
            }
            Some(join_download_path(base, selected))
        }
        _ => Some(join_download_path(base, &formatted_category_folder(filename))),
    }
}

/// Minimal `%XX` decoding for the detected filename.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `2026-10-25` → (year, month, day).
fn split_date(s: &str) -> Option<(i64, i64, i64)> {
    let mut it = s.split('-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// `14:35` → (hour, minute).
fn split_time(s: &str) -> Option<(i64, i64)> {
    let mut it = s.split(':');
    let h = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    if it.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn month_abbr(m: i64) -> &'static str {
    MONTHS
        .get(m.clamp(1, 12) as usize - 1)
        .copied()
        .unwrap_or("Jan")
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Local UTC offset in ms, derived by differencing the OS local wall clock
/// against UTC (so DST is applied as of now, matching what a browser's
/// `datetime-local` → `Date` conversion would do).
pub(crate) fn local_utc_offset_ms() -> i64 {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        #[derive(Default, Clone, Copy)]
        struct WinSysTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            milliseconds: u16,
        }
        extern "system" {
            fn GetLocalTime(t: *mut WinSysTime);
            fn GetSystemTime(t: *mut WinSysTime);
        }
        let stamp = |t: &WinSysTime| -> i64 {
            days_from_civil(t.year as i64, t.month as i64, t.day as i64) * 86_400
                + t.hour as i64 * 3600
                + t.minute as i64 * 60
                + t.second as i64
        };
        unsafe {
            let mut local = WinSysTime::default();
            let mut utc = WinSysTime::default();
            GetLocalTime(&mut local);
            GetSystemTime(&mut utc);
            (stamp(&local) - stamp(&utc)) * 1000
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

/// Swap two gids' slots in the manual order; `false` when either is absent
/// (or they are the same row), so `manual_move` can bail without side effects.
fn swap_in(order: &mut [String], a: &str, b: &str) -> bool {
    match (
        order.iter().position(|g| g == a),
        order.iter().position(|g| g == b),
    ) {
        (Some(i), Some(j)) if i != j => {
            order.swap(i, j);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod drag_order_tests {
    use super::swap_in;

    fn order(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn drop_on_sibling_swaps_slots() {
        let mut o = order(&["a", "b", "c"]);
        assert!(swap_in(&mut o, "a", "c"), "both present");
        assert_eq!(o, order(&["c", "b", "a"]));
    }

    #[test]
    fn same_or_unknown_row_is_a_noop() {
        let mut o = order(&["a", "b"]);
        assert!(!swap_in(&mut o, "a", "a"));
        assert!(!swap_in(&mut o, "a", "nope"));
        assert_eq!(o, order(&["a", "b"]));
    }

    /// The titlebar search preview prints `{{ item.status }}` from the web
    /// store, i.e. the raw token — not the capitalised label.
    #[test]
    fn status_tokens_match_the_web_store() {
        use crate::state::Status;
        assert_eq!(Status::Active.token(), "active");
        assert_eq!(Status::Complete.token(), "complete");
        assert_eq!(Status::Removed.token(), "removed");
        assert_eq!(Status::Complete.label(), "Complete");
    }
}
