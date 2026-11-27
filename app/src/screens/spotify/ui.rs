//! Spotify bridge layout. Each block is allocated, so the card is as tall as
//! what it paints. Labels go through `UiState::t`.

use super::{Download, Match, Note, Track};
use crate::download_dialogs::text_input;
use crate::screens::{Ctx, PAGE_GUTTER};
use crate::state::UiState;
use crate::ui::icons as mi;
use crate::ui::widgets as mk;
use crate::rtl::truncate;
use egui::{pos2, vec2, Align2, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};

pub fn show(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, viewport: Rect) {
    state.spotify.load_search_mode();
    if state.spotify.req_ytdlp.is_none() && !state.spotify.req_checking {
        super::start_requirements(state, ctx);
    }
    if state.spotify.take_need_match() {
        super::start_match(state, ctx);
    }
    ui.scope_builder(egui::UiBuilder::new().max_rect(viewport), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("spotify-bridge")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail = ui.available_rect_before_wrap();
                let avail = Rect::from_min_max(
                    avail.min,
                    pos2(avail.max.x - PAGE_GUTTER, avail.max.y),
                );
                let w = 720.0_f32.min(avail.width() - 64.0).max(240.0);
                let x = avail.min.x + 32.0 + ((avail.width() - 64.0 - w) * 0.5).max(0.0);
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_size(pos2(x, avail.min.y + 24.0), vec2(w, 10.0))),
                    |ui| {
                        ui.set_width(w);
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        card(state, ctx, ui, w);
                        if !state.spotify.tracks.is_empty() {
                            ui.add_space(14.0);
                            list_card(state, ctx, ui, w);
                        }
                        ui.add_space(16.0);
                    },
                );
            });
    });
}

fn card(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, w: f32) {
    let p = mk::pal();
    egui::Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(w - 40.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            title_block(
                ui,
                "library_music",
                state.t("Spotify Playlist Bridge"),
                state.t("Resolve Spotify links to YouTube matches and download tracks."),
                Some(state.t("Native host")),
            );
            ui.add_space(12.0);
            req_strip(state, ctx, ui);
            ui.add_space(16.0);
            line(ui, state.t("SPOTIFY LINK"), semi(11.0), p.text_muted, 16.0);
            url_row(state, ctx, ui);
            ui.add_space(10.0);
            action_row(state, ctx, ui);
            ui.add_space(8.0);
            proxy_line(state, ctx, ui);
            if !matches!(state.spotify.note, Note::Empty) {
                ui.add_space(12.0);
                let text = note_text(state);
                line(ui, &text, prop(11.0), p.text_dim, 16.0);
            }
            if !state.spotify.error.is_empty() {
                ui.add_space(4.0);
                let err = state.spotify.error.clone();
                let shown = state.t(&err).to_string();
                line(ui, &shown, prop(11.0), p.status_error, 16.0);
            }
            ui.add_space(10.0);
            search_mode_row(state, ui);
            ui.add_space(10.0);
            manual_block(state, ctx, ui);
            ui.add_space(8.0);
            logs_block(state, ui);
        });
}

