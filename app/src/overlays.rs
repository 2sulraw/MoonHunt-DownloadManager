//! Window-level overlays ported from `CloseDialog.vue` and
//! `ClipboardToast.vue`: the close-confirmation card (title-bar X / OS close)
//! and the "Download link detected" stack that the clipboard listener feeds.

use crate::screens::Ctx;
use crate::state::{AboutCheck, UiState};
use crate::ui::icons as mi;
use crate::ui::tokens;
use crate::ui::widgets as mk;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Rect, Sense, Stroke, StrokeKind};

fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8)
}

fn bold(size: f32) -> FontId {
    FontId::new(size, tokens::typography::bold_family())
}

fn semi(size: f32) -> FontId {
    FontId::new(size, tokens::typography::semibold_family())
}

fn prop(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

fn text_w(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width()
}

fn ellipsize(ui: &egui::Ui, text: &str, font: &FontId, max_w: f32) -> String {
    crate::rtl::truncate(text, |s| text_w(ui, s, font) <= max_w)
}

// ── CloseDialog ─────────────────────────────────────────────────────────

/// `CloseDialog.vue`: hero + tray/quit options + cancel footer.
pub fn close_dialog(state: &mut UiState, ctx: &egui::Context) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let w = 440.0_f32.min((screen.width() - 40.0).max(280.0));
    let mut stay = false;
    let mut quit = false;
    let mut close = false;

    // Overlay click closes (`@mouseup.self`); the card stops propagation.
    let card_h_guess = 380.0;
    let card_guess = Rect::from_center_size(screen.center(), vec2(w, card_h_guess));

    egui::Area::new(egui::Id::new("close-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(140));
            let clicked = ui.allocate_rect(screen, Sense::click()).clicked();
            let over_card = ui
                .ctx()
                .pointer_interact_pos()
                .is_some_and(|m| card_guess.contains(m));
            if clicked && !over_card {
                close = true;
            }
        });

    egui::Area::new(egui::Id::new("close-dialog"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Reserve the bottom shape slot for `var(--shadow-lg)` — the shadow
            // is a stack of filled rects, so it must be painted *under* the
            // card fill or it darkens the card itself.
            let under = ui.painter().add(egui::Shape::Noop);
            let mut card = Rect::NOTHING;
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(18))
                .stroke(Stroke::new(1.0, p.border_strong))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    card = ui.max_rect();

                    // ── close-x (30×30 at right 12, top 12) ────────────
                    let xr = Rect::from_min_size(
                        pos2(card.max.x - 12.0 - 30.0, card.min.y + 12.0),
                        vec2(30.0, 30.0),
                    );
                    let xresp = ui.interact(xr, egui::Id::new("close-dialog-x"), Sense::click());
                    let xhov = xresp.hovered();
                    ui.painter().rect_filled(
                        xr,
                        CornerRadius::same(8),
                        if xhov {
                            with_alpha(p.status_error, 0.12)
                        } else {
                            p.hover_veil
                        },
                    );
                    mi::paint_centered(
                        ui.painter(),
                        xr.center(),
                        "close",
                        16.0,
                        if xhov { p.status_error } else { p.text_dim },
                    );
                    if xresp.clicked() {
                        close = true;
                    }

                    // ── hero (padding 30 26 18, centered) ──────────────
                    // CSS line boxes: h2 `19px` inherits `body { line-height:
                    // 1.45 }` → 27.55; `.close-hero p { line-height: 1.55 }`
                    // → 19.375 per line.
                    const H2_LINE: f32 = 27.55;
                    const P_LINE: f32 = 19.375;
                    let p_text = state.t(
                        state.t("MoonHunt can keep running in the system tray so your downloads continue in the background."),
                    );
                    let mut p_job = egui::text::LayoutJob::single_section(
                        p_text.to_owned(),
                        egui::TextFormat {
                            font_id: prop(12.5),
                            color: p.text_muted,
                            line_height: Some(P_LINE),
                            ..Default::default()
                        },
                    );
                    p_job.wrap.max_width = 340.0;
                    let galley = ui.fonts_mut(|f| f.layout_job(p_job));
                    let hero_h = 30.0 + 52.0 + 10.0 + H2_LINE + 7.0 + galley.size().y + 18.0;
                    let (hero, _) = ui.allocate_exact_size(vec2(w, hero_h), Sense::hover());
                    let cx = hero.center().x;
                    let circle = pos2(cx, hero.min.y + 30.0 + 26.0);
                    ui.painter()
                        .circle_filled(circle, 26.0, with_alpha(p.accent, 0.09));
                    ui.painter()
                        .circle_stroke(circle, 25.5, Stroke::new(1.0, with_alpha(p.accent, 0.18)));
                    mi::paint_centered(ui.painter(), circle, "logout", 24.0, p.accent_strong);
                    let mut h_job = egui::text::LayoutJob::single_section(
                        state.t("Close MoonHunt?").to_owned(),
                        egui::TextFormat {
                            font_id: bold(19.0),
                            color: p.text,
                            line_height: Some(H2_LINE),
                            ..Default::default()
                        },
                    );
                    h_job.wrap.max_width = w - 52.0;
                    let h2 = ui.fonts_mut(|f| f.layout_job(h_job));
                    let h2_y = hero.min.y + 30.0 + 52.0 + 10.0;
                    ui.painter().galley(pos2(cx - h2.size().x * 0.5, h2_y), h2, p.text);
                    ui.painter().galley(
                        pos2(cx - galley.size().x * 0.5, h2_y + H2_LINE + 7.0),
                        galley,
                        p.text_muted,
                    );

                    // ── options (padding 4 18 0, gap 8) ────────────────
                    ui.add_space(4.0);
                    stay = close_option(
                        ui,
                        &p,
                        w,
                        "close-tray",
                        "minimize",
                        state.t("Stay in system tray"),
                        state.t("Hide to the Windows tray and keep downloading."),
                        OptKind::Accent,
                    );
                    ui.add_space(8.0);
                    quit = close_option(
                        ui,
                        &p,
                        w,
                        "close-quit",
                        "power_settings_new",
                        state.t("Exit completely"),
                        state.t("Close the app and stop all downloads."),
                        OptKind::Danger,
                    );

                    // ── footer (padding 12 18 18) ──────────────────────
                    ui.add_space(12.0);
                    let (fr, _) = ui.allocate_exact_size(vec2(w, 33.0), Sense::hover());
                    let cf = semi(12.0);
                    let cancel_label = state.t("Cancel");
                    let cw = text_w(ui, cancel_label, &cf);
                    let cr = Rect::from_center_size(fr.center(), vec2(cw + 32.0, 30.0));
                    let cresp = ui.interact(cr, egui::Id::new("close-dialog-cancel"), Sense::click());
                    let chov = cresp.hovered();
                    if chov {
                        ui.painter().rect_filled(cr, CornerRadius::same(8), p.hover_veil);
                    }
                    ui.painter().text(
                        cr.center(),
                        Align2::CENTER_CENTER,
                        cancel_label,
                        cf,
                        if chov { p.text } else { p.text_dim },
                    );
                    if cresp.clicked() {
                        close = true;
                    }
                    ui.add_space(18.0);
                });
            ui.painter().set(
                under,
                egui::Shape::Vec(crate::ui::shade::soft_shadow_shapes(
                    card,
                    18,
                    Color32::BLACK,
                    8.0,
                    32.0,
                    115,
                )),
            );
        });

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if quit {
        state.request_exit = true;
    } else if stay {
        // `hide_to_tray` has no tray icon yet (no tray integration) — minimize
        // so the window stays recoverable from the taskbar.
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        close = true;
    }
    if close {
        state.show_close_dialog = false;
    }
}

