//! Native Spotify Playlist Bridge.
//!
//! The embedded addon is an ESM module and this build has no JS runtime.
//! `moonhunt_core::spotify` reads the link; this screen matches each track on
//! YouTube and queues the downloads.
//!
//! Resolve, match, and download each have their own generation. Cancelling one
//! does not retire the others. Progress and the matching completion event are
//! both `app.emit_async`'d, in order, so the completion cannot land first.

mod ui;

use super::Ctx;
use crate::state::UiState;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub use ui::show;

pub const ADDON_ID: &str = "moonhunt-spotify-playlist-bridge";

/// YouTube search for one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    Idle,
    Searching,
    Matched {
        url: String,
        title: String,
        duration: Option<u64>,
    },
    Failed(String),
}

/// aria2 handoff for one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Download {
    Idle,
    Starting,
    Queued(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub mt: Match,
    pub dl: Download,
    match_token: u64,
    dl_token: u64,
}

impl Track {
    fn new(title: String, artist: String) -> Self {
        Self {
            title,
            artist,
            mt: Match::Idle,
            dl: Download::Idle,
            match_token: 0,
            dl_token: 0,
        }
    }

    fn matched_url(&self) -> Option<&str> {
        match &self.mt {
            Match::Matched { url, .. } => Some(url),
            _ => None,
        }
    }
}

/// Logical note. Painted through `state.t` / `state.raw` so it can translate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Empty,
    OembedOne,
    Scraped(usize),
    Named(usize, String),
}

pub struct SpotifyUi {
    pub url: String,
    pub manual: String,
    pub show_manual: bool,
    pub playlist: String,
    pub note: Note,
    pub error: String,
    pub resolving: bool,
    pub tracks: Vec<Track>,
    pub matching: bool,
    pub downloading: bool,
    /// "mp3" or "mp4".
    pub audio_format: String,
    pub group_id: String,
    /// Rolling search/download log shown in the screen's log drawer.
    pub logs: VecDeque<String>,
    pub show_logs: bool,
    /// "legacy" | "p2" | "p3" | "all". Persisted under `spm-search-mode`.
    pub search_mode: String,
    /// Requirements strip; `None` until the first check reports.
    pub req_ytdlp: Option<bool>,
    pub req_runtime: Option<bool>,
    pub req_runtime_name: String,
    pub req_merger: Option<bool>,
    pub req_merger_name: String,
    pub req_checking: bool,
    /// Set when a resolve succeeded; the next frame starts matching.
    need_match: bool,
    mode_loaded: bool,
    resolve_gen: u64,
    match_gen: u64,
    dl_gen: u64,
    seq: u64,
    /// Loop keeps running while this equals the job's generation. 0 stops it.
    match_run: Arc<AtomicU64>,
    dl_run: Arc<AtomicU64>,
}