fn url_row(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let (field, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
    let fid = egui::Id::new("sp-link");
    let focused = ui.ctx().memory(|m| m.has_focus(fid));
    ui.painter()
        .rect_filled(field, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        field,
        CornerRadius::same(8),
        Stroke::new(1.0, if focused { p.accent_strong } else { p.border }),
        StrokeKind::Inside,
    );
    text_input(
        ui,
        fid,
        field.shrink2(vec2(10.0, 0.0)),
        &mut state.spotify.url,
        "https://open.spotify.com/playlist/…",
        prop(13.0),
        0,
        1.0,
    );
    if focused
        && ui.input(|i| i.key_pressed(egui::Key::Enter))
        && !state.spotify.resolving
    {
        super::start_resolve(state, ctx);
    }
    if !state.spotify.url.is_empty() {
        let clr = Rect::from_min_size(
            pos2(field.max.x - 28.0, field.center().y - 10.0),
            vec2(20.0, 20.0),
        );
        ui.painter()
            .text(clr.center(), Align2::CENTER_CENTER, "×", prop(14.0), p.text_dim);
        if ui.interact(clr, egui::Id::new("sp-link-clear"), Sense::click()).clicked() {
            state.spotify.url.clear();
        }
    }
}

fn action_row(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let scanning = state.spotify.resolving;
    let label = if scanning {
        state.t("Resolving…")
    } else {
        state.t("Resolve tracks")
    };
    let rb = Rect::from_min_size(row.min, vec2(130.0, 32.0));
    let can_resolve = !scanning && !state.spotify.url.trim().is_empty();
    if button(ui, "sp-resolve", rb, label, true, can_resolve) {
        super::start_resolve(state, ctx);
    }
    let seg = Rect::from_min_size(pos2(rb.max.x + 8.0, row.min.y), vec2(78.0, 32.0));
    ui.painter()
        .rect_filled(seg, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        seg,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
    for (i, opt) in ["mp3", "mp4"].iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(seg.min.x + 39.0 * i as f32, seg.min.y),
            vec2(39.0, 32.0),
        );
        let on = state.spotify.audio_format == *opt;
        if on {
            ui.painter().rect_filled(
                cell.shrink(1.0),
                CornerRadius::same(7),
                p.accent.gamma_multiply(0.12),
            );
        }
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            opt.to_uppercase(),
            semi(11.0),
            if on { p.accent_strong } else { p.text_dim },
        );
        if ui
            .interact(cell, egui::Id::new("sp-fmt").with(opt), Sense::click())
            .clicked()
        {
            state.spotify.audio_format = (*opt).to_string();
        }
    }
    let ax = seg.max.x + 8.0;
    if state.spotify.downloading {
        let queued = state
            .spotify
            .tracks
            .iter()
            .filter(|t| matches!(t.dl, Download::Queued(_)))
            .count();
        let total = state.spotify.matched();
        let pct = if total > 0 {
            (queued as f32 / total as f32 * 100.0).round() as i32
        } else {
            0
        };
        queue_ring(
            ui,
            pos2(ax + 11.0, row.center().y),
            pct,
            p.accent_strong,
            p.border,
        );
        let qtxt = fill(
            state,
            "Queuing {done}/{total}",
            &[("done", &queued.to_string()), ("total", &total.to_string())],
        );
        let qw = ui
            .painter()
            .layout_no_wrap(qtxt.clone(), prop(12.0), p.text_dim)
            .rect
            .width();
        let text_x = ax + 30.0;
        ui.painter().text(
            pos2(text_x, row.center().y),
            Align2::LEFT_CENTER,
            &qtxt,
            prop(12.0),
            p.text_dim,
        );
        let cb = Rect::from_min_size(pos2(text_x + qw + 12.0, row.min.y), vec2(86.0, 32.0));
        if button(ui, "sp-dl-cancel", cb, state.t("Cancel"), false, true) {
            super::cancel_from_ui(state);
        }
    } else {
        let ready = state.spotify.ready_count();
        let label = fill(state, "Download all ({n})", &[("n", &ready.to_string())]);
        let dl = Rect::from_min_size(pos2(ax, row.min.y), vec2(150.0, 32.0));
        if button(ui, "sp-dl-all", dl, &label, false, ready > 0) {
            super::start_downloads(state, ctx, None);
        }
    }
}