#[derive(Clone, Copy, PartialEq)]
enum OptKind {
    Accent,
    Danger,
}

/// One `.close-option` row: icon tile + title/description + chevron.
fn close_option(
    ui: &mut egui::Ui,
    p: &crate::ui::tokens::Palette,
    w: f32,
    id: &str,
    icon: &str,
    title: &str,
    desc: &str,
    kind: OptKind,
) -> bool {
    let (r, _) = ui.allocate_exact_size(vec2(w, 62.0), Sense::hover());
    let rect = Rect::from_min_size(pos2(r.min.x + 18.0, r.min.y), vec2(w - 36.0, 62.0));
    let resp = ui.interact(rect, egui::Id::new(id), Sense::click());
    let hov = resp.hovered();
    let (tint, border) = match kind {
                        OptKind::Accent => (p.accent, p.accent),
                        OptKind::Danger => (p.status_error, p.status_error),
    };
    let rect = if hov { rect.translate(vec2(0.0, -1.0)) } else { rect };
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(13),
        if hov {
            with_alpha(tint, 0.07)
        } else {
            p.hover_veil
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(13),
        Stroke::new(1.0, if hov { with_alpha(border, 0.3) } else { p.border }),
        StrokeKind::Inside,
    );
    let tile = Rect::from_min_size(pos2(rect.min.x + 13.0, rect.center().y - 19.0), vec2(38.0, 38.0));
    let (tile_bg, tile_fg) = match kind {
        OptKind::Accent => (with_alpha(p.accent, 0.08), p.accent_strong),
        OptKind::Danger => (with_alpha(p.status_error, 0.09), p.status_error),
    };
    ui.painter().rect_filled(tile, CornerRadius::same(10), tile_bg);
    mi::paint_centered(ui.painter(), tile.center(), icon, 20.0, tile_fg);

    let tx = tile.max.x + 12.0;
    let strong = semi(13.0);
    let small = prop(11.0);
    let sh = ui.fonts_mut(|f| f.row_height(&strong));
    let dh = ui.fonts_mut(|f| f.row_height(&small));
    let top = rect.center().y - (sh + 2.0 + dh) * 0.5;
    ui.painter().text(pos2(tx, top), Align2::LEFT_TOP, title, strong, p.text);
    ui.painter().text(
        pos2(tx, top + sh + 2.0),
        Align2::LEFT_TOP,
        desc,
        small,
        p.text_dim,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.max.x - 13.0 - 9.0, rect.center().y),
        "chevron_right",
        18.0,
        p.text_dim,
    );
    resp.clicked()
}

// ── AboutModal ──────────────────────────────────────────────────────────

/// Text width without a `Ui` (layout happens on the shared font atlas).
fn tw(ctx: &egui::Context, text: &str, font: &FontId) -> f32 {
    ctx.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE))
        .size()
        .x
}