impl Default for SpotifyUi {
    fn default() -> Self {
        Self {
            url: String::new(),
            manual: String::new(),
            show_manual: false,
            playlist: String::new(),
            note: Note::Empty,
            error: String::new(),
            resolving: false,
            tracks: Vec::new(),
            matching: false,
            downloading: false,
            audio_format: "mp3".to_string(),
            group_id: String::new(),
            logs: VecDeque::new(),
            show_logs: false,
            search_mode: "legacy".to_string(),
            req_ytdlp: None,
            req_runtime: None,
            req_runtime_name: String::new(),
            req_merger: None,
            req_merger_name: String::new(),
            req_checking: false,
            need_match: false,
            mode_loaded: false,
            resolve_gen: 0,
            match_gen: 0,
            dl_gen: 0,
            seq: 0,
            match_run: Arc::new(AtomicU64::new(0)),
            dl_run: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl SpotifyUi {
    pub fn matched(&self) -> usize {
        self.tracks
            .iter()
            .filter(|t| matches!(t.mt, Match::Matched { .. }))
            .count()
    }

    /// Matched rows that are not already queued. Starting rows still count,
    /// matching the button label the screen has always shown.
    pub fn ready_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|t| t.matched_url().is_some() && !matches!(t.dl, Download::Queued(_)))
            .count()
    }

    fn bump(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    pub(crate) fn log(&mut self, msg: impl Into<String>) {
        self.logs.push_back(msg.into());
        while self.logs.len() > 300 {
            self.logs.pop_front();
        }
    }

    pub(crate) fn load_search_mode(&mut self) {
        if self.mode_loaded {
            return;
        }
        self.mode_loaded = true;
        if let Some(mode) = moonhunt_core::db::get_setting_json("spm-search-mode")
            .and_then(|v| v.as_str().map(str::to_string))
            .filter(|m| matches!(m.as_str(), "legacy" | "p2" | "p3" | "all"))
        {
            self.search_mode = mode;
        }
    }

    pub(crate) fn set_search_mode(&mut self, mode: &str) {
        if !matches!(mode, "legacy" | "p2" | "p3" | "all") || self.search_mode == mode {
            return;
        }
        self.search_mode = mode.to_string();
        let _ = moonhunt_core::db::set_setting_json("spm-search-mode", &serde_json::json!(mode));
    }

    fn apply_requirements(&mut self, ev: &RequirementsEv) {
        self.req_checking = false;
        self.req_ytdlp = Some(ev.ytdlp);
        self.req_runtime = Some(ev.runtime_ok);
        self.req_runtime_name = ev.runtime_name.clone();
        self.req_merger = Some(ev.merger_ok);
        self.req_merger_name = ev.merger_name.clone();
        self.log(format!(
            "Requirements: yt-dlp={}, runtime={} ({}), merger={} ({})",
            if ev.ytdlp { "ok" } else { "missing" },
            if ev.runtime_ok { "ok" } else { "missing" },
            if ev.runtime_name.is_empty() { "?" } else { &ev.runtime_name },
            if ev.merger_ok { "ok" } else { "missing" },
            if ev.merger_name.is_empty() { "?" } else { &ev.merger_name },
        ));
    }

    /// Drop every in-flight job. A new list must not receive the old one's events.
    fn supersede(&mut self) {
        let gen = self.bump();
        self.resolve_gen = gen;
        self.match_gen = gen;
        self.dl_gen = gen;
        self.match_run.store(0, Ordering::Release);
        self.dl_run.store(0, Ordering::Release);
        self.matching = false;
        self.downloading = false;
        self.resolving = false;
        self.need_match = false;
        self.tracks.clear();
        self.group_id.clear();
        self.note = Note::Empty;
        self.error.clear();
        self.playlist.clear();
        self.logs.clear();
    }

    fn begin_resolve(&mut self) -> Result<u64, ()> {
        if self.url.trim().is_empty() {
            self.error = "Please enter a URL".to_string();
            return Err(());
        }
        self.supersede();
        self.resolving = true;
        Ok(self.resolve_gen)
    }

    fn begin_manual(&mut self) -> Option<MatchJob> {
        let lines: Vec<(String, String)> = self
            .manual
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|line| match line.split_once(" - ") {
                Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
                None => ("Unknown artist".to_string(), line.to_string()),
            })
            .collect();
        if lines.is_empty() {
            self.error = "Paste one \"Artist - Title\" per line.".to_string();
            return None;
        }
        self.supersede();
        self.tracks = lines
            .into_iter()
            .map(|(artist, title)| Track::new(title, artist))
            .collect();
        self.playlist = "Spotify tracks".to_string();
        self.show_manual = false;
        self.begin_match()
    }

    fn begin_match(&mut self) -> Option<MatchJob> {
        let gen = self.bump();
        let mut items = Vec::new();
        for (idx, t) in self.tracks.iter_mut().enumerate() {
            if matches!(t.mt, Match::Matched { .. }) {
                continue;
            }
            t.mt = Match::Searching;
            t.match_token = gen;
            items.push(MatchItem {
                idx,
                query: format!("{} {}", t.title, t.artist),
                token: gen,
            });
        }
        if items.is_empty() {
            return None;
        }
        self.match_gen = gen;
        self.match_run.store(gen, Ordering::Release);
        self.matching = true;
        Some(MatchJob {
            gen,
            items,
            mode: self.search_mode.clone(),
        })
    }

    /// One row. Does not finish or cancel a match-all that is already running.
    fn begin_match_one(&mut self, idx: usize) -> Option<MatchItem> {
        let blocked = self
            .tracks
            .get(idx)
            .is_none_or(|t| matches!(t.mt, Match::Matched { .. } | Match::Searching));
        if blocked {
            return None;
        }
        let token = self.bump();
        let t = &mut self.tracks[idx];
        t.mt = Match::Searching;
        t.match_token = token;
        let query = format!("{} {}", t.title, t.artist);
        Some(MatchItem { idx, query, token })
    }

    fn cancel_match(&mut self) {
        let old = self.match_gen;
        let gen = self.bump();
        self.match_gen = gen;
        self.match_run.store(0, Ordering::Release);
        self.matching = false;
        for t in &mut self.tracks {
            if t.match_token == old && matches!(t.mt, Match::Searching) {
                t.mt = Match::Idle;
                t.match_token = 0;
            }
        }
    }

    fn begin_downloads(&mut self, only: Option<usize>) -> Option<DlJob> {
        let batch = only.is_none();
        let gen = self.bump();
        let mut items = Vec::new();
        for (idx, t) in self.tracks.iter_mut().enumerate() {
            if only.is_some_and(|o| o != idx) {
                continue;
            }
            let Some(url) = t.matched_url().map(str::to_string) else {
                continue;
            };
            if matches!(t.dl, Download::Starting | Download::Queued(_)) {
                continue;
            }
            t.dl = Download::Starting;
            t.dl_token = gen;
            items.push(DlItem { idx, url, token: gen });
        }
        if items.is_empty() {
            return None;
        }
        if batch {
            self.dl_gen = gen;
            self.dl_run.store(gen, Ordering::Release);
            self.downloading = true;
        }
        Some(DlJob {
            gen,
            batch,
            items,
            format: self.audio_format.clone(),
            run: self.dl_run.clone(),
        })
    }

    /// Stop the queue between tracks. Rows already sent to yt-dlp still report.
    fn cancel_downloads(&mut self) {
        self.dl_run.store(0, Ordering::Release);
    }

    fn clear(&mut self) -> Vec<String> {
        let gids = self
            .tracks
            .iter()
            .filter_map(|t| match &t.dl {
                Download::Queued(gid) if !gid.is_empty() => Some(gid.clone()),
                _ => None,
            })
            .collect();
        self.supersede();
        self.url.clear();
        gids
    }

    fn take_need_match(&mut self) -> bool {
        std::mem::take(&mut self.need_match) && !self.tracks.is_empty()
    }

    fn apply_resolve(&mut self, ev: &ResolveEv) {
        if ev.gen != self.resolve_gen {
            return;
        }
        self.resolving = false;
        if !ev.error.is_empty() {
            self.error = ev.error.clone();
            self.show_manual = true;
            self.log(format!("Resolve failed: {}", ev.error));
            return;
        }
        self.playlist = ev.playlist.clone();
        let n = ev.tracks.len();
        self.log(format!(
            "Resolved \"{}\": {n} track(s)",
            if self.playlist.is_empty() {
                "untitled"
            } else {
                self.playlist.as_str()
            }
        ));
        self.tracks = ev
            .tracks
            .iter()
            .map(|t| Track::new(t.title.clone(), t.artist.clone()))
            .collect();
        self.note = if n == 1 && self.playlist.is_empty() {
            Note::OembedOne
        } else if self.playlist.is_empty() {
            Note::Scraped(n)
        } else {
            Note::Named(n, self.playlist.clone())
        };
        self.need_match = !self.tracks.is_empty();
    }

    fn apply_match_one(&mut self, ev: &MatchOneEv) {
        let msg = {
            let Some(t) = self.tracks.get_mut(ev.idx) else {
                return;
            };
            if t.match_token != ev.token {
                return;
            }
            if !ev.url.is_empty() {
                t.mt = Match::Matched {
                    url: ev.url.clone(),
                    title: ev.title.clone(),
                    duration: ev.duration,
                };
                format!(
                    "#{} \"{}\" -> {} ({}ms)",
                    ev.idx + 1,
                    ev.query,
                    ev.title,
                    ev.elapsed_ms
                )
            } else {
                let err: String = if ev.error.is_empty() {
                    "No YouTube match".to_string()
                } else {
                    ev.error.chars().take(140).collect()
                };
                let msg = format!(
                    "#{} \"{}\" failed: {} ({}ms)",
                    ev.idx + 1,
                    ev.query,
                    err,
                    ev.elapsed_ms
                );
                t.mt = Match::Failed(err);
                msg
            }
        };
        self.log(msg);
    }

    fn apply_match_done(&mut self, gen: u64) {
        if gen != self.match_gen {
            return;
        }
        self.matching = false;
        let mut stuck = 0usize;
        for t in &mut self.tracks {
            if t.match_token == gen && matches!(t.mt, Match::Searching) {
                t.mt = Match::Failed("Search did not finish".to_string());
                stuck += 1;
            }
        }
        if stuck > 0 {
            self.log(format!("{stuck} search(es) never reported; marked failed"));
        }
        self.log("Search finished.");
    }

    /// `Some(gid)` when a row was newly queued and should join the group.
    fn apply_dl_one(&mut self, ev: &DlOneEv) -> Option<String> {
        let t = self.tracks.get_mut(ev.idx)?;
        if t.dl_token != ev.token {
            return None;
        }
        if ev.skipped {
            if matches!(t.dl, Download::Starting) {
                t.dl = Download::Idle;
            }
            return None;
        }
        if !ev.gid.is_empty() {
            t.dl = Download::Queued(ev.gid.clone());
            return Some(ev.gid.clone());
        }
        t.dl = Download::Failed(if ev.error.is_empty() {
            "Could not start".to_string()
        } else {
            ev.error.clone()
        });
        None
    }

    fn apply_dl_done(&mut self, gen: u64) {
        if gen == self.dl_gen {
            self.downloading = false;
        }
    }
}

struct MatchItem {
    idx: usize,
    query: String,
    token: u64,
}

struct MatchJob {
    gen: u64,
    items: Vec<MatchItem>,
    mode: String,
}

struct DlItem {
    idx: usize,
    url: String,
    token: u64,
}

struct DlJob {
    gen: u64,
    batch: bool,
    items: Vec<DlItem>,
    format: String,
    run: Arc<AtomicU64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResolveEv {
    gen: u64,
    playlist: String,
    tracks: Vec<moonhunt_core::spotify::SpTrack>,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MatchOneEv {
    token: u64,
    idx: usize,
    query: String,
    elapsed_ms: u64,
    url: String,
    title: String,
    duration: Option<u64>,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DoneEv {
    gen: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DlOneEv {
    token: u64,
    idx: usize,
    gid: String,
    error: String,
    skipped: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClearedEv {
    stopped: u64,
    skipped: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequirementsEv {
    ytdlp: bool,
    runtime_ok: bool,
    runtime_name: String,
    merger_ok: bool,
    merger_name: String,
}

fn parse<T: for<'de> Deserialize<'de>>(payload: &serde_json::Value) -> Option<T> {
    serde_json::from_value(payload.clone()).ok()
}

fn start_requirements(state: &mut UiState, ctx: &Ctx) {
    if state.spotify.req_checking {
        return;
    }
    state.spotify.req_checking = true;
    ctx.emit("spotify-ack", move |app| async move {
        let ytdlp = moonhunt_core::ytdlp::check_yt_dlp_exists(app.clone()).unwrap_or(false);
        let runtimes = moonhunt_core::js_runtime::get_js_runtimes(app.clone()).ok();
        let mergers = moonhunt_core::merger::get_mergers(app.clone()).ok();
        let ev = requirements_ev(ytdlp, runtimes, mergers);
        app.emit_async("spotify-requirements", ev).await;
        Ok(serde_json::Value::Null)
    });
}

fn requirements_ev(
    ytdlp: bool,
    runtimes: Option<serde_json::Value>,
    mergers: Option<serde_json::Value>,
) -> RequirementsEv {
    let flag = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
    let name = |v: &serde_json::Value| {
        v.get("name")
            .and_then(|x| x.as_str())
            .or_else(|| v.get("id").and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string()
    };

    let mut runtime_ok = false;
    let mut runtime_name = String::new();
    if let Some(v) = runtimes.as_ref() {
        let list = v
            .get("runtimes")
            .and_then(|x| x.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let sys = v.get("systemNode").filter(|x| !x.is_null());
        if sys.is_some_and(|x| flag(x, "selected")) {
            runtime_name = "System Node.js".to_string();
            runtime_ok = true;
        } else if let Some(chosen) = list
            .iter()
            .find(|r| flag(r, "selected") && flag(r, "installed"))
        {
            runtime_name = name(chosen);
            runtime_ok = true;
        } else if sys.is_some() {
            runtime_name = "System Node.js (auto)".to_string();
            runtime_ok = true;
        } else if list.iter().any(|r| flag(r, "installed")) {
            runtime_name = "Node.js (auto, PATH)".to_string();
            runtime_ok = true;
        }
    }

    let mut merger_ok = false;
    let mut merger_name = String::new();
    if let Some(v) = mergers.as_ref() {
        let list = v
            .get("mergers")
            .and_then(|x| x.as_array())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let sys = v.get("systemFfmpeg").filter(|x| !x.is_null());
        if sys.is_some_and(|x| flag(x, "selected")) {
            merger_name = "System FFmpeg".to_string();
            merger_ok = true;
        } else if let Some(chosen) = list
            .iter()
            .find(|m| flag(m, "selected") && flag(m, "installed"))
        {
            merger_name = name(chosen);
            merger_ok = true;
        } else if let Some(any) = list.iter().find(|m| flag(m, "installed")) {
            merger_name = format!("{} (auto)", name(any));
            merger_ok = true;
        }
    }

    RequirementsEv {
        ytdlp,
        runtime_ok,
        runtime_name,
        merger_ok,
        merger_name,
    }
}

fn start_resolve(state: &mut UiState, ctx: &Ctx) {
    let Ok(gen) = state.spotify.begin_resolve() else {
        return;
    };
    let url = state.spotify.url.trim().to_string();
    ctx.emit("spotify-ack", move |app| async move {
        let ev = if !moonhunt_core::ytdlp::check_yt_dlp_exists(app.clone()).unwrap_or(false) {
            ResolveEv {
                gen,
                playlist: String::new(),
                tracks: Vec::new(),
                error: "yt-dlp is not installed — set it up on the Media page first.".to_string(),
            }
        } else {
            match moonhunt_core::spotify::spotify_resolve(app.clone(), url).await {
                Ok((playlist, tracks)) => ResolveEv {
                    gen,
                    playlist,
                    tracks,
                    error: String::new(),
                },
                Err(error) => ResolveEv {
                    gen,
                    playlist: String::new(),
                    tracks: Vec::new(),
                    error,
                },
            }
        };
        app.emit_async("spotify-resolve", ev).await;
        Ok(serde_json::Value::Null)
    });
}

fn start_manual(state: &mut UiState, ctx: &Ctx) {
    let Some(job) = state.spotify.begin_manual() else {
        return;
    };
    spawn_match(ctx, state.spotify.match_run.clone(), job);
}

fn start_match(state: &mut UiState, ctx: &Ctx) {
    let Some(job) = state.spotify.begin_match() else {
        return;
    };
    spawn_match(ctx, state.spotify.match_run.clone(), job);
}

fn start_match_one(state: &mut UiState, ctx: &Ctx, idx: usize) {
    let Some(item) = state.spotify.begin_match_one(idx) else {
        return;
    };
    ctx.emit("spotify-ack", move |app| async move {
        app.emit_async("spotify-match-one", search_one(&app, item).await)
            .await;
        Ok(serde_json::Value::Null)
    });
}

fn spawn_match(ctx: &Ctx, run: Arc<AtomicU64>, job: MatchJob) {
    ctx.emit("spotify-ack", move |app| async move {
        let (concurrency, stagger_ms, legacy) = match job.mode.as_str() {
            "p2" => (2usize, 150u64, false),
            "p3" => (3, 150, false),
            "all" => (job.items.len().max(1), 150, false),
            _ => (1, 0, true),
        };
        let mut iter = job.items.into_iter();
        let mut set = tokio::task::JoinSet::new();
        let spawn = |slot: u64, item: MatchItem, app: &moonhunt_core::AppHandle, set: &mut tokio::task::JoinSet<MatchOneEv>| {
            let app_c = app.clone();
            let delay = stagger_ms * slot;
            set.spawn(async move {
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
                search_one(&app_c, item).await
            });
        };
        for slot in 0..concurrency as u64 {
            let Some(item) = iter.next() else { break };
            spawn(slot, item, &app, &mut set);
        }
        while let Some(res) = set.join_next().await {
            if run.load(Ordering::Acquire) != job.gen {
                set.abort_all();
                break;
            }
            match res {
                Ok(ev) => app.emit_async("spotify-match-one", ev).await,
                Err(e) => {
                    // A panicked/cancelled task would otherwise leave its row
                    // stuck on "Searching" forever.
                    app.emit("yt-log", format!("[spotify] search task failed: {e}"));
                }
            }
            if let Some(item) = iter.next() {
                if legacy {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    if run.load(Ordering::Acquire) != job.gen {
                        set.abort_all();
                        break;
                    }
                }
                spawn(0, item, &app, &mut set);
            }
        }
        app.emit_async("spotify-match-done", DoneEv { gen: job.gen })
            .await;
        Ok(serde_json::Value::Null)
    });
}

async fn search_one(app: &moonhunt_core::AppHandle, item: MatchItem) -> MatchOneEv {
    let started = std::time::Instant::now();
    let query = item.query.clone();
    let (token, idx) = (item.token, item.idx);
    let elapsed_ms = |started: std::time::Instant| started.elapsed().as_millis() as u64;
    match moonhunt_core::addon::addon_yt_search(app.clone(), item.query).await {
        Ok(s) => MatchOneEv {
            token,
            idx,
            query,
            elapsed_ms: elapsed_ms(started),
            url: s.webpage_url,
            title: s.title,
            duration: s.duration,
            error: String::new(),
        },
        Err(e) => MatchOneEv {
            token,
            idx,
            query,
            elapsed_ms: elapsed_ms(started),
            url: String::new(),
            title: String::new(),
            duration: None,
            error: e,
        },
    }
}

fn start_downloads(state: &mut UiState, ctx: &Ctx, only: Option<usize>) {
    let Some(job) = state.spotify.begin_downloads(only) else {
        return;
    };
    ctx.emit("spotify-ack", move |app| async move {
        let mut stop = false;
        for item in job.items {
            if job.batch && (stop || job.run.load(Ordering::Acquire) != job.gen) {
                stop = true;
                app.emit_async(
                    "spotify-dl-one",
                    DlOneEv {
                        token: item.token,
                        idx: item.idx,
                        gid: String::new(),
                        error: String::new(),
                        skipped: true,
                    },
                )
                .await;
                continue;
            }
            let ev = match moonhunt_core::ytdlp::add_yt_download(
                app.clone(),
                item.url,
                job.format.clone(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            {
                Ok(gid) => DlOneEv {
                    token: item.token,
                    idx: item.idx,
                    gid,
                    error: String::new(),
                    skipped: false,
                },
                Err(e) => DlOneEv {
                    token: item.token,
                    idx: item.idx,
                    gid: String::new(),
                    error: e,
                    skipped: false,
                },
            };
            app.emit_async("spotify-dl-one", ev).await;
        }
        if job.batch {
            app.emit_async("spotify-dl-done", DoneEv { gen: job.gen })
                .await;
        }
        Ok(serde_json::Value::Null)
    });
}

fn cancel_from_ui(state: &mut UiState) {
    state.spotify.cancel_downloads();
    let left = state.spotify.ready_count();
    state.toast(
        true,
        format!("Queueing stopped ({left} remaining). Started downloads keep running."),
    );
}

fn clear_list(state: &mut UiState, ctx: &Ctx) {
    let gids = state.spotify.clear();
    if gids.is_empty() {
        return;
    }
    ctx.emit("spotify-ack", move |app| async move {
        let mut stopped = 0u64;
        let mut skipped = 0u64;
        let active: Vec<String> = moonhunt_core::aria2::get_all_downloads()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|d| {
                matches!(
                    d.get("status").and_then(|v| v.as_str()),
                    Some("active" | "waiting" | "paused")
                )
            })
            .filter_map(|d| d.get("gid").and_then(|v| v.as_str()).map(str::to_owned))
            .collect();
        for gid in gids {
            if !active.contains(&gid) {
                skipped += 1;
                continue;
            }
            if moonhunt_core::aria2::remove_download(app.clone(), gid)
                .await
                .is_ok()
            {
                stopped += 1;
            }
        }
        app.emit_async("spotify-cleared", ClearedEv { stopped, skipped })
            .await;
        Ok(serde_json::Value::Null)
    });
}

fn remember_group(state: &mut UiState, gid: &str) {
    let key = if state.spotify.playlist.is_empty() {
        "Spotify playlist".to_string()
    } else {
        state.spotify.playlist.clone()
    };
    let name = state.raw(&key).to_string();
    if state.spotify.group_id.is_empty() {
        state.spotify.group_id = state.group_create(&name, "#1db954", vec![gid.to_string()]);
        state.toast(true, format!("Group created: {name}"));
    } else {
        let id = state.spotify.group_id.clone();
        state.group_add_member(&id, gid);
    }
}

/// Spotify events. Returns `true` when the name belongs to this screen.
pub fn apply_event(state: &mut UiState, name: &str, payload: &serde_json::Value) -> bool {
    match name {
        "spotify-ack" => true,
        "spotify-requirements" => {
            if let Some(ev) = parse::<RequirementsEv>(payload) {
                state.spotify.apply_requirements(&ev);
            }
            true
        }
        "spotify-resolve" => {
            if let Some(ev) = parse::<ResolveEv>(payload) {
                state.spotify.apply_resolve(&ev);
            }
            true
        }
        "spotify-match-one" => {
            if let Some(ev) = parse::<MatchOneEv>(payload) {
                state.spotify.apply_match_one(&ev);
            }
            true
        }
        "spotify-match-done" => {
            if let Some(ev) = parse::<DoneEv>(payload) {
                state.spotify.apply_match_done(ev.gen);
            }
            true
        }
        "spotify-dl-one" => {
            if let Some(ev) = parse::<DlOneEv>(payload) {
                if let Some(gid) = state.spotify.apply_dl_one(&ev) {
                    remember_group(state, &gid);
                }
            }
            true
        }
        "spotify-dl-done" => {
            if let Some(ev) = parse::<DoneEv>(payload) {
                state.spotify.apply_dl_done(ev.gen);
            }
            true
        }
        "spotify-cleared" => {
            if let Some(ev) = parse::<ClearedEv>(payload) {
                let tail = if ev.skipped > 0 {
                    format!(", {} already finished", ev.skipped)
                } else {
                    String::new()
                };
                state.toast(
                    true,
                    format!(
                        "List cleared. Stopped {} running download(s){tail}.",
                        ev.stopped
                    ),
                );
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched(title: &str, artist: &str, url: &str) -> Track {
        let mut t = Track::new(title.to_string(), artist.to_string());
        t.mt = Match::Matched {
            url: url.to_string(),
            title: title.to_string(),
            duration: Some(180),
        };
        t
    }

    #[test]
    fn audio_format_defaults_to_mp3() {
        assert_eq!(SpotifyUi::default().audio_format, "mp3");
    }

    #[test]
    fn search_mode_defaults_to_legacy() {
        assert_eq!(SpotifyUi::default().search_mode, "legacy");
    }

    #[test]
    fn requirements_prefer_selected_system_node() {
        let runtimes = serde_json::json!({
            "runtimes": [{"id": "node", "name": "Node.js", "installed": true, "selected": false}],
            "systemNode": {"version": "22", "path": "node.exe", "selected": true}
        });
        let ev = requirements_ev(false, Some(runtimes), None);
        assert!(ev.runtime_ok);
        assert_eq!(ev.runtime_name, "System Node.js");
        assert!(!ev.ytdlp);
        assert!(!ev.merger_ok);
    }

    #[test]
    fn requirements_name_an_auto_merger() {
        let mergers = serde_json::json!({
            "mergers": [{"id": "ffmpeg", "name": "FFmpeg 7", "installed": true, "selected": false}],
            "systemFfmpeg": null
        });
        let ev = requirements_ev(true, None, Some(mergers));
        assert!(ev.merger_ok);
        assert_eq!(ev.merger_name, "FFmpeg 7 (auto)");
    }

    #[test]
    fn requirements_report_missing_when_nothing_is_installed() {
        let runtimes = serde_json::json!({"runtimes": [], "systemNode": null});
        let mergers = serde_json::json!({"mergers": [], "systemFfmpeg": null});
        let ev = requirements_ev(false, Some(runtimes), Some(mergers));
        assert!(!ev.runtime_ok);
        assert!(!ev.merger_ok);
    }

    #[test]
    fn cancel_match_does_not_retire_the_download() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![
            matched("Song", "Ada", "https://youtu.be/a"),
            Track::new("Nope".into(), "X".into()),
        ];
        let dl = s.begin_downloads(None).expect("batch");
        assert!(s.downloading);
        let _batch = s.begin_match().expect("match");
        assert!(s.matching);
        s.cancel_match();
        assert!(!s.matching);
        assert!(s.downloading);
        s.apply_dl_done(dl.gen);
        assert!(!s.downloading);
    }

    #[test]
    fn resolve_clears_a_live_download_flag() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![matched("Song", "Ada", "https://youtu.be/a")];
        s.begin_downloads(None).expect("batch");
        s.url = "https://open.spotify.com/playlist/abc".into();
        s.begin_resolve().expect("gen");
        assert!(s.resolving);
        assert!(!s.downloading);
        assert!(s.tracks.is_empty());
        s.apply_dl_done(1);
        assert!(!s.downloading);
    }

    #[test]
    fn row_search_does_not_finish_the_batch() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![
            Track::new("A".into(), "a".into()),
            Track::new("B".into(), "b".into()),
        ];
        let batch = s.begin_match().expect("batch");
        s.apply_match_one(&MatchOneEv {
            token: batch.items[1].token,
            idx: 1,
            query: String::new(),
            elapsed_ms: 0,
            url: String::new(),
            title: String::new(),
            duration: None,
            error: "nope".into(),
        });
        let row = s.begin_match_one(1).expect("retry");
        assert!(s.matching);
        assert_ne!(row.token, batch.gen);
        s.apply_match_one(&MatchOneEv {
            token: row.token,
            idx: 1,
            query: String::new(),
            elapsed_ms: 0,
            url: "https://youtu.be/z".into(),
            title: "Hit".into(),
            duration: Some(10),
            error: String::new(),
        });
        assert!(s.matching);
        s.apply_match_one(&MatchOneEv {
            token: batch.items[1].token,
            idx: 1,
            query: String::new(),
            elapsed_ms: 0,
            url: "https://youtu.be/old".into(),
            title: "Old".into(),
            duration: None,
            error: String::new(),
        });
        match &s.tracks[1].mt {
            Match::Matched { url, .. } => assert_eq!(url, "https://youtu.be/z"),
            other => panic!("row was overwritten: {other:?}"),
        }
        s.apply_match_done(batch.gen);
        assert!(!s.matching);
    }

    #[test]
    fn cancel_match_drops_inflight_results_and_resets_the_row() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![Track::new("A".into(), "a".into())];
        let job = s.begin_match().expect("match");
        s.cancel_match();
        assert!(matches!(s.tracks[0].mt, Match::Idle));
        s.apply_match_one(&MatchOneEv {
            token: job.items[0].token,
            idx: 0,
            query: String::new(),
            elapsed_ms: 0,
            url: "https://youtu.be/a".into(),
            title: "A".into(),
            duration: None,
            error: String::new(),
        });
        assert!(matches!(s.tracks[0].mt, Match::Idle));
        s.apply_match_done(job.gen);
        assert!(!s.matching);
    }

    #[test]
    fn wrong_download_token_does_not_mark_the_row_queued() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![matched("Song", "Ada", "https://youtu.be/a")];
        let job = s.begin_downloads(None).expect("batch");
        let gid = s.apply_dl_one(&DlOneEv {
            token: job.items[0].token.wrapping_add(9),
            idx: 0,
            gid: "gid".into(),
            error: String::new(),
            skipped: false,
        });
        assert!(gid.is_none());
        assert!(matches!(s.tracks[0].dl, Download::Starting));
    }

    #[test]
    fn skipped_row_returns_to_idle() {
        let mut s = SpotifyUi::default();
        s.tracks = vec![matched("Song", "Ada", "https://youtu.be/a")];
        let job = s.begin_downloads(None).expect("batch");
        s.apply_dl_one(&DlOneEv {
            token: job.items[0].token,
            idx: 0,
            gid: String::new(),
            error: String::new(),
            skipped: true,
        });
        assert!(matches!(s.tracks[0].dl, Download::Idle));
    }
}