/// Media-page prerequisites as pills. Names the effective JS runtime and
/// merger; a missing piece gets a hint instead of blocking the screen.
fn req_strip(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let font = semi(11.0);
    let checking = state.spotify.req_checking;
    let missing = [state.spotify.req_ytdlp, state.spotify.req_runtime, state.spotify.req_merger]
        .iter()
        .any(|o| *o == Some(false));
    let specs: [(String, Option<bool>, String); 3] = [
        (
            state.t("yt-dlp").to_string(),
            state.spotify.req_ytdlp,
            String::new(),
        ),
        (
            state.t("JS runtime").to_string(),
            state.spotify.req_runtime,
            state.spotify.req_runtime_name.clone(),
        ),
        (
            state.t("Merger (MP3)").to_string(),
            state.spotify.req_merger,
            state.spotify.req_merger_name.clone(),
        ),
    ];
    let h = if missing { 40.0 } else { 24.0 };
    let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
    let mut x = row.min.x;
    for (label, ok, name) in specs {
        let text = if name.is_empty() {
            label
        } else {
            truncate(&name, |s| {
                ui.painter()
                    .layout_no_wrap(s.to_owned(), font.clone(), p.text)
                    .rect
                    .width()
                    <= 150.0
            })
        };
        let tw = ui
            .painter()
            .layout_no_wrap(text.clone(), font.clone(), p.text)
            .rect
            .width();
        let w = 10.0 + 14.0 + 5.0 + tw + 10.0;
        if x + w > row.max.x && x > row.min.x {
            break;
        }
        let pill = Rect::from_min_size(pos2(x, row.min.y), vec2(w, 24.0));
        let (color, icon) = if checking {
            (p.text_dim, "hourglass_top")
        } else {
            match ok {
                Some(true) => (p.accent_strong, "check_circle"),
                Some(false) => (p.status_error, "error_outline"),
                None => (p.text_dim, "help_outline"),
            }
        };
        ui.painter()
            .rect_filled(pill, CornerRadius::same(12), p.hover_veil);
        ui.painter().rect_stroke(
            pill,
            CornerRadius::same(12),
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(pill.min.x + 17.0, pill.center().y),
            icon,
            14.0,
            color,
        );
        ui.painter().text(
            pos2(pill.min.x + 29.0, pill.center().y),
            Align2::LEFT_CENTER,
            text,
            font.clone(),
            color,
        );
        x = pill.max.x + 6.0;
    }
    if missing {
        let hint = state.t("Missing pieces are set up on the Media page.");
        ui.painter().text(
            pos2(row.min.x, row.min.y + 26.0),
            Align2::LEFT_TOP,
            hint,
            prop(11.0),
            p.text_dim,
        );
    }
    if resp.clicked() && !checking {
        super::start_requirements(state, ctx);
    }
}

/// Legacy / 2x / 3x / All, persisted under `spm-search-mode`.
fn search_mode_row(state: &mut UiState, ui: &mut Ui) {
    let p = mk::pal();
    line(ui, state.t("SEARCH MODE"), semi(11.0), p.text_muted, 16.0);
    ui.add_space(6.0);
    let modes = [
        (
            "legacy",
            "Legacy",
            "Sequential, one at a time. Recommended for restricted networks.",
        ),
        ("p2", "2x", "Two parallel searches."),
        ("p3", "3x", "Three parallel searches."),
        (
            "all",
            "All",
            "Everything at once. Experimental — may hit YouTube rate limits.",
        ),
    ];
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let seg = Rect::from_min_size(row.min, vec2(4.0 * 64.0, 30.0));
    ui.painter()
        .rect_filled(seg, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        seg,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
    for (i, (key, label, _)) in modes.iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(seg.min.x + 64.0 * i as f32, seg.min.y),
            vec2(64.0, 30.0),
        );
        let on = state.spotify.search_mode.as_str() == *key;
        if on {
            ui.painter().rect_filled(
                cell.shrink(1.0),
                CornerRadius::same(7),
                p.accent.gamma_multiply(0.12),
            );
        }
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            *label,
            semi(11.0),
            if on { p.accent_strong } else { p.text_dim },
        );
        if ui
            .interact(cell, egui::Id::new("sp-mode").with(*key), Sense::click())
            .clicked()
        {
            state.spotify.set_search_mode(key);
        }
    }
    ui.add_space(4.0);
    let hint = modes
        .iter()
        .find(|(k, _, _)| *k == state.spotify.search_mode.as_str())
        .map(|(_, _, h)| *h)
        .unwrap_or("");
    line(ui, hint, prop(11.0), p.text_dim, 16.0);
}