/// `AboutModal.vue` — 400px card: hero (logo ring, name, version pill,
/// description, tech chips), GitHub row, update-check row and the footer.
///
/// shortcut: the overlay's `backdrop-filter: blur(4px)` is skipped (egui has no
/// cheap backdrop blur) and the GitHub mark (an ionicons SVG) is drawn with the
/// `code` glyph, since Material Icons has no GitHub logo.
pub fn about_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    // `width: min(400px, 90vw)`.
    let w = 400.0_f32.min(screen.width() * 0.9);
    let card_w = w - 34.0; // border 1 + `.ab-github/.ab-update` padding 16
    let version = format!("v{}", moonhunt_core::APP_VERSION);
    let desc = state.t(
        state.t("A polished desktop download manager built for speed, structure, and a calm experience."),
    );
    let desc_g = galley(ctx, desc, prop(12.5), p.text_muted, 19.375, 300.0);

    // ── the update row's inline status (`v-if` chain) ─────────────────
    let checking = state.about_check == AboutCheck::Checking;
    let (msg, msg_color) = match state.about_check {
        AboutCheck::Idle | AboutCheck::Checking => (String::new(), p.text_dim),
        // `--status-success` is undefined app-wide, so `.up` inherits `--text`.
        AboutCheck::UpToDate => ("\u{2713} You are up to date".to_owned(), p.text),
        AboutCheck::NoReleases => (state.t("No releases yet").to_owned(), p.text_dim),
        AboutCheck::Error => (
            if state.about_check_error.is_empty() {
                state.t("Could not check for updates").to_owned()
            } else {
                state.about_check_error.clone()
            },
            p.status_error,
        ),
    };

    // ── vertical chain (CSS line boxes, measured against Chromium) ─────
    const NAME_H: f32 = 24.0; // `line-height: normal` at 20px
    let ver_h = 10.0 * 1.45 + 4.0; // 1px padding + 1px border (border-box)
    let ver_top = 28.0 + 52.0 + 6.0 + 2.0 + NAME_H + 2.0 + 3.0;
    let desc_top = ver_top + ver_h + 2.0 + 6.0;
    let tech_top = desc_top + desc_g.size().y + 2.0;
    const CHIP_H: f32 = 24.0;
    let hero_h = tech_top + 10.0 + CHIP_H + 16.0;

    let gh_card_h = 10.0 + 32.675 + 10.0;
    let gh_h = 12.0 + gh_card_h + 12.0;
    // `.ab-update-msg` wraps (only `.ab-github-url` is nowrap+ellipsis), so the
    // card grows with a long backend error.
    let row_text_x = 24.0 + 32.0 + 10.0; // padding + icon + gap
    let msg_max_w = card_w - row_text_x - 16.0 - 10.0;
    let (msg_lead, msg_body) = match msg.strip_prefix("\u{2713} ") {
        Some(rest) => (true, rest),
        None => (false, msg.as_str()),
    };
    let msg_g = (!msg_body.is_empty()).then(|| {
        galley(
            ctx,
            msg_body,
            prop(11.5),
            msg_color,
            16.675,
            msg_max_w - if msg_lead { 13.0 } else { 0.0 },
        )
    });
    let upd_block_h = 15.0
        + msg_g
            .as_ref()
            .map(|g| 1.0 + g.size().y)
            .unwrap_or(0.0);
    let upd_card_h = 10.0 + upd_block_h.max(32.0) + 10.0;
    let upd_h = upd_card_h + 4.0;
    let foot_top = hero_h + gh_h + upd_h;
    const FOOT_H: f32 = 42.4;
    let h = foot_top + FOOT_H;

    let card = Rect::from_min_size(
        pos2(screen.center().x - w * 0.5, screen.center().y - h * 0.5),
        vec2(w, h),
    );

    let mut close = false;

    // `.about-modal-overlay { background: rgba(0,0,0,.5) }` (its
    // `backdrop-filter: blur(4px)` is not reproduced).
    egui::Area::new(egui::Id::new("about-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(128));
            let clicked = ui.allocate_rect(screen, Sense::click()).clicked();
            let over_card = ui
                .ctx()
                .pointer_interact_pos()
                .is_some_and(|m| card.contains(m));
            if clicked && !over_card {
                close = true;
            }
        });

    let inner_x = card.min.x + 1.0;
    let cx = card.center().x;

    egui::Area::new(egui::Id::new("about-modal"))
        .order(egui::Order::Foreground)
        .fixed_pos(card.min)
        .show(ctx, |ui| {
            let under = ui.painter().add(egui::Shape::Noop);
            ui.allocate_exact_size(card.size(), Sense::hover());
            ui.painter().set(
                under,
                egui::Shape::Vec(crate::ui::shade::soft_shadow_shapes(
                    card,
                    16,
                    Color32::BLACK,
                    8.0,
                    32.0,
                    115,
                )),
            );
            let painter_owned = ui.painter().clone();
            let painter = &painter_owned;
            painter.rect_filled(card, CornerRadius::same(16), p.bg_card);
            painter.rect_stroke(
                card,
                CornerRadius::same(16),
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );

            // ── `.about-modal-close`: 28×28 at top/right 12, radius 6 ────
            let xr = Rect::from_min_size(
                pos2(card.max.x - 1.0 - 12.0 - 28.0, card.min.y + 1.0 + 12.0),
                vec2(28.0, 28.0),
            );
            let xresp = ui.interact(xr, egui::Id::new("about-modal-x"), Sense::click());
            painter.rect_filled(xr, CornerRadius::same(6), p.hover_veil);
            mi::paint_centered(
                painter,
                xr.center(),
                "close",
                16.0,
                if xresp.hovered() { p.text } else { p.text_dim },
            );
            if xresp.clicked() {
                close = true;
            }

            // ── hero ────────────────────────────────────────────────────
            let ring = pos2(cx, card.min.y + 28.0 + 26.0);
            painter.circle_filled(ring, 26.0, with_alpha(p.accent, 0.08));
            painter.circle_stroke(ring, 25.25, Stroke::new(1.5, with_alpha(p.accent, 0.20)));
            mi::paint_centered(painter, ring, "nightlight_round", 26.0, p.accent_strong);

            // `.ab-name`: 20px/700, letter-spacing -0.01em.
            painter.text(
                pos2(cx, card.min.y + 28.0 + 52.0 + 6.0 + 2.0),
                Align2::CENTER_TOP,
                "MoonHunt",
                bold(20.0),
                p.text,
            );

            // `.ab-ver` pill: 10px, padding 1 8, 1px border, radius 999.
            let vf = prop(10.0);
            let vw = tw(ctx, &version, &vf) + 18.0;
            let ver = Rect::from_min_size(
                pos2(cx - vw * 0.5, card.min.y + ver_top),
                vec2(vw, ver_h),
            );
            painter.rect_filled(ver, CornerRadius::same(9), p.hover_veil);
            painter.rect_stroke(
                ver,
                CornerRadius::same(9),
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );
            painter.text(ver.center(), Align2::CENTER_CENTER, version.clone(), vf, p.text_dim);

            // `.ab-desc`: 12.5px/1.55, max-width 300, centered.
            painter.galley(
                pos2(cx - desc_g.size().x * 0.5, card.min.y + desc_top),
                desc_g.clone(),
                p.text_muted,
            );

            // `.ab-tech` chips: 11px/600, padding 3 9, gap 6, radius 5.
            let chips = ["Rust", "egui", "aria2"];
            let cf = semi(11.0);
            let widths: Vec<f32> = chips.iter().map(|c| tw(ctx, c, &cf) + 20.0).collect();
            let row_w: f32 = widths.iter().sum::<f32>() + 6.0 * (chips.len() - 1) as f32;
            let mut x = cx - row_w * 0.5;
            let cy = card.min.y + tech_top + 10.0;
            for (c, cw) in chips.iter().zip(widths.iter()) {
                let r = Rect::from_min_size(pos2(x, cy), vec2(*cw, CHIP_H));
                painter.rect_filled(r, CornerRadius::same(5), with_alpha(p.accent, 0.06));
                painter.rect_stroke(
                    r,
                    CornerRadius::same(5),
                    Stroke::new(1.0, with_alpha(p.accent, 0.10)),
                    StrokeKind::Inside,
                );
                painter.text(r.center(), Align2::CENTER_CENTER, *c, cf.clone(), p.accent_strong);
                x += cw + 6.0;
            }

            // ── GitHub row (`.ab-github` padding 12 16) ─────────────────
            let gh = Rect::from_min_size(
                pos2(inner_x + 16.0, card.min.y + hero_h + 12.0),
                vec2(card_w, gh_card_h),
            );
            let gresp = ui.interact(gh, egui::Id::new("about-modal-github"), Sense::click());
            let ghov = gresp.hovered();
            painter.rect_filled(
                gh,
                CornerRadius::same(12),
                if ghov { with_alpha(p.accent, 0.06) } else { p.hover_veil },
            );
            painter.rect_stroke(
                gh,
                CornerRadius::same(12),
                Stroke::new(
                    1.0,
                    if ghov { with_alpha(p.accent, 0.28) } else { p.border },
                ),
                StrokeKind::Inside,
            );
            paint_row_icon(painter, gh, "code", p.accent_strong);
            paint_row_text(
                painter,
                ctx,
                gh,
                card_w,
                state.t("View on GitHub"),
                "github.com/2sulraw/MoonHunt-DownloadManager",
                None,
            );
            mi::paint_centered(
                painter,
                pos2(gh.max.x - 12.0 - 8.0, gh.center().y),
                "open_in_new",
                16.0,
                p.text_dim,
            );
            if gresp.clicked() {
                cctx.run(|app| async move {
                    moonhunt_core::fs_ops::open_external(
                        app,
                        "https://github.com/2sulraw/MoonHunt-DownloadManager".to_owned(),
                    )
                    .map(|()| String::new())
                });
            }

            // ── update row (`.ab-update` padding 0 16 4) ────────────────
            let up = Rect::from_min_size(
                pos2(inner_x + 16.0, card.min.y + hero_h + gh_h),
                vec2(card_w, upd_card_h),
            );
            let uresp = ui.interact(up, egui::Id::new("about-modal-check"), Sense::click());
            let uhov = uresp.hovered() && !checking;
            painter.rect_filled(
                up,
                CornerRadius::same(12),
                if uhov { with_alpha(p.accent, 0.06) } else { p.hover_veil },
            );
            painter.rect_stroke(
                up,
                CornerRadius::same(12),
                Stroke::new(
                    1.0,
                    if uhov { with_alpha(p.accent, 0.28) } else { p.border },
                ),
                StrokeKind::Inside,
            );
            paint_row_icon(
                painter,
                up,
                "system_update_alt",
                if checking { p.text_dim } else { p.accent_strong },
            );
            paint_row_text(
                painter,
                ctx,
                up,
                card_w,
                if checking {
                    state.t("Checking for updates...")
                } else {
                    state.t("Check for Update")
                },
                "",
                msg_g.as_ref().map(|g| (g, msg_color, msg_lead)),
            );
            if !checking {
                mi::paint_centered(
                    painter,
                    pos2(up.max.x - 12.0 - 8.0, up.center().y),
                    "chevron_right",
                    16.0,
                    p.text_dim,
                );
            }
            if uresp.clicked() && !checking {
                state.about_check = AboutCheck::Checking;
                state.about_check_error.clear();
                cctx.emit("update-available", |app| async move {
                    moonhunt_core::update::check_for_update(app)
                        .await
                        .map(|s| serde_json::to_value(s).unwrap_or_default())
                });
            }

            // ── `.ab-footer`: padding 12 20, border-top ─────────────────
            let fy = card.min.y + foot_top;
            let fr = Rect::from_min_size(pos2(inner_x, fy), vec2(w - 2.0, FOOT_H));
            painter.hline(
                fr.x_range(),
                fr.min.y + 0.5,
                Stroke::new(1.0, p.border),
            );
            let f1 = prop(12.0);
            let f2 = semi(12.0);
            let w1 = tw(ctx, "Built by ", &f1);
            let w2 = tw(ctx, "2sulraw", &f2);
            let tx = cx - (w1 + w2) * 0.5;
            let ty = fr.center().y + 0.5;
            painter.text(pos2(tx, ty), Align2::LEFT_CENTER, "Built by ", f1, p.text_dim);
            painter.text(
                pos2(tx + w1, ty),
                Align2::LEFT_CENTER,
                "2sulraw",
                f2,
                p.text_muted,
            );
        });

    if close {
        state.show_about = false;
        // The Vue component is unmounted (`v-if`), so `check` resets.
        state.about_check = AboutCheck::Idle;
        state.about_check_error.clear();
    }
}

/// `.ab-github-icon`: 32×32, radius 8, accent-tinted square.
fn paint_row_icon(painter: &egui::Painter, row: Rect, glyph: &str, color: Color32) {
    let p = mk::pal();
    let r = Rect::from_min_size(
        pos2(row.min.x + 12.0, row.center().y - 16.0),
        vec2(32.0, 32.0),
    );
    painter.rect_filled(r, CornerRadius::same(8), with_alpha(p.accent, 0.06));
    mi::paint_centered(painter, r.center(), glyph, 16.0, color);
}

/// The two-line text block shared by both rows: `<strong>` then an optional
/// message line (`12.5px/600` + `11.5px/1.45`).
fn paint_row_text(
    painter: &egui::Painter,
    ctx: &egui::Context,
    row: Rect,
    card_w: f32,
    strong: &str,
    url: &str,
    msg: Option<(&std::sync::Arc<egui::Galley>, Color32, bool)>,
) {
    let p = mk::pal();
    let sfont = semi(12.5);
    let ufont = prop(11.5);
    let x = row.min.x + 12.0 + 32.0 + 10.0;
    let max_w = card_w - 24.0 - 32.0 - 10.0 - 16.0 - 10.0;
    let mut block_h = 15.0;
    if !url.is_empty() {
        block_h += 1.0 + 16.675;
    }
    if let Some((g, _, _)) = msg {
        block_h += 1.0 + g.size().y;
    }
    let mut y = row.center().y - block_h * 0.5;
    painter.text(pos2(x, y), Align2::LEFT_TOP, strong, sfont, p.text_muted);
    y += 15.0;
    if !url.is_empty() {
        y += 1.0;
        painter.text(
            pos2(x, y),
            Align2::LEFT_TOP,
            ellipsize_ctx(ctx, url, &ufont, max_w),
            ufont.clone(),
            p.text_dim,
        );
        y += 16.675;
    }
    if let Some((g, color, lead)) = msg {
        y += 1.0;
        // The reference writes a literal "✓"; the proportional stack has no
        // U+2713, so use the Material Icons check at the same size.
        let mut tx = x;
        if lead {
            mi::paint_centered(painter, pos2(x + 5.5, y + 8.4), "check", 11.0, color);
            tx += 13.0;
        }
        painter.galley(pos2(tx, y), g.clone(), color);
    }
}