/// Upstream's 40×40 queue ring, scaled to the action row.
fn queue_ring(
    ui: &Ui,
    center: egui::Pos2,
    pct: i32,
    color: egui::Color32,
    track: egui::Color32,
) {
    let r = 10.0;
    ui.painter()
        .circle_stroke(center, r, Stroke::new(2.5, track));
    let frac = (pct.clamp(0, 100) as f32) / 100.0;
    if frac > 0.0 {
        let n = 24usize;
        let start = -std::f32::consts::FRAC_PI_2;
        let sweep = frac * std::f32::consts::TAU;
        let pts: Vec<egui::Pos2> = (0..=n)
            .map(|i| {
                let a = start + sweep * (i as f32 / n as f32);
                center + vec2(r * a.cos(), r * a.sin())
            })
            .collect();
        ui.painter()
            .add(egui::Shape::line(pts, Stroke::new(2.5, color)));
    }
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        format!("{}%", pct.clamp(0, 100)),
        prop(8.0),
        color,
    );
}

/// Which proxy the bridge's YouTube calls will use, and whether it reaches
/// YouTube. Same test (and same result slot) as Settings → Network, so the
/// answer here is the answer there. Click to re-test.
fn proxy_line(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let s = &state.settings;
    let configured = s.proxy_type != "none" && !s.proxy_host.is_empty();
    let target = if configured {
        format!("{}://{}:{}", s.proxy_type, s.proxy_host, s.proxy_port)
    } else {
        state.t("Direct connection (no proxy)").to_string()
    };
    let (color, verdict) = match (&state.st_test_yt, state.st_testing_yt) {
        (_, true) => (p.text_dim, state.t("Testing…").to_string()),
        (Some((true, msg)), _) => (p.status_complete, msg.clone()),
        (Some((false, msg)), _) => (p.status_error, msg.clone()),
        (None, _) => (
            p.text_dim,
            if configured {
                state.t("Check proxy").to_string()
            } else {
                String::new()
            },
        ),
    };
    let text = if verdict.is_empty() {
        format!("Proxy · {}", target)
    } else {
        format!("Proxy · {} — {}", target, verdict)
    };
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::click());
    let dot = pos2(r.min.x + 4.0, r.center().y);
    ui.painter()
        .circle_filled(dot, 3.5, if configured { color } else { p.text_muted });
    ui.painter().text(
        pos2(r.min.x + 14.0, r.center().y),
        Align2::LEFT_CENTER,
        &text,
        prop(11.0),
        color,
    );
    if configured && resp.clicked() && !state.st_testing_yt {
        state.st_testing_yt = true;
        state.st_test_yt = None;
        ctx.emit("settings-test-yt", |app| async move {
            moonhunt_core::aria2::test_proxy_url(app, "https://www.youtube.com/".to_string(), None)
                .await
        });
    }
}

fn manual_block(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let label = state.t("Paste tracks manually");
    let icon = if state.spotify.show_manual {
        "expand_more"
    } else {
        "chevron_right"
    };
    let (link, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::click());
    mi::paint_centered(
        ui.painter(),
        pos2(link.min.x + 7.0, link.center().y),
        icon,
        14.0,
        p.accent_strong,
    );
    ui.painter().text(
        pos2(link.min.x + 18.0, link.center().y),
        Align2::LEFT_CENTER,
        label,
        prop(11.0),
        p.accent_strong,
    );
    if resp.clicked() {
        state.spotify.show_manual = !state.spotify.show_manual;
    }
    if !state.spotify.show_manual {
        return;
    }
    ui.add_space(8.0);
    let (box_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), Sense::hover());
    ui.painter()
        .rect_filled(box_rect, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        box_rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let hint = state.t("Artist - Title").to_string();
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(box_rect.shrink(8.0))
            .id(egui::Id::new("sp-manual-scope")),
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.spotify.manual)
                    .id(egui::Id::new("sp-manual"))
                    .desired_width(f32::INFINITY)
                    .desired_rows(4)
                    .font(prop(12.0))
                    .text_color(p.text)
                    .hint_text(hint)
                    .frame(egui::Frame::NONE),
            );
        },
    );
    ui.add_space(18.0);
    let (use_rect, _) = ui.allocate_exact_size(vec2(84.0, 30.0), Sense::hover());
    if button(ui, "sp-use-list", use_rect, state.t("Use list"), true, true) {
        super::start_manual(state, ctx);
    }
}

fn logs_block(state: &mut UiState, ui: &mut Ui) {
    let p = mk::pal();
    let label = state.t("Search log");
    let icon = if state.spotify.show_logs {
        "expand_more"
    } else {
        "chevron_right"
    };
    let (link, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::click());
    mi::paint_centered(
        ui.painter(),
        pos2(link.min.x + 7.0, link.center().y),
        icon,
        14.0,
        p.text_muted,
    );
    ui.painter().text(
        pos2(link.min.x + 18.0, link.center().y),
        Align2::LEFT_CENTER,
        label,
        prop(11.0),
        p.text_muted,
    );
    if resp.clicked() {
        state.spotify.show_logs = !state.spotify.show_logs;
    }
    if !state.spotify.show_logs {
        return;
    }
    ui.add_space(8.0);
    let (box_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 160.0), Sense::hover());
    ui.painter()
        .rect_filled(box_rect, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        box_rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let empty_hint = state.t("No log entries yet.").to_string();
    let mut copy_text = String::new();
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(box_rect.shrink(8.0))
            .id(egui::Id::new("sp-logs-scope")),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("sp-logs-scroll")
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if state.spotify.logs.is_empty() {
                        ui.label(
                            egui::RichText::new(&empty_hint)
                                .font(prop(11.0))
                                .color(p.text_dim),
                        );
                    } else {
                        for line in &state.spotify.logs {
                            ui.label(
                                egui::RichText::new(line.as_str())
                                    .font(mono(11.0))
                                    .color(p.text_dim),
                            );
                            copy_text.push_str(line);
                            copy_text.push('\n');
                        }
                    }
                });
        },
    );
    ui.add_space(8.0);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let copy_rect = Rect::from_min_size(row.min, vec2(64.0, 26.0));
    let clear_rect = Rect::from_min_size(pos2(row.min.x + 72.0, row.min.y), vec2(64.0, 26.0));
    let has_logs = !state.spotify.logs.is_empty();
    if button(ui, "sp-logs-copy", copy_rect, state.t("Copy"), false, has_logs) {
        ui.ctx().copy_text(copy_text);
        let msg = state.t("Log copied").to_string();
        state.toast(true, msg);
    }
    if button(ui, "sp-logs-clear", clear_rect, state.t("Clear"), false, has_logs) {
        state.spotify.logs.clear();
    }
}

fn list_card(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, w: f32) {
    let p = mk::pal();
    egui::Frame::new()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(w - 24.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let n = state.spotify.tracks.len();
            let matched = state.spotify.matched();
            let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
            ui.painter().text(
                head.left_top(),
                Align2::LEFT_TOP,
                fill(state, "Tracks ({n})", &[("n", &n.to_string())]),
                semi(13.0),
                p.text,
            );
            ui.painter().text(
                pos2(head.min.x, head.min.y + 18.0),
                Align2::LEFT_TOP,
                fill(
                    state,
                    "{matched} / {n} matched",
                    &[("matched", &matched.to_string()), ("n", &n.to_string())],
                ),
                prop(11.0),
                p.text_dim,
            );
            let clear = Rect::from_min_size(
                pos2(head.max.x - 76.0, head.min.y),
                vec2(76.0, 28.0),
            );
            let cleared = button(ui, "sp-clear", clear, state.t("Clear"), false, true);
            if !state.spotify.matching {
                let unmatched = state
                    .spotify
                    .tracks
                    .iter()
                    .filter(|t| matches!(t.mt, Match::Idle | Match::Failed(_)))
                    .count();
                let label = fill(state, "Match all ({n})", &[("n", &unmatched.to_string())]);
                let m = Rect::from_min_size(pos2(clear.min.x - 8.0 - 130.0, clear.min.y), vec2(130.0, 28.0));
                if button(ui, "sp-match-all", m, &label, true, unmatched > 0) {
                    super::start_match(state, ctx);
                }
            } else {
                let m = Rect::from_min_size(pos2(clear.min.x - 8.0 - 84.0, clear.min.y), vec2(84.0, 28.0));
                if button(ui, "sp-match-cancel", m, state.t("Cancel"), false, true) {
                    state.spotify.cancel_match();
                }
            }
            if cleared {
                super::clear_list(state, ctx);
            }
            if state.spotify.tracks.is_empty() {
                return;
            }
            ui.add_space(8.0);
            let (rule, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter()
                .rect_filled(rule, CornerRadius::ZERO, p.border);
            ui.add_space(8.0);
            let mut i = 0;
            while i < state.spotify.tracks.len() {
                row(state, ctx, ui, i);
                i += 1;
                if i < state.spotify.tracks.len() {
                    ui.add_space(8.0);
                }
            }
        });
}