fn ellipsize_ctx(ctx: &egui::Context, text: &str, font: &FontId, max_w: f32) -> String {
    crate::rtl::truncate(text, |s| tw(ctx, s, font) <= max_w)
}

// ── UpdateModal ─────────────────────────────────────────────────────────

/// `UpdateModal.vue` — "Update is Ready" card. CSS is laid out by hand so the
/// line boxes match Chromium's (19px title at 1.45, 13px sub at 1.5, …).
pub fn update_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    // `width: min(400px, 90vw)`.
    let w = 400.0_f32.min(screen.width() * 0.9);
    // 1px border + 20px padding on each side.
    let content_w = w - 42.0;

    // The button re-enables 2.5s after a successful install (`setTimeout`).
    if let Some(at) = state.upd_reset_at {
        if std::time::Instant::now() >= at {
            state.upd_installing = false;
            state.upd_reset_at = None;
        } else {
            ctx.request_repaint_after(at - std::time::Instant::now());
        }
    }

    let info = state.update_info.clone().unwrap_or_default();
    let version = if info.version.is_empty() { "…" } else { info.version.as_str() };
    let show_progress = state
        .upd_progress
        .is_some_and(|pr| pr.total > 0);
    let percent = state.upd_progress.map(|pr| pr.percent).unwrap_or(0.0);
    let show_error = !state.upd_error.is_empty();
    let show_cl = !info.body.is_empty();

    // Hero (`.um-hero` padding 6 8 16, gap 4): 6 + ring 52 + mb 10 + gap 4 +
    // title 27.55 + gap 4 + (sub mt 4 + 19.5) + gap 4 + (ver mt 6 + 21.95) +
    // gap 4 + (ready mt 8 + 25.4) + pad 16.
    const HERO_H: f32 = 212.4;
    let hero_top = 25.0; // border 1 + padding 24

    // Error text box (padding 8 10, border 1, 12px at 1.45).
    let err_galley = show_error.then(|| {
        galley(ctx, &state.upd_error, prop(12.0), p.status_error, 17.4, content_w - 22.0)
    });
    let err_h = err_galley.as_ref().map(|g| g.size().y + 18.0).unwrap_or(0.0);
    // Changelog body (padding 10, 12px at 1.5, max-height 160 border-box).
    let cl_galley = show_cl.then(|| {
        galley(ctx, &info.body, prop(12.0), p.text_dim, 18.0, content_w - 22.0)
    });
    let cl_inner_h = cl_galley.as_ref().map(|g| g.size().y).unwrap_or(0.0);
    let cl_box_h = (cl_inner_h + 20.0).min(160.0);

    let actions_top = hero_top + HERO_H + 6.0;
    let mut y = actions_top + 38.0;
    let progress_top = show_progress.then(|| {
        y += 14.0;
        y
    });
    if progress_top.is_some() {
        y += 17.4;
    }
    let error_top = err_galley.as_ref().map(|_| {
        y += 12.0;
        y
    });
    if error_top.is_some() {
        y += err_h;
    }
    let cl_top = cl_galley.as_ref().map(|_| {
        y += 12.0;
        y
    });
    if cl_top.is_some() {
        y += 17.4 + 6.0 + cl_box_h;
    }
    let footer_top = y + 16.0;
    let h = footer_top + 33.4 + 21.0; // footer 33.4 + padding 20 + border 1

    let card = Rect::from_min_size(
        pos2(screen.center().x - w * 0.5, screen.center().y - h * 0.5),
        vec2(w, h),
    );

    let mut close = false;

    // `.um-overlay { background: rgba(0,0,0,.5) }` + click-outside close.
    egui::Area::new(egui::Id::new("update-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(128));
            let clicked = ui.allocate_rect(screen, Sense::click()).clicked();
            let over_card = ui
                .ctx()
                .pointer_interact_pos()
                .is_some_and(|m| card.contains(m));
            if clicked && !over_card {
                close = true;
            }
        });

    let green = |a: f32| with_alpha(p.status_complete, a);
    let inner_x = card.min.x + 21.0;
    let inner_r = card.max.x - 21.0;

    egui::Area::new(egui::Id::new("update-modal"))
        .order(egui::Order::Foreground)
        .fixed_pos(card.min)
        .show(ctx, |ui| {
            let under = ui.painter().add(egui::Shape::Noop);
            ui.allocate_exact_size(card.size(), Sense::hover());
            ui.painter().set(
                under,
                egui::Shape::Vec(crate::ui::shade::soft_shadow_shapes(
                    card,
                    16,
                    Color32::BLACK,
                    8.0,
                    32.0,
                    115,
                )),
            );
            let painter_owned = ui.painter().clone();
            let painter = &painter_owned;
            painter.rect_filled(card, CornerRadius::same(16), p.bg_card);
            painter.rect_stroke(
                card,
                CornerRadius::same(16),
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );

            // ── close-x: 28×28 at top/right 12 ───────────────────────────
            let xr = Rect::from_min_size(
                pos2(card.max.x - 1.0 - 12.0 - 28.0, card.min.y + 1.0 + 12.0),
                vec2(28.0, 28.0),
            );
            let xresp = ui.interact(xr, egui::Id::new("update-modal-x"), Sense::click());
            let xhov = xresp.hovered();
            painter.rect_filled(xr, CornerRadius::same(6), p.hover_veil);
            mi::paint_centered(
                painter,
                xr.center(),
                "close",
                16.0,
                if xhov { p.text } else { p.text_dim },
            );
            if xresp.clicked() {
                close = true;
            }

            // ── hero ─────────────────────────────────────────────────────
            let cx = card.center().x;
            let ring = pos2(cx, card.min.y + hero_top + 6.0 + 26.0);
            painter.circle_filled(ring, 26.0, with_alpha(p.accent, 0.08));
            painter.circle_stroke(
                ring,
                25.25,
                Stroke::new(1.5, with_alpha(p.accent, 0.20)),
            );
            mi::paint_centered(painter, ring, "nightlight_round", 26.0, p.accent_strong);

            let title_y = card.min.y + hero_top + 6.0 + 52.0 + 10.0 + 4.0;
            let tgal = galley(ctx, state.t("Update is Ready"), bold(19.0), p.text, 27.55, content_w - 16.0);
            painter.galley(pos2(cx - tgal.size().x * 0.5, title_y), tgal, p.text);
            let sub_y = title_y + 27.55 + 4.0 + 4.0;
            let sgal = galley(
                ctx,
                state.t("A new version is available"),
                prop(13.0),
                p.text_muted,
                19.5,
                content_w - 16.0,
            );
            painter.galley(pos2(cx - sgal.size().x * 0.5, sub_y), sgal, p.text_muted);

            // `.um-ver` pill: 11px, padding 2 9, 1px accent border.
            let vf = prop(11.0);
            let vtw = text_w(ui, version, &vf);
            let ver_top = sub_y + 19.5 + 4.0 + 6.0;
            let ver = Rect::from_min_size(
                pos2(cx - (vtw + 20.0) * 0.5, ver_top),
                vec2(vtw + 20.0, 21.95),
            );
            painter.rect_filled(ver, CornerRadius::same(11), with_alpha(p.accent, 0.06));
            painter.rect_stroke(
                ver,
                CornerRadius::same(11),
                Stroke::new(1.0, with_alpha(p.accent, 0.10)),
                StrokeKind::Inside,
            );
            painter.text(ver.center(), Align2::CENTER_CENTER, version, vf, p.accent_strong);

            // `.um-ready` pill: 12/600, icon 14, gap 5, padding 3 10.
            let rf = semi(12.0);
            let rtw = text_w(ui, state.t("Ready"), &rf);
            let ready_top = ver_top + 21.95 + 4.0 + 8.0;
            let rw = 20.0 + 14.0 + 5.0 + rtw + 2.0;
            let ready = Rect::from_min_size(
                pos2(cx - rw * 0.5, ready_top),
                vec2(rw, 25.4),
            );
            painter.rect_filled(ready, CornerRadius::same(13), green(0.08));
            painter.rect_stroke(
                ready,
                CornerRadius::same(13),
                Stroke::new(1.0, green(0.18)),
                StrokeKind::Inside,
            );
            mi::paint_centered(
                painter,
                pos2(ready.min.x + 11.0 + 7.0, ready.center().y),
                "check_circle",
                14.0,
                p.status_complete,
            );
            painter.text(
                pos2(ready.min.x + 11.0 + 14.0 + 5.0, ready.center().y),
                Align2::LEFT_CENTER,
                state.t("Ready"),
                rf,
                p.status_complete,
            );

            // ── actions: two 174px buttons, gap 10, h 38 ─────────────────
            let by = card.min.y + actions_top;
            let bw = (content_w - 10.0) * 0.5;
            let later = Rect::from_min_size(pos2(inner_x, by), vec2(bw, 38.0));
            let lresp = ui.interact(later, egui::Id::new("update-modal-later"), Sense::click());
            let lhov = lresp.hovered();
            painter.rect_filled(later, CornerRadius::same(10), p.hover_veil);
            painter.rect_stroke(
                later,
                CornerRadius::same(10),
                Stroke::new(1.0, if lhov { with_alpha(p.accent, 0.30) } else { p.border }),
                StrokeKind::Inside,
            );
            painter.text(later.center(), Align2::CENTER_CENTER, state.t("Later"), semi(13.0), p.text);
            if lresp.clicked() {
                close = true;
            }

            let inst = Rect::from_min_size(
                pos2(inner_x + bw + 10.0, by),
                vec2(bw, 38.0),
            );
            let iresp = ui.interact(inst, egui::Id::new("update-modal-install"), Sense::click());
            let ihov = iresp.hovered() && !state.upd_installing;
            let dim = state.upd_installing;
            let (ifill, iborder, ifg) = if dim {
                (green(0.048), green(0.108), with_alpha(p.status_complete, 0.6))
            } else {
                (
                    green(if ihov { 0.14 } else { 0.08 }),
                    green(if ihov { 0.30 } else { 0.18 }),
                    p.status_complete,
                )
            };
            painter.rect_filled(inst, CornerRadius::same(10), ifill);
            painter.rect_stroke(
                inst,
                CornerRadius::same(10),
                Stroke::new(1.0, iborder),
                StrokeKind::Inside,
            );
            let label = if dim {
                state.t("Downloading update…")
            } else {
                state.t("Install")
            };
            let lf = semi(13.0);
            let ltw = text_w(ui, label, &lf);
            let total = 16.0 + 6.0 + ltw;
            let ix = inst.center().x - total * 0.5;
            mi::paint_centered(
                painter,
                pos2(ix + 8.0, inst.center().y),
                if dim { "hourglass_top" } else { "check_circle" },
                16.0,
                ifg,
            );
            painter.text(
                pos2(ix + 22.0, inst.center().y),
                Align2::LEFT_CENTER,
                label,
                lf,
                ifg,
            );
            if iresp.clicked() && !state.upd_installing {
                state.upd_installing = true;
                state.upd_error.clear();
                let asset = info.asset_url.clone();
                cctx.emit("update-install-result", move |app| async move {
                    moonhunt_core::update::install_update(app, asset)
                        .await
                        .map(serde_json::Value::String)
                });
            }

            // ── progress ─────────────────────────────────────────────────
            if let Some(top) = progress_top {
                let py = card.min.y + top;
                let track = Rect::from_min_size(
                    pos2(inner_x, py + 5.7),
                    vec2(content_w - 44.0, 6.0),
                );
                painter.rect_filled(track, CornerRadius::same(3), p.hover_veil);
                let fw = track.width() * (percent.clamp(0.0, 100.0) / 100.0);
                if fw > 0.0 {
                    painter.rect_filled(
                        Rect::from_min_size(track.min, vec2(fw, 6.0)),
                        CornerRadius::same(3),
                        p.accent_strong,
                    );
                }
                painter.text(
                    pos2(inner_r, py + 8.7),
                    Align2::RIGHT_CENTER,
                    format!("{percent:.0}%"),
                    prop(12.0),
                    p.text_dim,
                );
            }

            // ── error ────────────────────────────────────────────────────
            if let (Some(top), Some(g)) = (error_top, err_galley.as_ref()) {
                let er = Rect::from_min_size(
                    pos2(inner_x, card.min.y + top),
                    vec2(content_w, err_h),
                );
                painter.rect_filled(
                    er,
                    CornerRadius::same(8),
                    Color32::from_rgba_unmultiplied(244, 67, 54, 20),
                );
                painter.rect_stroke(
                    er,
                    CornerRadius::same(8),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(244, 67, 54, 46)),
                    StrokeKind::Inside,
                );
                painter.galley(er.min + vec2(11.0, 9.0), g.clone(), p.status_error);
            }

            // ── changelog section ────────────────────────────────────────
            if let (Some(top), Some(g)) = (cl_top, cl_galley.as_ref()) {
                let cy = card.min.y + top;
                painter.text(
                    pos2(inner_x, cy),
                    Align2::LEFT_TOP,
                    state.t("Changelog"),
                    semi(12.0),
                    p.text,
                );
                let body = Rect::from_min_size(
                    pos2(inner_x, cy + 17.4 + 6.0),
                    vec2(content_w, cl_box_h),
                );
                painter.rect_filled(body, CornerRadius::same(8), p.hover_veil);
                // `overflow-y: auto` — a real ScrollArea inside the 10px
                // padding box (a wheel handler reading the raw input would
                // miss the delta: the screen's ScrollArea consumes it first).
                let view = body.shrink2(vec2(10.0, 10.0));
                let mut sui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(view)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                egui::ScrollArea::vertical()
                    .id_salt("update-modal-changelog")
                    .auto_shrink([false, false])
                    .show(&mut sui, |ui| {
                        let (r, _) = ui.allocate_exact_size(g.size(), Sense::hover());
                        ui.painter().galley(r.min, g.clone(), p.text_dim);
                    });
            }

            // ── "View changelog on GitHub" footer link ───────────────────
            let fy = card.min.y + footer_top;
            let fr = Rect::from_min_size(pos2(inner_x, fy), vec2(content_w, 33.4));
            let fresp = ui.interact(fr, egui::Id::new("update-modal-changelog"), Sense::click());
            let fhov = fresp.hovered();
            if fhov {
                painter.rect_filled(fr, CornerRadius::same(8), with_alpha(p.accent, 0.04));
            }
            let ff = prop(12.0);
            let ftw = text_w(ui, state.t("View changelog on GitHub"), &ff);
            let ftotal = 15.0 + 6.0 + ftw;
            let fx = fr.center().x - ftotal * 0.5;
            let ffg = if fhov { p.accent_strong } else { p.text_dim };
            mi::paint_centered(painter, pos2(fx + 7.5, fr.center().y), "open_in_new", 15.0, ffg);
            painter.text(
                pos2(fx + 21.0, fr.center().y),
                Align2::LEFT_CENTER,
                state.t("View changelog on GitHub"),
                ff,
                ffg,
            );
            if fresp.clicked() && !info.url.is_empty() {
                let url = info.url.clone();
                cctx.run(move |app| async move {
                    moonhunt_core::fs_ops::open_external(app, url).map(|()| String::new())
                });
            }
        });

    if close {
        state.show_update_modal = false;
        // The Vue component is destroyed (`v-if`), so its local refs reset.
        state.upd_installing = false;
        state.upd_progress = None;
        state.upd_error.clear();
        state.upd_reset_at = None;
    }
    // The 2.5s re-enable needs a repaint even while idle.
    if state.upd_reset_at.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

/// Lay out wrapped text with a CSS line-height (Chromium's line box height).
fn galley(
    ctx: &egui::Context,
    text: &str,
    font: FontId,
    color: Color32,
    line_h: f32,
    wrap: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: font,
            color,
            line_height: Some(line_h),
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap;
    ctx.fonts_mut(|f| f.layout_job(job))
}

// ── ClipboardToast ──────────────────────────────────────────────────────

/// Fixed stack 100px above the bottom edge, newest at the bottom
/// (`flex-direction: column-reverse`).
pub fn clipboard_toasts(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let _ = cctx;
    if state.clip_toasts.is_empty() {
        return;
    }
    let p = mk::pal();
    let screen = ctx.content_rect();
    let w = 380.0_f32.min((screen.width() - 40.0).max(280.0));
    let bottom = screen.max.y - 100.0;
    // Height per card: 12 + body + 10 + actions 24 + 12.
    let title_font = semi(12.0);
    let url_font = prop(11.0);
    let body_h = (ui_row_height(ctx, &title_font) + 2.0 + ui_row_height(ctx, &url_font)).max(20.0);
    let card_h = 12.0 + body_h + 10.0 + 24.0 + 12.0;

    let mut add_url: Option<String> = None;
    let mut dismiss: Option<u64> = None;

    let mut y = bottom;
    let toasts: Vec<(u64, String)> = state
        .clip_toasts
        .iter()
        .map(|t| (t.id, t.url.clone()))
        .collect();
    for (id, url) in &toasts {
        let rect = Rect::from_min_size(pos2(screen.center().x - w * 0.5, y - card_h), vec2(w, card_h));
        y -= card_h + 8.0;
        let mut added = false;
        let mut dismissed = false;
        egui::Area::new(egui::Id::new(("clip-toast", *id)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                // Reserve the card's box so the Area's clip rect covers it;
                // otherwise the area is empty and every raw `rect_filled`
                // below is clipped away.
                ui.allocate_exact_size(rect.size(), egui::Sense::hover());
                crate::ui::shade::soft_shadow(ui, rect, 12, Color32::BLACK, 8.0, 24.0, 51);
                ui.painter().rect_filled(rect, CornerRadius::same(12), p.bg_card);
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(12),
                    Stroke::new(1.0, p.border_strong),
                    StrokeKind::Inside,
                );
                // `.toast-icon` 20px, accent, margin-top 2.
                mi::paint_centered(
                    ui.painter(),
                    pos2(rect.min.x + 16.0 + 10.0, rect.min.y + 12.0 + 10.0),
                    "link",
                    20.0,
                    p.accent_strong,
                );
                let tx = rect.min.x + 16.0 + 20.0 + 10.0;
                ui.painter().text(
                    pos2(tx, rect.min.y + 12.0),
                    Align2::LEFT_TOP,
                    state.t("Download link detected"),
                    title_font.clone(),
                    p.text,
                );
                let url_w = (rect.max.x - 16.0 - tx).max(40.0);
                ui.painter().text(
                    pos2(tx, rect.min.y + 12.0 + body_h - ui_row_height(ctx, &url_font)),
                    Align2::LEFT_TOP,
                    ellipsize(ui, url, &url_font, url_w),
                    url_font.clone(),
                    p.text_dim,
                );
                // Actions (right-aligned, gap 6, padding 5 14).
                let (dismiss_clicked, dismiss_w) =
                    clip_btn(ui, &p, rect.max.x - 16.0, rect.max.y - 12.0, state.t("Dismiss"), false, *id);
                let add_right = rect.max.x - 16.0 - dismiss_w - 6.0;
                let (add_clicked, _) = clip_btn(ui, &p, add_right, rect.max.y - 12.0, state.t("Add"), true, *id);
                added = add_clicked;
                dismissed = dismiss_clicked;
            });
        if added {
            add_url = Some(url.clone());
        }
        if dismissed {
            dismiss = Some(*id);
        }
    }

    if let Some(url) = add_url {
        dismiss = Some(u64::MAX);
        // `handleClipboardDownload` → `downloadDialog.openDialog(url, '', '', undefined)`.
        state.add_url = url;
        state.nd_pasted = true;
        state.nd_filename.clear();
        state.nd_path.clear();
        state.show_new_download = true;
    }
    if let Some(id) = dismiss {
        if id == u64::MAX {
            state.clip_toasts.clear();
        } else {
            state.dismiss_clip_toast(id);
        }
    }
    if !state.clip_toasts.is_empty() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