fn row(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, i: usize) {
    let p = mk::pal();
    let Some(t) = state.spotify.tracks.get(i).cloned() else {
        return;
    };
    let h = if matches!(t.mt, Match::Idle) { 46.0 } else { 62.0 };
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    ui.painter()
        .rect_filled(r, CornerRadius::same(10), p.bg_card);
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    ui.painter().text(
        pos2(r.min.x + 4.0, r.center().y),
        Align2::LEFT_CENTER,
        (i + 1).to_string(),
        FontId::monospace(11.0),
        p.text_dim,
    );
    let text_x = r.min.x + 28.0;
    let max_w = (r.max.x - text_x - 96.0).max(60.0);
    let fit = |txt: &str, font: FontId, color: egui::Color32| {
        truncate(txt, |s| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), font.clone(), color)
                .rect
                .width()
                <= max_w
        })
    };
    let title = fit(&t.title, semi(12.5), p.text);
    ui.painter().text(
        pos2(text_x, r.min.y + 8.0),
        Align2::LEFT_TOP,
        title,
        semi(12.5),
        p.text,
    );
    ui.painter().text(
        pos2(text_x, r.min.y + 24.0),
        Align2::LEFT_TOP,
        fit(&t.artist, prop(11.0), p.text_dim),
        prop(11.0),
        p.text_dim,
    );
    let sub = pos2(text_x, r.min.y + 40.0);
    match &t.mt {
        Match::Matched { title, duration, .. } => {
            let dur = duration
                .map(|d| format!(" · {}", fmt_dur(d)))
                .unwrap_or_default();
            mi::paint_centered(
                ui.painter(),
                pos2(sub.x + 5.5, sub.y + 7.0),
                "check",
                11.0,
                p.accent_strong,
            );
            let line = fit(&format!("{title}{dur}"), prop(11.0), p.accent_strong);
            ui.painter().text(
                pos2(sub.x + 13.0, sub.y),
                Align2::LEFT_TOP,
                line,
                prop(11.0),
                p.accent_strong,
            );
        }
        Match::Searching => {
            ui.painter().text(
                sub,
                Align2::LEFT_TOP,
                state.t("Searching YouTube…"),
                prop(11.0),
                p.text_dim,
            );
        }
        Match::Failed(err) => {
            let msg = fill(state, "No match ({err})", &[("err", err)]);
            ui.painter().text(
                sub,
                Align2::LEFT_TOP,
                fit(&msg, prop(11.0), p.status_error),
                prop(11.0),
                p.status_error,
            );
        }
        Match::Idle => {}
    }
    let (lbl, enabled, primary) = row_action(state, &t);
    let br = Rect::from_min_size(
        pos2(r.max.x - 14.0 - 78.0, r.center().y - 13.0),
        vec2(78.0, 26.0),
    );
    if button(ui, &format!("sp-row-{i}"), br, &lbl, primary, enabled) {
        if t.matched_url().is_some() {
            super::start_downloads(state, ctx, Some(i));
        } else {
            super::start_match_one(state, ctx, i);
        }
    }
}