fn ui_row_height(ctx: &egui::Context, font: &FontId) -> f32 {
    ctx.fonts_mut(|f| f.row_height(font))
}

/// `.toast-btn`: padding 5 14, radius 6, 12/600. Returns (clicked, width).
/// `salt` keeps the widget id unique per toast: two toasts on the same frame
/// share the label, and a duplicate id in different layers is a hard egui
/// error.
fn clip_btn(
    ui: &mut egui::Ui,
    p: &crate::ui::tokens::Palette,
    right: f32,
    bottom: f32,
    label: &str,
    primary: bool,
    salt: u64,
) -> (bool, f32) {
    let font = semi(12.0);
    let tw = text_w(ui, label, &font);
    let rect = Rect::from_min_size(
        pos2(right - (tw + 28.0), bottom - 24.0),
        vec2(tw + 28.0, 24.0),
    );
    let resp = ui.interact(rect, egui::Id::new(("clip-btn", salt, label)), Sense::click());
    let hovered = resp.hovered();
    if primary {
        // `.toast-btn-primary:hover` → `box-shadow: 0 2px 8px rgba(accent,.3)`,
        // painted under the fill like CSS does.
        if hovered {
            crate::ui::shade::soft_shadow(ui, rect, 6, p.accent, 2.0, 8.0, 77);
        }
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            font,
            Color32::from_rgb(0x0a, 0x0e, 0x1a),
        );
    } else {
        // `.toast-btn-ghost:hover` → `background: rgba(255,255,255,0.08)`.
        let fill = if hovered {
            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
        } else {
            p.hover_veil
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            font,
            p.text,
        );
    }
    (resp.clicked(), rect.width())
}