fn row_action(state: &UiState, t: &Track) -> (String, bool, bool) {
    if let Match::Matched { .. } = t.mt {
        let (label, enabled) = match &t.dl {
            Download::Starting => (state.t("…"), false),
            Download::Queued(_) => (state.t("queued"), false),
            Download::Failed(_) => (state.t("Retry"), true),
            Download::Idle => (state.t("Download"), true),
        };
        (label.to_string(), enabled, true)
    } else {
        (state.t("Search").to_string(), !matches!(t.mt, Match::Searching), false)
    }
}

fn title_block(ui: &mut Ui, icon: &str, title: &str, sub: &str, badge: Option<&str>) {
    let p = mk::pal();
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
    let ib = Rect::from_min_size(r.min, vec2(44.0, 44.0));
    ui.painter()
        .rect_filled(ib, CornerRadius::same(12), p.accent.gamma_multiply(0.12));
    mi::paint_centered(ui.painter(), ib.center(), icon, 24.0, p.accent_strong);
    ui.painter().text(
        pos2(ib.max.x + 14.0, ib.min.y + 2.0),
        Align2::LEFT_TOP,
        title,
        semi(15.0),
        p.text,
    );
    if let Some(badge) = badge {
        let bw = ui
            .painter()
            .layout_no_wrap(badge.to_string(), semi(10.0), p.accent_strong)
            .rect
            .width()
            + 18.0;
        let br = Rect::from_min_size(pos2(r.max.x - bw, r.min.y + 3.0), vec2(bw, 18.0));
        ui.painter()
            .rect_filled(br, CornerRadius::same(9), p.accent.gamma_multiply(0.12));
        ui.painter().rect_stroke(
            br,
            CornerRadius::same(9),
            Stroke::new(1.0, p.accent.gamma_multiply(0.4)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            br.center(),
            Align2::CENTER_CENTER,
            badge,
            semi(10.0),
            p.accent_strong,
        );
    }
    ui.painter().text(
        pos2(ib.max.x + 14.0, ib.min.y + 24.0),
        Align2::LEFT_TOP,
        sub,
        prop(12.0),
        p.text_dim,
    );
}

fn line(ui: &mut Ui, text: &str, font: FontId, color: egui::Color32, h: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    ui.painter()
        .text(r.left_top(), Align2::LEFT_TOP, text, font, color);
}

fn button(ui: &mut Ui, salt: &str, rect: Rect, label: &str, primary: bool, enabled: bool) -> bool {
    let p = mk::pal();
    let fill = if primary {
        p.accent.gamma_multiply(if enabled { 0.18 } else { 0.07 })
    } else {
        p.bg_card_inset
    };
    let color = if !enabled {
        p.text_dim
    } else if primary {
        p.accent
    } else {
        p.text
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(
            1.0,
            if primary {
                p.accent.gamma_multiply(0.4)
            } else {
                p.border
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, semi(12.0), color);
    ui.interact(rect, ui.id().with(salt), Sense::click()).clicked() && enabled
}

fn note_text(state: &UiState) -> String {
    match &state.spotify.note {
        Note::Empty => String::new(),
        Note::OembedOne => state.t("Resolved 1 track via Spotify oEmbed.").to_string(),
        Note::Scraped(n) => fill(
            state,
            "Resolved {n} tracks (scraped embed data).",
            &[("n", &n.to_string())],
        ),
        Note::Named(n, name) => fill(
            state,
            "Resolved {n} tracks from \"{name}\" (scraped embed data).",
            &[("n", &n.to_string()), ("name", name)],
        ),
    }
}

fn fill(state: &UiState, key: &str, pairs: &[(&str, &str)]) -> String {
    let mut text = state.raw(key).to_string();
    for (k, v) in pairs {
        text = text.replace(&format!("{{{k}}}"), v);
    }
    state.t_of(&text)
}

fn fmt_dur(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn semi(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::semibold_family())
}

fn prop(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

fn mono(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Monospace)
}
