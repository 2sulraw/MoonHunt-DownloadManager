//! Downloads screen â€” pixel port of `DownloadsPage.vue` + `DownloadListCard.vue`
//! (list + compact variants): settings-hero (diagonal gradient, orb, 32/800 h1,
//! filter-bar with sliding indicator, hero bottom toolbar), `.download-list-card`
//! rows (status pill, http/yt-dlp badge, size, speed/eta, 5px progress, layers
//! connections row) and the Downloads empty state.

use super::Ctx;
use crate::state::{Filter, SortKey, Status, UiState};
use crate::ui::{icons as mi, tokens, widgets as mk};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Mesh, Pos2, Rect,
    ScrollArea, Sense, Stroke, StrokeKind, Ui,
};
use moonhunt_core::util::format_bytes;

pub fn show(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    // `.content-area` scrolls edge-to-edge: in the old app the scrollbar hugs
    // the window's right edge (4px) and the 20px page padding lives *inside*
    // the scroll content. egui draws the bar just outside the content rect, so
    // grow the scroll viewport into the panel's right margin and pad the
    // content by 20 instead â€” cards stay at x..+951 while the bar moves from
    // x+951 to the window edge.
    let mut vrect = ui.available_rect_before_wrap();
    vrect.max.x += 20.0;
    // `.scroll-top-btn` scrolls `.content-area` back to the top; the flag is
    // raised by the button and consumed on the next ScrollArea build.
    let jump_top = ui
        .ctx()
        .data_mut(|d| d.remove_temp::<bool>(egui::Id::new("dl_scroll_to_top")))
        .unwrap_or(false);
    let mut sa = ScrollArea::vertical().auto_shrink([false, false]);
    if jump_top {
        sa = sa.vertical_scroll_offset(0.0);
    }
    let sort_was_open = state.sort_menu_open;
    let scrolled = ui
        .scope_builder(egui::UiBuilder::new().max_rect(vrect), |ui| {
            sa.show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 20.0);
                // Exact CSS gaps: kill egui's default item spacing first.
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let w = ui.available_width();
                hero(state, ctx, ui, w);
                ui.add_space(18.0); // `.settings-hero` margin-bottom
                if state.visible_downloads().is_empty() {
                    empty_state(state, ui, w);
                } else if state.view_compact {
                    compact_list(state, ctx, ui, w);
                } else {
                    list(state, ctx, ui, w);
                    ui.add_space(40.0); // `.downloads-list` padding-bottom
                }
            })
            .state
            .offset()
            .y
        })
        .inner;
    // `.content-area` scroll listener: sticky toolbar past 160, FAB past 400.
    if scrolled > 160.0 {
        sticky_toolbar(state, ctx, ui);
    }
    if state.sort_menu_open {
        let anchor = ui
            .data(|d| d.get_temp::<Rect>(egui::Id::new("dl_sort_anchor")))
            .unwrap_or_else(|| Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 0.0)));
        let menu = sort_menu(state, ui, anchor);
        // Any click outside the menu and its button closes it. Skip the frame
        // the menu was opened in (that click is the button press itself).
        if sort_was_open {
            let (clicked, pos) =
                ui.input(|i| (i.pointer.primary_clicked(), i.pointer.interact_pos()));
            if clicked
                && pos.is_some_and(|q| !menu.contains(q) && !anchor.contains(q))
            {
                state.sort_menu_open = false;
            }
        }
    }
    if scrolled > 400.0 {
        scroll_top_button(ui);
    }
    // `.dlc-dropdown` / `.group-dropdown-menu` — the per-card "Group" popover,
    // drawn above the cards as a Foreground area.
    if let Some(gid) = state.g_menu_gid.clone() {
        let anchor = ui
            .data(|d| d.get_temp::<Rect>(egui::Id::new("dl_group_anchor")))
            .unwrap_or_else(|| Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 0.0)));
        let menu = group_menu(state, ctx, ui, anchor, &gid);
        let (clicked, pos) =
            ui.input(|i| (i.pointer.primary_clicked(), i.pointer.interact_pos()));
        if clicked && pos.is_some_and(|q| !menu.contains(q) && !anchor.contains(q)) {
            state.g_menu_gid = None;
        }
    }
    if state.drag_gid.is_some() || state.drag_group.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        draw_drag_ghost(state, ui);
    } else {
        // Reset the ghost lift animation when not dragging.
        ui.ctx()
            .animate_bool_with_time(egui::Id::new("dl_drag_ghost_anim"), false, 0.12);
    }
}

/// Floating card/group ghost that follows the cursor while dragging in grab mode.
fn draw_drag_ghost(state: &UiState, ui: &Ui) {
    let Some(pos) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let p = mk::pal();
    let t = ui
        .ctx()
        .animate_bool_with_time(egui::Id::new("dl_drag_ghost_anim"), true, 0.14);
    let lift = 4.0 * t;

    let (title, badge, accent_col) = if let Some(gid) = state.drag_gid.as_deref() {
        let Some(vm) = state.downloads.iter().find(|d| d.gid == gid) else {
            return;
        };
        let label = match vm.status {
            Status::Active => "Active",
            Status::Waiting => "Waiting",
            Status::Paused => "Paused",
            Status::Complete => "Completed",
            Status::Error => "Failed",
            Status::Removed => "Cancelled",
        };
        (vm.filename.clone(), state.t(label).to_owned(), p.accent_strong)
    } else if let Some(group_id) = state.drag_group.as_deref() {
        let Some(g) = state.group_by_id(group_id) else {
            return;
        };
        let n = g.member_gids.len();
        (g.name.clone(), format!("{n}"), g.egui_color())
    } else {
        return;
    };

    let w = 260.0f32;
    let h = 44.0f32;
    let ghost_pos = pos2(pos.x + 14.0, pos.y - h * 0.5 - lift);
    let rect = Rect::from_min_size(ghost_pos, vec2(w, h));

    egui::Area::new(egui::Id::new("dl_drag_ghost"))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .fixed_pos(ghost_pos)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(vec2(w, h));
            ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
                rect,
                10,
                Color32::BLACK,
                8.0 + 4.0 * t,
                22.0,
                (55.0 * t) as u8,
            ));
            let bg = Color32::from_rgba_unmultiplied(
                p.bg_card.r(),
                p.bg_card.g(),
                p.bg_card.b(),
                (235.0 * t.max(0.25)) as u8,
            );
            ui.painter().rect_filled(rect, CornerRadius::same(10), bg);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(10),
                Stroke::new(1.5, accent_col.gamma_multiply(0.85)),
                StrokeKind::Inside,
            );
            mi::paint_centered(
                ui.painter(),
                pos2(rect.min.x + 18.0, rect.center().y),
                "drag_indicator",
                16.0,
                accent_col,
            );
            let badge_w = text_width(ui, &badge, regular(11.0)) + 14.0;
            let text_max = (w - 32.0 - badge_w - 14.0).max(60.0);
            draw_text_spaced(
                ui,
                pos2(rect.min.x + 32.0, rect.center().y),
                Align2::LEFT_CENTER,
                ellipsize(ui, &title, regular(12.5), text_max).as_str(),
                regular(12.5),
                p.text,
                0.0,
                0.3,
            );
            let pill = Rect::from_min_size(
                pos2(rect.max.x - 10.0 - badge_w, rect.center().y - 10.0),
                vec2(badge_w, 20.0),
            );
            ui.painter().rect_filled(
                pill,
                CornerRadius::same(6),
                accent_col.gamma_multiply(0.16),
            );
            ui.painter().text(
                pill.center(),
                Align2::CENTER_CENTER,
                &badge,
                regular(11.0),
                accent_col,
            );
        });
}

/// Paints a drop-insertion indicator bar at the top or bottom of `rect` when
/// another card is being dragged over it.
fn draw_card_drop_indicator(ui: &Ui, rect: Rect, color: Color32) {
    let Some(ptr) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    if !rect.contains(ptr) {
        return;
    }
    let y = if ptr.y < rect.center().y {
        rect.min.y - 2.0
    } else {
        rect.max.y + 2.0
    };
    let x0 = rect.min.x + 6.0;
    let x1 = rect.max.x - 6.0;
    ui.painter().line_segment(
        [pos2(x0, y), pos2(x1, y)],
        Stroke::new(2.5, color),
    );
    ui.painter().circle_filled(pos2(x0, y), 3.5, color);
    ui.painter().circle_filled(pos2(x1, y), 3.5, color);
}

/// The card-level "Group" dropdown (`.dlc-dropdown`, compact
/// `.group-dropdown-menu`): a dot + name per group, bottom-right anchored.
fn group_menu(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, anchor: Rect, gid: &str) -> Rect {
    let p = mk::pal();
    let compact = state.view_compact;
    let items: Vec<(String, String, Color32)> = state
        .groups
        .iter()
        .map(|g| (g.id.clone(), g.name.clone(), g.egui_color()))
        .collect();
    let (min_w, item_h, pad_x, font_sz, dot, gap) = if compact {
        (160.0f32, 27.0, 10.0, 12.0, 7.0, 7.0)
    } else {
        (150.0, 30.0, 12.0, 12.5, 7.0, 8.0)
    };
    let mut w = min_w;
    for (_, name, _) in &items {
        let tw = pad_x * 2.0 + dot + gap + text_width(ui, name, regular(font_sz));
        w = w.max(tw);
    }
    let h = 6.0 + items.len() as f32 * item_h + 6.0;
    let pos = pos2(anchor.max.x - w, anchor.min.y - 6.0 - h);
    let rect = Rect::from_min_size(pos, vec2(w, h));

    let mut picked: Option<String> = None;
    egui::Area::new(egui::Id::new("dl_group_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(vec2(w, h));
            if p.is_light {
                ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
                    rect,
                    10,
                    Color32::from_rgb(0x10, 0x18, 0x28),
                    10.0,
                    30.0,
                    26,
                ));
            } else {
                ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
                    rect,
                    10,
                    Color32::BLACK,
                    8.0,
                    32.0,
                    50,
                ));
            }
            ui.painter()
                .rect_filled(rect, CornerRadius::same(10), p.bg_card);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(10),
                Stroke::new(1.0, p.border),
                StrokeKind::Inside,
            );
            let mut iy = rect.min.y + 6.0;
            for (i, (id, name, color)) in items.iter().enumerate() {
                let row = Rect::from_min_size(
                    pos2(rect.min.x, iy),
                    vec2(rect.width(), item_h),
                );
                let rr = ui.interact(
                    row,
                    egui::Id::new("dl_group_item").with(i),
                    Sense::click(),
                );
                if rr.hovered() {
                    ui.painter().rect_filled(
                        row,
                        CornerRadius::same(0),
                        Color32::from_rgba_unmultiplied(
                            p.accent_strong.r(),
                            p.accent_strong.g(),
                            p.accent_strong.b(),
                            20,
                        ),
                    );
                }
                ui.painter().circle_filled(
                    pos2(row.min.x + pad_x + dot * 0.5, row.center().y),
                    dot * 0.5,
                    *color,
                );
                draw_text(
                    ui,
                    pos2(row.min.x + pad_x + dot + gap, row.center().y),
                    Align2::LEFT_CENTER,
                    name,
                    regular(font_sz),
                    p.text,
                );
                if rr.clicked() {
                    picked = Some(id.clone());
                }
                iy += item_h;
            }
        });

    if let Some(group_id) = picked {
        let limit = state
            .group_by_id(&group_id)
            .map(|g| g.speed_limit)
            .unwrap_or(0);
        let name = state
            .group_by_id(&group_id)
            .map(|g| g.name.clone())
            .unwrap_or_default();
        state.group_add_member(&group_id, gid);
        state.g_menu_gid = None;
        state.toast(true, format!("Added to: \"{name}\""));
        let gid = gid.to_owned();
        ctx.run(move |_| async move {
            let _ =
                moonhunt_core::aria2::apply_download_speed_limit(gid, limit.saturating_mul(1024))
                    .await;
            Ok::<String, String>(String::new())
        });
    }
    rect
}


// â”€â”€ Text/font helpers (same translation as home.rs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn draw_text(
    ui: &Ui,
    at: egui::Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    let p = ui.painter();
    let galley = p.layout_no_wrap(text.to_string(), font, color);
    let size = galley.size();
    let anchor_y = match anchor.y() {
        Align::Min => 0.0,
        Align::Center => -size.y / 2.0,
        Align::Max => -size.y,
    };
    let anchor_x = match anchor.x() {
        Align::Min => 0.0,
        Align::Center => -size.x / 2.0,
        Align::Max => -size.x,
    };
    let origin = at + vec2(anchor_x, anchor_y);
    p.galley(origin, galley, color);
    Rect::from_min_size(origin, size)
}

fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font, Color32::WHITE)
        .size()
        .x
}

/// Draw text with CSS-style `letter-spacing` (egui has no such feature).
/// Characters are laid out one by one, each advance shrunk/grown by
/// `spacing` pixels. Kerning is not applied (acceptable for headings).
fn draw_text_spaced(
    ui: &Ui,
    at: egui::Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
    smear: f32,
) -> Rect {
    let p = ui.painter();
    // A per-character layout cannot render a cursive script: each letter would
    // be laid out on its own (so nothing joins) and the runs would go left to
    // right (so the word reads backwards). Hand the whole run to the shaper —
    // letter spacing is wrong for Arabic script anyway.
    if crate::rtl::has_rtl(text) {
        let g = p.layout_no_wrap(text.to_owned(), font, color);
        let r = anchor.anchor_size(at, g.size());
        p.galley(r.min, g, color);
        return r;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Rect::from_min_size(at, vec2(0.0, 0.0));
    }
    let galleys: Vec<std::sync::Arc<egui::Galley>> = chars
        .iter()
        .map(|c| p.layout_no_wrap(c.to_string(), font.clone(), color))
        .collect();
    let advances: Vec<f32> = galleys.iter().map(|g| g.size().x).collect();
    let total: f32 = advances.iter().sum::<f32>() + spacing * (chars.len() as f32 - 1.0);
    let height = galleys[0].size().y;
    let anchor_x = match anchor.x() {
        Align::Min => 0.0,
        Align::Center => -total / 2.0,
        Align::Max => -total,
    };
    let anchor_y = match anchor.y() {
        Align::Min => 0.0,
        Align::Center => -height / 2.0,
        Align::Max => -height,
    };
    let mut x = at.x + anchor_x;
    let y = at.y + anchor_y;
    for (g, adv) in galleys.iter().zip(advances.iter()) {
        p.galley(pos2(x, y), g.clone(), color);
        // Chromium-style synthetic bold: the original app bundles Nunito 400
        // only, so any weight above 400 renders as Regular advances with the
        // ink smeared `smear`px horizontally (Skia fake-bold stroke). emulating
        // it with a second galley paint keeps both the 400 advances and the
        // 400 outlines â€” the real 600/700 faces we have render visibly
        // different (their stems sit ~0.4px *inside* the faux-bold ink and are
        // thinner).
        if smear > 0.0 {
            p.galley(pos2(x + smear, y), g.clone(), color);
        }
        x += adv + spacing;
    }
    Rect::from_min_size(pos2(at.x + anchor_x, y), vec2(total, height))
}

fn bold(size: f32) -> FontId {
    FontId::new(size, tokens::typography::bold_family())
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, tokens::typography::semibold_family())
}

fn regular(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

/// bg-card fill + 1px border (`.download-list-card` surfaces).
fn paint_card(ui: &Ui, rect: Rect, r: f32, fill: Color32, stroke_c: Color32) {
    ui.painter().rect_filled(rect, CornerRadius::same(r as u8), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(r as u8),
        Stroke::new(1.0, stroke_c),
        StrokeKind::Inside,
    );
}

/// Press amount (0..1) for a download card while the pointer is held on it.
/// The caller nudges the card down and tints its border by this factor.
fn card_press(ui: &Ui, gid: &str, resp: &egui::Response) -> f32 {
    let press = ui.ctx().animate_bool_with_time(
        egui::Id::new("dlc_press").with(gid),
        resp.is_pointer_button_down_on(),
        0.10,
    );
    if press > 0.0 {
        ui.ctx().request_repaint();
    }
    press
}

/// Expanding accent ripple from the pointer when a card is pressed, clipped to
/// the card. Starts once per press and fades over `RIPPLE_SECS`.
fn card_ripple(ui: &Ui, gid: &str, rect: Rect, resp: &egui::Response, accent: Color32) {
    const RIPPLE_SECS: f64 = 0.5;
    let now = ui.input(|i| i.time);
    let id = egui::Id::new("dlc_ripple").with(gid);
    let mut anim = ui.data(|d| d.get_temp::<(Pos2, f64, bool)>(id));
    let down = resp.is_pointer_button_down_on();
    let was_down = anim.map(|a| a.2).unwrap_or(false);
    if down && !was_down {
        let origin = ui
            .ctx()
            .pointer_interact_pos()
            .map(|q| q.clamp(rect.min, rect.max))
            .unwrap_or_else(|| rect.center());
        anim = Some((origin, now, true));
        ui.data_mut(|d| d.insert_temp(id, (origin, now, true)));
    } else if !down && was_down {
        if let Some(a) = anim {
            ui.data_mut(|d| d.insert_temp(id, (a.0, a.1, false)));
        }
    }
    if let Some((origin, start, _)) = anim {
        let age = now - start;
        if (0.0..RIPPLE_SECS).contains(&age) {
            let k = (age / RIPPLE_SECS) as f32;
            let radius = (rect.width().max(rect.height()) * 0.9).max(1.0) * k;
            let alpha = (0.20 * (1.0 - k) * 255.0) as u8;
            ui.painter().with_clip_rect(rect).circle_filled(
                origin,
                radius,
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha),
            );
            ui.ctx().request_repaint();
        } else if age >= RIPPLE_SECS {
            ui.data_mut(|d| d.remove::<(Pos2, f64, bool)>(id));
        }
    }
}

// over an opaque backdrop is exactly a linear `from -> to` lerp along the
// top-left->bottom-right diagonal â€” so the caller pre-composites the from-stop
// over the underlying color (hero: `accent@.12 over surface`; orb/add:
// `accent@a over local hero color`). `c_bg` is the backdrop color visible
// outside the rounded corner (what the arc tips must be erased to).
//
// NOTE: rendered as a grid of solid fills, NOT a vertex mesh â€” egui's mesh
// renderer leaked triangles beyond the geometry in this build.
fn diag_gradient(ui: &Ui, rect: Rect, radius: u8, c_from: Color32, c_to: Color32, c_bg: Color32) {
    diag_gradient_tips(ui, rect, radius, c_from, c_to, c_bg, &TipErase::Flat);
}

/// How the corner *tips* (the strip between the rounded arc and the border-box
/// corner) are repainted after the gradient grid covers them. `Flat` = one
/// backdrop color â€” nothing else was painted there. `Glow` = a CSS
/// `box-shadow` glow sits over the tips (the shadow silhouette is the rounded
/// arc, so the strip lies *outside* it where the blur is still visible); the
/// erase must re-composite the glow per position or the corners read as dark
/// square patches (the reference orb).
enum TipErase<'a> {
    Flat,
    Glow {
        color: Color32,
        alpha: f32,
        blur: f32,
        base: &'a dyn Fn(f32, f32) -> Color32,
    },
}

fn diag_gradient_tips(
    ui: &Ui,
    rect: Rect,
    radius: u8,
    c_from: Color32,
    c_to: Color32,
    c_bg: Color32,
    tips: &TipErase<'_>,
) {
    let span = (rect.width() + rect.height()).max(1.0);
    // Finer cells on small elements: the hero's 12px cells are invisible at
    // its gradient scale, but on a 36px button they read as flat quadrants
    // (Î”R â‰ˆ 12 units). ~22 columns across keeps the max center-sample error
    // â‰ˆ 2% of the stop distance.
    let cell = (rect.width().max(rect.height()) / 22.0).clamp(1.2, 12.0);
    let cols = ((rect.width() / cell).ceil() as usize).max(1);
    let rows = ((rect.height() / cell).ceil() as usize).max(1);
    let pw = rect.width() / cols as f32;
    let ph = rect.height() / rows as f32;
    let painter = ui.painter();
    for iy in 0..rows {
        for ix in 0..cols {
            let x = rect.min.x + ix as f32 * pw;
            let y = rect.min.y + iy as f32 * ph;
            // Sample t at the cell center: halves the linear error.
            let t = ((x + pw * 0.5 - rect.min.x) + (y + ph * 0.5 - rect.min.y)) / span;
            // Overlap each cell by 0.6px so adjacent AA edges don't seam.
            let ex = if ix + 1 == cols { 0.0 } else { 0.6 };
            let ey = if iy + 1 == rows { 0.0 } else { 0.6 };
            painter.rect_filled(
                Rect::from_min_size(pos2(x, y), vec2(pw + ex, ph + ey)),
                CornerRadius::same(0),
                mk::blend(c_from, c_to, t),
            );
        }
    }
    // No corner caps: the grid covers the full rounded arc interior (cell
    // colors are exact for a linear gradient, and larger caps would paint
    // flat patches â€” on the 36px add button r=18 caps became whole quadrants).
    // Erase the square corner *tips*: within each rÃ—r corner square, the
    // region between the arc and the rect's corner point lies outside the
    // rounded fill â€” paint it with the backdrop so the rounding reads as
    // transparent (the grid also covers only the rect square).
    //
    // The tips are painted ROW BY ROW with a fractional width, exactly how
    // the browser rasterizes a rounded rect: the arc crossing of each pixel
    // row lands on a sub-pixel x, so the edge is smoothly anti-aliased.
    // (Stacking 1px tiles instead leaves a stepped, streaky edge.)
    let r = radius as f32;
    let rows = r.ceil() as i32;
    let painter = ui.painter();
    // One row band of one corner tip, arc centre (cx, cy). Flat repaints the
    // backdrop in a single quad; Glow slices the band along x (~1.5px) so each
    // slice carries the glow alpha at its own distance from the silhouette
    // (the profile falls ~0.01 alpha per px near the arc, so a whole-row quad
    // would paint the bright arc-adjacent strip flat).
    let tip = |x0: f32, x1: f32, yc: f32, cx: f32, cy: f32| {
        if x1 <= x0 {
            return;
        }
        match tips {
            TipErase::Flat => {
                painter.rect_filled(
                    Rect::from_min_size(pos2(x0, yc - 0.5), vec2(x1 - x0, 1.0)),
                    CornerRadius::same(0),
                    c_bg,
                );
            }
            TipErase::Glow { color, alpha, blur, base } => {
                let sigma = (blur / 2.0).max(1.0);
                let mut x = x0;
                while x < x1 - 0.001 {
                    let e = (x + 1.5).min(x1);
                    let xc = (x + e) * 0.5;
                    let d = ((cx - xc).hypot(cy - yc) - r).max(0.0);
                    let a = alpha * (1.0 - gauss_cdf(d / sigma));
                    let col = if a <= 0.004 {
                        base(xc, yc)
                    } else {
                        mk::blend(base(xc, yc), *color, a)
                    };
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x, yc - 0.5), vec2(e - x, 1.0)),
                        CornerRadius::same(0),
                        col,
                    );
                    x = e;
                }
            }
        }
    };
    for i in 0..rows {
        let dy = i as f32 + 0.5;
        let s = (r * r - (r - dy) * (r - dy)).max(0.0).sqrt();
        // nw / ne: the top two corner squares.
        let y = rect.min.y + dy;
        let xl = rect.min.x + r - s;
        tip(rect.min.x, xl, y, rect.min.x + r, rect.min.y + r);
        tip(rect.max.x - r + s, rect.max.x, y, rect.max.x - r, rect.min.y + r);
        // sw / se: the bottom two corner squares.
        let yb = rect.max.y - dy;
        tip(rect.min.x, rect.min.x + r - s, yb, rect.min.x + r, rect.max.y - r);
        tip(rect.max.x - r + s, rect.max.x, yb, rect.max.x - r, rect.max.y - r);
    }
}

/// Diagonal (135Â°, `linear-gradient(135deg, ..)`) fill of a FULL ROUND
/// element (radius == half the box, e.g. the 36px add button). The grid +
/// row-erase path of `diag_gradient` can't render this: the erase quads are
/// 1px tall, where epaint's feather degenerates (the arc edges break into
/// missing strips / twin-18% plateaus). One fan mesh instead: the stop lerp
/// is LINEAR in (x + y), so sampling it at the vertices reproduces the exact
/// gradient across every triangle (Gouraud), and a 1px premultiplied fade
/// ring supplies the arc anti-aliasing.
fn circle_gradient(painter: &egui::Painter, rect: Rect, c_from: Color32, c_to: Color32) {
    let span = (rect.width() + rect.height()).max(1.0);
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    let grad = |p: Pos2| -> Color32 {
        let t = (((p.x - rect.min.x) + (p.y - rect.min.y)) / span).clamp(0.0, 1.0);
        mk::blend(c_from, c_to, t)
    };
    let n = 64usize;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, grad(c));
    // Arc vertex is index 2i+1, its 1px feather ring vertex 2i+2.
    for i in 0..n {
        let a = i as f32 * std::f32::consts::TAU / n as f32;
        let dir = egui::Vec2::new(a.cos(), a.sin());
        // Fan stops 0.5px in, feather reaches 0.5px out: at pixel centers the
        // ramp then reads Î± = clamp(0.5 âˆ’ signed_dist) â€” exactly the linear
        // per-pixel coverage a straight edge rasterizes with (the old fan at
        // the raw radius painted whole spurious 0.5-alpha halos along rows
        // the circle merely grazes).
        let p = c + (r - 0.5) * dir;
        mesh.colored_vertex(p, grad(p));
        mesh.colored_vertex(c + (r + 0.5) * dir, Color32::TRANSPARENT);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ai, ao, bi, bo) = (2 * i as u32 + 1, 2 * i as u32 + 2, 2 * j as u32 + 1, 2 * j as u32 + 2);
        // Fan + feather quads, one rotational sense throughout.
        mesh.add_triangle(0, ai, bi);
        mesh.add_triangle(ai, ao, bo);
        mesh.add_triangle(ai, bo, bi);
    }
    // The radial feather bands reach 1px beyond the circle â€” i.e. outside the
    // button at the four tangent rows/cols (top, bottom, left, right differ
    // from zero only there). Chromium's analytic coverage is 0/full across
    // those exact-tangent rows, so clip the mesh to the box: the band keeps
    // the anti-aliased arc streaks and everything outside adds nothing.
    painter
        .with_clip_rect(rect)
        .add(mesh);
}


/// Soft accent glow: CSS `box-shadow 0 <oy> <blur> rgba(accent,a)`.
/// CSS paints a blurred, FILLED silhouette (Ïƒ = blur/2), so the visible
/// profile outside it is Î±Â·(1 âˆ’ Î¦(d/Ïƒ)) per 1px band [ref FAB, x = centre:
/// Î”R 51/45/38/33/28/23 at 1..6px below the 36px circle] and the sliver
/// between the element's own edge and the silhouette stays â‰ˆ Î±Â·Î¦(0.5/Ïƒ)
/// [ref Î”R 39/31 under the button's bottom arc]. Emulated as the silhouette
/// fill painted first, then disjoint 1px stroke bands outside-in â€” a glow is
/// symmetric on all sides (unlike the card shadow) and composites over
/// whatever backdrop: the hero gradient for the orb, the toolbar for the
/// add button. Chromium's two-pass blur conserves â‰ˆ80% of the nominal alpha
/// at this shape size, so call sites pre-scale (.30 nominal â†’ .24 measured).
fn css_glow(
    ui: &Ui,
    rect: Rect,
    radius: f32,
    oy: f32,
    blur: f32,
    color: Color32,
    alpha: f32,
) {
    let sigma = (blur / 2.0).max(1.0);
    let ext = (sigma * 2.4).ceil() as i32;
    // The shadow silhouette is the box translated down by `oy`.
    let shape = Rect::from_min_size(pos2(rect.min.x, rect.min.y + oy), rect.size());
    let painter = ui.painter();
    // Interior fill first: with a ring-only silhouette the dy sliver between
    // the element edge and its shadow (2px here) read as a dark hole.
    let a_fill = alpha * gauss_cdf(0.5 / sigma);
    painter.rect_filled(
        shape,
        CornerRadius::same(radius as u8),
        Color32::from_rgba_premultiplied(
            (color.r() as f32 * a_fill) as u8,
            (color.g() as f32 * a_fill) as u8,
            (color.b() as f32 * a_fill) as u8,
            (color.a() as f32 * a_fill) as u8,
        ),
    );
    for i in 0..ext {
        let d = i as f32 + 0.5;
        let a = alpha * (1.0 - gauss_cdf(d / sigma));
        if a <= 0.004 {
            break;
        }
        let col = Color32::from_rgba_premultiplied(
            (color.r() as f32 * a) as u8,
            (color.g() as f32 * a) as u8,
            (color.b() as f32 * a) as u8,
            (color.a() as f32 * a) as u8,
        );
        painter.rect_stroke(
            shape.expand(d),
            CornerRadius::same((radius + d) as u8),
            Stroke::new(1.0, col),
            // `Middle` centres the 1px band on the distance d; `Inside` would
            // push it a half pixel further out and halve the profile.
            StrokeKind::Middle,
        );
    }
}

/// Î¦(x) â€” standard normal CDF (Abramowitzâ€“Stegun erf approximation).
fn gauss_cdf(x: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741
                    + t * (-1.453_152_027 + t * 1.061_405_429))));
    let erf = x.signum() * (1.0 - poly * (-(x * x)).exp());
    (0.5 + 0.5 * erf).clamp(0.0, 1.0)
}

// â”€â”€ Hero (`DownloadsPage.vue` copies SettingsPage hero) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Icon glyphs of the 3 hero filter buttons (all/downloading/completed).
const FILTER_ICONS: [&str; 3] = ["inbox", "download", "done"];

fn hero(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, w: f32) {
    let p = mk::pal();
    // `.settings-hero`: pad 16 28, radius 20, 1px border-strong.
    // Content: h1 line (32 * 1.1 = 35.2) + 10 + filter/bar 48 = 125.2 + 2px
    // borders â‰ˆ 127 total height.
    let hero_h = 129.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, hero_h), Sense::hover());
    // Dark-mode `.settings-hero`: linear-gradient(135deg, rgba(accent,.12),
    // var(--bg-card)) â€” pre-composite the from-stop over the surface below,
    // then lerp to bg_card along the diagonal (matches ref pixels).
    let hero_from = if p.is_light { 0.16 } else { 0.12 };
    let hero_c0 = mk::blend(p.bg_surface, p.accent, hero_from);
    // No box-shadow under the hero (decided 2026-10-22). Only the 1px ring
    // from --shadow-card remains, painted as a true transparent wash so it
    // composites over whatever is below: white .03 in dark, `--border` in
    // light (plus a 1px 2px rgba(16,24,40,.04) drop).
    if p.is_light {
        crate::ui::shade::soft_shadow(ui, rect, 20, Color32::from_rgb(16, 24, 40), 1.0, 2.0, 10);
    }
    ui.painter().rect_stroke(
        rect.expand(1.0),
        CornerRadius::same(21),
        Stroke::new(
            1.0,
            if p.is_light {
                p.border
            } else {
                egui::Color32::from_white_alpha(8)
            },
        ),
        StrokeKind::Inside,
    );
    diag_gradient_tips(
        ui,
        rect,
        20,
        hero_c0,
        p.bg_card,
        p.bg_surface,
        &TipErase::Flat,
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(20),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );

    // `.hero-orb`: 56Ã—56 r16 gradient rgba(accent,.25)â†’.08 over the hero's
    // top-left color + accent .18 border. The orb lives in `.hero-icon-area`,
    // vertically centred inside the `.hero-left` block. Plus the orbPulse
    // box-shadow, whose backdrop is the hero's local gradient colour.
    //
    // Hero box model (CSS): padding 16/28; the content box is
    // `hero height (129) âˆ’ 2Ã—16 = 97` and holds the `.hero-left` block
    // (h1 35.2 + 10 margin + filter bar 50 = 95.2) centred in it, so the copy
    // starts 0.9px below the padding edge. Reference capture: titlebar 51
    // (50 + 1px border), hero 69..198, filter bar 131..181.
    let copy_h = 35.2 + 10.0 + 50.0;
    let copy_top = 16.0 + ((129.0 - 32.0) - copy_h) / 2.0;
    let orb = Rect::from_min_size(
        pos2(
            rect.min.x + 28.0,
            rect.min.y + copy_top + (copy_h - 56.0) / 2.0,
        ),
        vec2(56.0, 56.0),
    );
    let hero_span = rect.width() + rect.height();
    let hero_local = |x: f32, y: f32| -> Color32 {
        let t = (((x - rect.min.x) + (y - rect.min.y)) / hero_span).clamp(0.0, 1.0);
        mk::blend(hero_c0, p.bg_card, t)
    };
    // The reference capture froze the orbPulse animation at its 50% keyframe
    // (box-shadow: 0 0 24px rgba(accent,.3)) â€” the measured ramp fits
    // Ïƒ=12/Î±=.3 exactly (Î”Râ‰ˆ23 at 4px out, â‰ˆ10 at 12px out).
    css_glow(ui, orb, 16.0, 0.0, 24.0, p.accent, 0.30);
    diag_gradient_tips(
        ui,
        orb,
        16,
        mk::blend(hero_c0, p.accent, 0.25),
        mk::blend(hero_c0, p.accent, 0.08),
        hero_c0,
        // The corner tips sit outside the orb's rounded arc, where the CSS
        // box-shadow still paints â€” re-composite the pulse glow over the
        // hero's local gradient instead of erasing to flat `hero_c0` (that
        // wiped the glow and read as dark square corner patches).
        &TipErase::Glow {
            color: p.accent,
            alpha: 0.30,
            blur: 24.0,
            base: &hero_local,
        },
    );
    ui.painter().rect_stroke(
        orb,
        CornerRadius::same(16),
        Stroke::new(1.0, mk::blend(hero_c0, p.accent, 0.18)),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        // Chromium rasters the glyph 1px right of egui's integer snap.
        pos2(orb.center().x + 1.0, orb.center().y),
        "cloud_download",
        28.0,
        p.accent_strong,
    );

    // `h1`: clamp(1.5rem,3vw,2rem) = 32px at this window, 800, lh 1.1,
    // letter-spacing -.03em (â‰ˆ -0.96px per char). The old app only bundles
    // Nunito 400, so Chromium renders the 800 request as Regular advances
    // + synthetic-bold ink; emulate by double-painting (a real SemiBold/
    // Bold face renders visibly different â€” its stems sit ~0.4px inside the
    // faux ink at the same advances). Basline sits 1px above egui's line-box
    // center the way Chromium snaps the 35.2 box.
    draw_text_spaced(
        ui,
        pos2(rect.min.x + 104.0, rect.min.y + copy_top + 35.2 / 2.0 - 0.45),
        Align2::LEFT_CENTER,
        state.t("Downloads"),
        regular(32.0),
        p.text,
        -0.03 * 32.0,
        // Chromium fakes the 800 weight on the bundled 400 face with a
        // horizontal smear; measured against the reference the stem gains
        // â‰ˆ1.6px (32px size).
        1.6,
    );

    // `.filter-bar` (left, under h1): pad 10 6, 3x28px icon buttons + gap 2,
    // 1px border => 28 + 20 + 2 = 50 tall.
    let fb = Rect::from_min_size(
        pos2(rect.min.x + 104.0, rect.min.y + copy_top + 35.2 + 10.0),
        vec2(100.0, 50.0),
    );
    filter_bar(state, ui, fb, "dl_filter");

    // `.hero-bottom-bar`: bar-box, bottom 16 right 16 (popover toolbar).
    // Outer size = 2 borders + 12 padding + (9x28 buttons + 3x17 dividers +
    // 36 FAB + 12x2 flex gaps = 363) = 377x50.
    let bar = Rect::from_min_size(
        pos2(
            (rect.max.x - 1.0 - 16.0 - BAR_W).round(),
            (rect.min.y + hero_h - 1.0 - 16.0 - BAR_H).round(),
        ),
        vec2(BAR_W, BAR_H),
    );
    // `.bar-box` fill/border are translucent over the hero gradient. The add
    // button's gradient stops need the opaque local backdrop there:
    // `rgba(accent,.7)` over `bar-base` = local hero color + .15 black.
    let bar_t = ((bar.center().x - rect.min.x) + (bar.center().y - rect.min.y))
        / (rect.width() + rect.height());
    let bar_base = if p.is_light {
        // light `--toolbar-bg: rgba(20,24,34,.04)`
        mk::blend(
            mk::blend(hero_c0, p.bg_card, bar_t),
            Color32::from_rgb(20, 24, 34),
            0.04,
        )
    } else {
        mk::blend(mk::blend(hero_c0, p.bg_card, bar_t), Color32::BLACK, 0.15)
    };
    let sort_rect = bar_box(state, ctx, ui, bar, false, bar_base, "dl_hero");

    // The `.sort-menu` popover is drawn once in `show()`: the last bar to lay
    // out stores its button rect here, so the sticky toolbar wins when both
    // exist.
    ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_sort_anchor"), sort_rect));
}

// --- Toolbar colours (CSS variables) -------------------------------------

/// `--toolbar-bg`: rgba(0,0,0,.15) dark, rgba(20,24,34,.04) light.
fn toolbar_bg(p: &crate::ui::tokens::Palette) -> Color32 {
    if p.is_light {
        Color32::from_rgba_unmultiplied(20, 24, 34, 10)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 38)
    }
}

/// `--toolbar-border`: rgba(255,255,255,.04) dark, rgba(20,24,34,.06) light.
fn toolbar_border(p: &crate::ui::tokens::Palette) -> Color32 {
    if p.is_light {
        Color32::from_rgba_unmultiplied(20, 24, 34, 15)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    }
}

/// `--toolbar-active`: white .06 in dark, opaque #ffffff in light.
fn toolbar_active(p: &crate::ui::tokens::Palette) -> Color32 {
    if p.is_light {
        Color32::WHITE
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 15)
    }
}

/// `.filter-bar`: 100x50 box with the sliding 28x28 indicator and 3 buttons.
fn filter_bar(state: &mut UiState, ui: &mut Ui, fb: Rect, salt: &str) {
    let p = mk::pal();
    ui.painter()
        .rect_filled(fb, CornerRadius::same(10), toolbar_bg(&p));
    ui.painter().rect_stroke(
        fb,
        CornerRadius::same(10),
        Stroke::new(1.0, toolbar_border(&p)),
        StrokeKind::Inside,
    );
    let idx = match state.filter {
        Filter::All => 0,
        Filter::Downloading => 1,
        Filter::Completed => 2,
    };
    // `.filter-btn-indicator { transition: transform .22s }` — pos-N slides one
    // button (28 + 2 gap) at a time.
    let ind_x = ui.ctx().animate_value_with_time(
        egui::Id::new("dl_filter_ind").with(salt),
        fb.min.x + 7.0 + idx as f32 * 30.0,
        0.22,
    );
    let ind = Rect::from_min_size(pos2(ind_x, fb.min.y + 11.0), vec2(28.0, 28.0));
    if p.is_light {
        // --toolbar-active-shadow: 0 1px 3px rgba(16,24,40,.10) + 1px ring .05.
        ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
            ind,
            6,
            Color32::from_rgb(16, 24, 40),
            1.0,
            3.0,
            26,
        ));
        ui.painter().rect_stroke(
            ind,
            CornerRadius::same(6),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(16, 24, 40, 13)),
            StrokeKind::Inside,
        );
    } else {
        // 0 1px 4px rgba(0,0,0,.15).
        ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
            ind,
            6,
            Color32::BLACK,
            1.0,
            4.0,
            38,
        ));
    }
    ui.painter()
        .rect_filled(ind, CornerRadius::same(6), toolbar_active(&p));
    for (i, icon) in FILTER_ICONS.iter().enumerate() {
        let b = Rect::from_min_size(
            pos2(fb.min.x + 7.0 + i as f32 * 30.0, fb.min.y + 11.0),
            vec2(28.0, 28.0),
        );
        let id = egui::Id::new("dl_filter").with(salt).with(i);
        let r = ui.interact(b, id, Sense::click());
        let active = i == idx;
        let color = if active || r.hovered() {
            p.accent_strong
        } else {
            p.text_muted
        };
        mi::paint_centered(
            ui.painter(),
            // Chromium rasters these glyphs 1px right of egui's integer snap.
            pos2(b.center().x + 1.0, b.center().y),
            icon,
            18.0,
            color,
        );
        if r.clicked() {
            state.filter = match i {
                0 => Filter::All,
                1 => Filter::Downloading,
                _ => Filter::Completed,
            };
        }
        let tip = match i {
            0 => "All",
            1 => "Downloading",
            _ => "Completed",
        };
        r.on_hover_text(state.t(tip));
    }
}

/// `.bar-box` action row. `sticky` drops the divider after the two view
/// buttons (the sticky toolbar shares one group there). Returns the sort
/// button rect so the caller can anchor `.sort-menu`.
fn bar_box(
    state: &mut UiState,
    ctx: &Ctx,
    ui: &mut Ui,
    bar: Rect,
    sticky: bool,
    base: Color32,
    salt: &str,
) -> Rect {
    let p = mk::pal();
    ui.painter()
        .rect_filled(bar, CornerRadius::same(10), toolbar_bg(&p));
    ui.painter().rect_stroke(
        bar,
        CornerRadius::same(10),
        Stroke::new(1.0, toolbar_border(&p)),
        StrokeKind::Inside,
    );

    // Buttons: [list][stream] | [open_with][sort] | [pause][play][sweep] |
    // [folder][batch] + add — x positions tracked exactly like the CSS flex
    // row (gap 2, dividers 1 + 8+8 margins = 17), content starts 7px inside
    // the box (1px border + 6px padding).
    let mut x = bar.min.x + 7.0;
    let btn = |ui: &Ui, rect: Rect, icon: &str, active: bool, id: &str, tip: &str| -> bool {
        let r = ui.interact(rect, egui::Id::new(id).with(salt), Sense::click());
        let color = if active || r.hovered() {
            p.accent_strong
        } else {
            p.text_muted
        };
        if active {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), toolbar_active(&p));
        }
        mi::paint_centered(ui.painter(), rect.center(), icon, 18.0, color);
        let clicked = r.clicked();
        r.on_hover_text(tip);
        clicked
    };
    let list_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, list_rect, "view_list", !state.view_compact, "dl_view_list", state.t("List View")) {
        state.view_compact = false;
    }
    x += 30.0;
    let cs_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, cs_rect, "view_stream", state.view_compact, "dl_view_stream", state.t("Compact View")) {
        state.view_compact = true;
    }
    x += 30.0;
    if !sticky {
        // `.bar-divider` #1: 1px x 20 centered in the 36px content row.
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(x + 8.0, bar.min.y + 15.0), vec2(1.0, 20.0)),
            CornerRadius::same(0),
            Color32::from_rgba_unmultiplied(255, 255, 255, 51), // #fff3
        );
        x += 17.0 + 2.0;
    }
    let grab_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, grab_rect, "open_with", state.grab_enabled, "dl_grab", state.t("Grab Mode")) {
        state.grab_enabled = !state.grab_enabled;
    }
    x += 30.0;
    let sort_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, sort_rect, "sort", false, "dl_sort", state.t("Sort")) {
        state.sort_menu_open = !state.sort_menu_open;
    }
    x += 28.0 + 2.0;
    // `.bar-divider` #2 after [open_with][sort].
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(x + 8.0, bar.min.y + 15.0), vec2(1.0, 20.0)),
        CornerRadius::same(0),
        Color32::from_rgba_unmultiplied(255, 255, 255, 51),
    );
    x += 17.0 + 2.0;
    let pause_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, pause_rect, "pause", false, "dl_pause_all", state.t("Pause All")) {
        let gids: Vec<String> = state
            .downloads
            .iter()
            .filter(|d| d.status == Status::Active)
            .map(|d| d.gid.clone())
            .collect();
        ctx.run(move |_| async move {
            // Vue's `pauseAll` fans out per item and toasts the first failure.
            let mut err = None;
            for gid in gids {
                if let Err(e) = moonhunt_core::aria2::pause_download(gid.clone()).await {
                    err.get_or_insert(e);
                }
            }
            match err {
                Some(e) => Err(format!("Failed to pause: {e}")),
                None => Ok(String::new()),
            }
        });
    }
    x += 30.0;
    let play_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, play_rect, "play_arrow", false, "dl_resume_all", state.t("Resume All")) {
        let gids: Vec<String> = state
            .downloads
            .iter()
            .filter(|d| d.status == Status::Paused)
            .map(|d| d.gid.clone())
            .collect();
        ctx.run(move |_| async move {
            let mut err = None;
            for gid in gids {
                if let Err(e) = moonhunt_core::aria2::resume_download(gid.clone()).await {
                    err.get_or_insert(e);
                }
            }
            match err {
                Some(e) => Err(format!("Failed to resume: {e}")),
                None => Ok(String::new()),
            }
        });
    }
    x += 30.0;
    let sweep_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, sweep_rect, "delete_sweep", false, "dl_sweep", state.t("Clear History")) {
        // Clear every card: terminal history plus active/waiting/paused rows,
        // and the group containers. Marking cancelled first keeps the poller
        // from re-emitting rows while the async removals catch up.
        let gids: Vec<String> = state.downloads.iter().map(|d| d.gid.clone()).collect();
        for gid in &gids {
            moonhunt_core::extension::mark_cancelled(gid);
        }
        state.downloads.clear();
        state.groups.clear();
        crate::groups::save_groups(&state.groups);
        state.g_edit = None;
        state.g_create_open = false;
        state.g_dl_open = false;
        state.g_menu_gid = None;
        state.g_dd_open = None;
        state.g_pending.clear();
        state.drag_gid = None;
        state.drag_from_group = None;
        state.drag_group = None;
        ctx.run(move |app| async move {
            for gid in gids {
                let _ = moonhunt_core::aria2::remove_download(app.clone(), gid.clone()).await;
                // Also covers rows aria2 already forgot about.
                let _ = moonhunt_core::db::remove_download_from_db(app.clone(), gid.clone());
            }
            Ok::<String, String>("History cleared".to_string())
        });
    }
    x += 28.0 + 2.0;
    // `.bar-divider` #3 after [pause][play][sweep].
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(x + 8.0, bar.min.y + 15.0), vec2(1.0, 20.0)),
        CornerRadius::same(0),
        Color32::from_rgba_unmultiplied(255, 255, 255, 51),
    );
    x += 17.0 + 2.0;
    let group_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    // `@click="showCreateGroupDialog = true"` — the create-group dialog.
    if btn(ui, group_rect, "folder_shared", false, "dl_group", state.t("Create Group")) {
        state.g_create_open = true;
        // The dialog's number inputs seed from the stored values.
        state.g_speed_text = state.g_create_speed.to_string();
        state.g_dl_value_text = state.g_create_dl_value.to_string();
        state.g_dd_open = None;
    }
    x += 30.0;
    let batch_rect = Rect::from_min_size(pos2(x, bar.min.y + 11.0), vec2(28.0, 28.0));
    if btn(ui, batch_rect, "playlist_add", false, "dl_batch", state.t("Batch Download")) {
        state.show_batch_dialog = true;
    }
    x += 28.0 + 2.0;
    // `.btn-add-icon`: 36x36 accent gradient circle, dark `add` glyph.
    // box-shadow: 0 2px 8px rgba(var(--accent-rgb),.3).
    let add = Rect::from_min_size(pos2(x, bar.min.y + 7.0), vec2(36.0, 36.0));
    css_glow(ui, add, 18.0, 2.0, 8.0, p.accent, 0.24);
    circle_gradient(ui.painter(), add, p.accent, mk::blend(base, p.accent, 0.7));
    let add_r = ui.interact(add, egui::Id::new("dl_add").with(salt), Sense::click());
    if add_r.clicked() {
        state.show_new_download = true;
        state.nd_focus_next = true;
    }
    add_r.on_hover_text(state.t("New Download"));
    mi::paint_centered(
        ui.painter(),
        add.center(),
        "add",
        20.0,
        Color32::from_rgb(0x0a, 0x0e, 0x1a),
    );

    sort_rect
}

/// `.sticky-toolbar` — fixed strip under the titlebar once the list scrolls
/// past 160px: filter bar left, action row right.
/// shortcut: `backdrop-filter: blur(12px)` is skipped.
fn sticky_toolbar(state: &mut UiState, ctx: &Ctx, ui: &mut Ui) {
    let p = mk::pal();
    let screen = ui.ctx().content_rect();
    // `top: 50px; left: var(--sidebar-width, 72px); right: 0`, padding 6 14,
    // 1px bottom border; the 50px filter bar sets the content height (62+1).
    let sb = Rect::from_min_size(
        pos2(screen.min.x + 72.0, screen.min.y + 50.0),
        vec2(screen.width() - 72.0, 63.0),
    );
    // shortcut: `backdrop-filter: blur(12px)` is skipped, so the wash alone
    // would let card text through unchanged; keep the frosted look by laying
    // the surface colour down at ~92% first, then the `--toolbar-bg` wash.
    let under = Color32::from_rgba_unmultiplied(
        p.bg_surface.r(),
        p.bg_surface.g(),
        p.bg_surface.b(),
        235,
    );
    ui.painter()
        .rect_filled(sb, CornerRadius::same(0), under);
    ui.painter()
        .rect_filled(sb, CornerRadius::same(0), toolbar_bg(&p));
    ui.painter().hline(
        sb.x_range(),
        sb.max.y - 0.5,
        Stroke::new(1.0, toolbar_border(&p)),
    );
    let fb = Rect::from_min_size(pos2(sb.min.x + 14.0, sb.min.y + 6.0), vec2(100.0, 50.0));
    filter_bar(state, ui, fb, "dl_sticky");
    let bar = Rect::from_min_size(
        pos2(sb.max.x - 14.0 - STICKY_BAR_W, sb.min.y + 6.0),
        vec2(STICKY_BAR_W, BAR_H),
    );
    // Backdrop under the bar: the toolbar wash over the surface below it.
    let base = mk::blend(p.bg_surface, Color32::BLACK, if p.is_light { 0.0 } else { 0.15 });
    let sort_rect = bar_box(state, ctx, ui, bar, true, base, "dl_sticky");
    ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_sort_anchor"), sort_rect));
}

/// `.scroll-top-btn`: 44px circle, bottom/right 24, appears past 400px.
fn scroll_top_button(ui: &mut Ui) {
    let p = mk::pal();
    let screen = ui.ctx().content_rect();
    let c = pos2(screen.max.x - 24.0 - 22.0, screen.max.y - 24.0 - 22.0);
    let r = Rect::from_center_size(c, vec2(44.0, 44.0));
    let resp = ui.interact(r, egui::Id::new("dl_scroll_top"), Sense::click());
    // `.scroll-top-btn { box-shadow: var(--shadow-lg) }` — black .45 in dark,
    // rgba(16,24,40,.12) in light.
    let (ink, a) = if p.is_light {
        (Color32::from_rgb(16, 24, 40), 33)
    } else {
        (Color32::BLACK, 90)
    };
    crate::ui::shade::soft_shadow(ui, r, 22, ink, 4.0, 16.0, a);
    if resp.hovered() {
        ui.painter()
            .circle_filled(c, 22.0, p.accent);
        ui.painter().circle_stroke(c, 21.5, Stroke::new(1.0, p.accent_strong));
        mi::paint_centered(
            ui.painter(),
            c,
            "keyboard_arrow_up",
            24.0,
            Color32::from_rgb(0x0a, 0x0e, 0x1a),
        );
    } else {
        ui.painter().circle_filled(c, 22.0, p.bg_card);
        ui.painter()
            .circle_stroke(c, 21.5, Stroke::new(1.0, p.border_strong));
        mi::paint_centered(ui.painter(), c, "keyboard_arrow_up", 24.0, p.text);
    }
    if resp.clicked() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("dl_scroll_to_top"), true));
    }
}

/// `.hero-bottom-bar .bar-box`: 377x50 (three dividers).
const BAR_W: f32 = 377.0;
const BAR_H: f32 = 50.0;
/// Sticky variant has one divider fewer (its view buttons share a group with
/// `open_with`): 377 - 17 - 2 = 358.
const STICKY_BAR_W: f32 = 358.0;

const SORT_KEYS: [(SortKey, &str); 5] = [
    (SortKey::Added, "Date Added"),
    (SortKey::Name, "Name"),
    (SortKey::Size, "Size"),
    (SortKey::Completed, "Date Completed"),
    (SortKey::Manual, "Manual"),
];

/// `.sort-menu`: 160px min width, 6px padding, 1px border, r10, 6px below the
/// button; labels 10px/700 with .06em tracking at 60% opacity, items 13px with
/// 7px 10px padding (31.7px tall). Returns the painted rect so the caller can
/// close it on an outside click.
fn sort_menu(state: &mut UiState, ui: &mut Ui, sort_btn: Rect) -> Rect {
    let p = mk::pal();
    let menu_w = 160.0;
    const ITEM_H: f32 = 7.0 + 13.0 * 1.364 + 7.0;
    const LABEL_H: f32 = 6.0 + 10.0 * 1.364 + 4.0;
    let h = 6.0 + LABEL_H + ITEM_H * 5.0 + 9.0 + LABEL_H + ITEM_H * 2.0 + 6.0 + 2.0;
    let outer = Rect::from_min_size(
        pos2(sort_btn.max.x - menu_w, sort_btn.max.y + 6.0),
        vec2(menu_w, h),
    );
    egui::Area::new(egui::Id::new("dl_sort_menu"))
        .fixed_pos(outer.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            let (menu, _) = ui.allocate_exact_size(outer.size(), Sense::click());
            let (ink, a) = if p.is_light {
                (Color32::from_rgb(16, 24, 40), 33)
            } else {
                (Color32::BLACK, 110)
            };
            crate::ui::shade::soft_shadow(ui, menu, 10, ink, 8.0, 24.0, a);
            paint_card(ui, menu, 10.0, p.bg_card, p.border_strong);
            let mut y = menu.min.y + 1.0 + 6.0;
            let label = |ui: &Ui, y: f32, text: &str| {
                // `.sort-menu-label { opacity: .6 }` — dim the token, not the ink.
                let c = Color32::from_rgba_unmultiplied(
                    p.text_dim.r(),
                    p.text_dim.g(),
                    p.text_dim.b(),
                    153,
                );
                draw_text_spaced(
                    ui,
                    pos2(menu.min.x + 17.0, y + 6.0 + 10.0 * 1.364 / 2.0),
                    Align2::LEFT_CENTER,
                    text,
                    regular(10.0),
                    c,
                    0.06 * 10.0,
                    10.0 / 24.0,
                );
            };
            label(ui, y, state.t("SORT BY"));
            y += LABEL_H;
            for (key, text) in SORT_KEYS {
                let text = state.t(text);
                let r = Rect::from_min_size(
                    pos2(menu.min.x + 7.0, y),
                    vec2(menu_w - 14.0, ITEM_H),
                );
                let rr = ui.interact(r, egui::Id::new("dl_smi").with(text), Sense::click());
                if rr.hovered() {
                    ui.painter().rect_filled(
                        r,
                        CornerRadius::same(6),
                        mk::blend(p.bg_card, p.accent, 0.06),
                    );
                }
                let active = state.sort == key;
                draw_text(
                    ui,
                    pos2(r.min.x + 10.0, r.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    if active { semibold(13.0) } else { regular(13.0) },
                    if active { p.accent_strong } else { p.text },
                );
                if rr.clicked() {
                    state.sort = key;
                    // `setSortOption` closes the menu; `setSortAscending` doesn't.
                    state.sort_menu_open = false;
                }
                y += ITEM_H;
            }
            // `.sort-menu-divider { height: 1px; margin: 4px 8px }`.
            y += 4.0;
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(menu.min.x + 8.0, y), vec2(menu_w - 16.0, 1.0)),
                CornerRadius::same(0),
                p.border_strong,
            );
            y += 1.0 + 4.0;
            label(ui, y, state.t("ORDER"));
            y += LABEL_H;
            for (asc, text) in [(true, "Ascending"), (false, "Descending")] {
                let text = state.t(text);
                let r = Rect::from_min_size(
                    pos2(menu.min.x + 7.0, y),
                    vec2(menu_w - 14.0, ITEM_H),
                );
                let rr = ui.interact(r, egui::Id::new("dl_smo").with(text), Sense::click());
                if rr.hovered() {
                    ui.painter().rect_filled(
                        r,
                        CornerRadius::same(6),
                        mk::blend(p.bg_card, p.accent, 0.06),
                    );
                }
                let active = state.sort_ascending == asc;
                draw_text(
                    ui,
                    pos2(r.min.x + 10.0, r.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    if active { semibold(13.0) } else { regular(13.0) },
                    if active { p.accent_strong } else { p.text },
                );
                if rr.clicked() {
                    state.sort_ascending = asc;
                }
                y += ITEM_H;
            }
        });
    outer
}

// â”€â”€ Card data snapshot (breaks the borrow before actions run) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

struct CardView {
    gid: String,
    filename: String,
    path: String,
    status: Status,
    is_yt: bool,
    progress: f32,
    speed: String,
    completed: u64,
    total: u64,
    size_text: String,
    eta: String,
    connections: Option<u32>,
    error: String,
    /// `fileState` of completed rows (`moved` / `removed` render a badge).
    file_state: String,
}

impl CardView {
    fn of(d: &crate::state::DownloadVm) -> Self {
        Self {
            gid: d.gid.clone(),
            filename: d.filename.clone(),
            path: d.path.clone(),
            status: d.status,
            is_yt: d.is_yt,
            progress: d.progress,
            speed: d.speed_display(),
            completed: d.completed,
            total: d.total,
            size_text: d.size_text.clone(),
            eta: d.eta_display(),
            connections: d.connections,
            error: if d.error_message.is_empty() {
                d.error_code.clone()
            } else {
                d.error_message.clone()
            },
            file_state: d.file_state.clone(),
        }
    }

    /// (glyph, color, label) of the status pill (`getStatusIcon`/getTextStyle).
    fn pill(&self) -> (&'static str, Color32, &'static str) {
        let p = mk::pal();
        match self.status {
            Status::Active => ("play_circle", p.status_active, "Active"),
            Status::Waiting => ("schedule", p.status_waiting, "Waiting"),
            Status::Paused => ("pause_circle", p.status_paused, "Paused"),
            Status::Complete => ("check_circle", p.status_complete, "Completed"),
            Status::Error => ("error", p.status_error, "Failed"),
            Status::Removed => ("cancel", p.status_cancelled, "Cancelled"),
        }
    }

    /// `statusBg` in `DownloadListCard.vue` — literal RGBs (the dark-theme
    /// status hues), *not* the `--status-*` tokens, so the wash stays the same
    /// in light mode while the text colour flips.
    fn pill_bg(&self) -> Color32 {
        let c = match self.status {
            Status::Active => (79, 195, 247),
            Status::Waiting => (100, 181, 246),
            Status::Paused => (255, 167, 38),
            Status::Complete => (129, 199, 132),
            Status::Error => (239, 83, 80),
            Status::Removed => (158, 158, 158),
        };
        Color32::from_rgb(c.0, c.1, c.2)
    }

    /// `.dlc-size`: a persisted row keeps its stored string; live rows show
    /// `completed / total` (`0 KB / 0 KB` while aria2 doesn't know the length).
    fn size_label(&self) -> String {
        if !self.size_text.is_empty() {
            return self.size_text.clone();
        }
        match self.status {
            Status::Active | Status::Waiting | Status::Paused => {
                if self.total == 0 {
                    "0 KB / 0 KB".to_string()
                } else {
                    format!(
                        "{} / {}",
                        format_bytes(self.completed),
                        format_bytes(self.total)
                    )
                }
            }
            _ => format_bytes(self.total),
        }
    }

    /// `.dlc-file-state` badge `(moved, glyph, label)`; `None` hides it.
    fn file_state_badge(&self) -> Option<(bool, &'static str, &'static str)> {
        match self.file_state.as_str() {
            "moved" => Some((true, "drive_file_move", "Moved")),
            "removed" => Some((false, "delete", "Removed")),
            _ => None,
        }
    }

    /// `.dlc-progress-fill` background per status.
    fn progress_color(&self) -> Color32 {
        match self.status {
            Status::Error => Color32::from_rgb(0xef, 0x53, 0x50),
            Status::Paused => Color32::from_rgb(0xd6, 0x8e, 0x26),
            Status::Removed => Color32::from_rgb(0x9e, 0x9e, 0x9e),
            _ if self.progress >= 100.0 => Color32::from_rgb(0x81, 0xc7, 0x84),
            _ => mk::pal().accent,
        }
    }
}

// â”€â”€ List view (`DownloadListCard`) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// `.download-list-card` height: pad 12 + body (28 + 7 + 18.3 + 7 + 5 + 7 +
/// 22.3) + pad 12 + 2px border = 120.6.
const CARD_H: f32 = 120.6;

fn list(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, _w: f32) {
    let entries = state.grouped_entries();
    for (i, entry) in entries.iter().enumerate() {
        // `.downloads-list` is a flex column with `gap: 6px` â€” the gap sits
        // *between* the cards, not above the first one.
        if i > 0 {
            ui.add_space(6.0);
        }
        let force = state.focus_gid.is_some();
        culled_row(ui, i, force, |ui| match entry {
            crate::state::DlEntry::Group {
                id,
                gids,
                date_added,
            } => draw_group(state, ctx, ui, id, gids, *date_added, false),
            crate::state::DlEntry::Download { gid } => {
                draw_plain_card(state, ctx, ui, gid, false)
            }
        });
    }
}

/// Group container under `pos` (rects are registered during the draw pass).
fn group_at(ui: &Ui, state: &UiState, pos: Pos2) -> Option<String> {
    state.groups.iter().find_map(|g| {
        ui.ctx()
            .data(|d| d.get_temp::<Rect>(egui::Id::new("dl_group_rect").with(&g.id)))
            .filter(|r| r.contains(pos))
            .map(|_| g.id.clone())
    })
}

/// Card under `pos` — only used outside group containers, which win the test.
fn card_at(ui: &Ui, state: &UiState, pos: Pos2) -> Option<String> {
    state.downloads.iter().find_map(|d| {
        ui.ctx()
            .data(|d2| d2.get_temp::<Rect>(egui::Id::new("dl_card_rect").with(&d.gid)))
            .filter(|r| r.contains(pos))
            .map(|_| d.gid.clone())
    })
}

/// Resolve a grab-mode drop: containers take membership, siblings take a
/// manual slot, empty space drops the group membership (`handleDrop`).
fn drop_card(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, gid: &str, from_group: Option<&str>) {
    let Some(pos) = ui.ctx().pointer_interact_pos() else {
        eprintln!("[drag] drop {gid}: no pointer");
        return;
    };
    eprintln!(
        "[drag] drop {gid} at {pos:?} from={from_group:?} group={:?} card={:?}",
        group_at(ui, state, pos),
        card_at(ui, state, pos)
    );
    if let Some(target) = group_at(ui, state, pos) {
        if from_group == Some(target.as_str()) {
            return;
        }
        if state
            .downloads
            .iter()
            .find(|d| d.gid == gid)
            .is_some_and(|d| d.status == Status::Complete)
        {
            state.toast(false, "Cannot add completed download to group");
            return;
        }
        if let Some(fg) = from_group {
            state.group_remove_member(fg, gid);
        }
        let limit = state
            .group_by_id(&target)
            .map(|g| g.speed_limit)
            .unwrap_or(0);
        let name = state
            .group_by_id(&target)
            .map(|g| g.name.clone())
            .unwrap_or_default();
        state.group_add_member(&target, gid);
        state.toast(true, format!("Added to: \"{name}\""));
        let gid = gid.to_owned();
        ctx.run(move |_| async move {
            let _ = moonhunt_core::aria2::apply_download_speed_limit(
                gid,
                limit.saturating_mul(1024),
            )
            .await;
            Ok::<String, String>(String::new())
        });
        return;
    }
    if let Some(target) = card_at(ui, state, pos) {
        if target != gid {
            state.manual_move(gid, &target);
        }
    }
    // Landed on the top level: leaving a container drops the membership.
    if let Some(fg) = from_group {
        let Some(name) = state.group_by_id(fg).map(|g| g.name.clone()) else {
            return;
        };
        state.group_remove_member(fg, gid);
        state.toast(true, format!("Removed from: \"{name}\""));
        let gid = gid.to_owned();
        ctx.run(move |_| async move {
            let _ = moonhunt_core::aria2::apply_download_speed_limit(gid, 0).await;
            Ok::<String, String>(String::new())
        });
    }
}

/// `focus-download`: scroll the card into view once, then keep the accent
/// ring for 1.6s (the web adds `.flash-highlight` and drops it after 1600ms).
fn card_focus(state: &mut UiState, ui: &Ui, gid: &str) -> bool {
    if state.focus_gid.as_deref() == Some(gid) {
        state.focus_gid = None;
        ui.scroll_to_cursor(Some(egui::Align::Center));
    }
    match state.flash_ms_left(gid) {
        Some(ms) => {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(ms));
            true
        }
        None => false,
    }
}

/// A lone `.download-list-card` (not nested in a group). Ungrouped cards pass
/// `groups: []` in the old template, so they get no Group button.
fn draw_plain_card(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, gid: &str, in_group: bool) {
    if let Some(vm) = state.downloads.iter().find(|d| d.gid == gid) {
        let card = CardView::of(vm);
        let focused = card_focus(state, ui, gid);
        draw_card_top(state, ctx, ui, &card, focused, in_group, None);
    }
}

/// `.group-container` — a coloured shell around its member cards. The header
/// carries the avatar, name, meta pills, edit/delete buttons and the progress
/// strip; the body is the padded card list (list) or the tight compact rows.
fn draw_group(
    state: &mut UiState,
    ctx: &Ctx,
    ui: &mut Ui,
    group_id: &str,
    gids: &[String],
    date_added: i64,
    compact: bool,
) {
    let Some(group) = state.group_by_id(group_id).cloned() else {
        return;
    };
    let p = mk::pal();
    let w = ui.available_width();
    let collapsed = group.collapsed;
    let gcol = group.egui_color();

    // `.compact-group` metric block vs the default one.
    let (pad_y, pad_x, av, av_r, av_icon, name_sz, name_max, icon_sz, pill_sz, pill_pad,
        pill_icon, pill_gap, meta_gap, btn, btn_r, btn_icon, btn_gap, prog_inset, prog_h):
        (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) = if compact {
        (6.0f32, 12.0, 28.0, 8.0, 16.0, 12.0, 160.0, 15.0, 9.0, 5.0, 10.0, 3.0, 3.0,
         26.0, 5.0, 15.0, 2.0, 8.0, 2.0)
    } else if collapsed {
        (12.0, 16.0, 36.0, 10.0, 19.0, 13.5, f32::INFINITY, 17.0, 10.5, 8.0, 12.0, 4.0, 5.0,
         28.0, 6.0, 17.0, 4.0, 16.0, 4.0)
    } else {
        (8.0, 14.0, 28.0, 8.0, 16.0, 12.0, f32::INFINITY, 17.0, 10.5, 8.0, 12.0, 4.0, 5.0,
         28.0, 6.0, 17.0, 4.0, 16.0, 4.0)
    };

    // Header box: title row + 2px + pill row, inside the padded header-top.
    let title_h = ui
        .fonts_mut(|f| f.row_height(&semibold(name_sz)))
        .max(icon_sz);
    let pill_h = pill_sz.max(pill_icon) + 4.0;
    let heading_h = title_h + 2.0 + pill_h;
    let top_h = av.max(heading_h);
    let min_header = if compact {
        if collapsed {
            48.0
        } else {
            46.0
        }
    } else if collapsed {
        116.0
    } else {
        46.0
    };
    let header_h = (pad_y * 2.0 + top_h).max(min_header);

    // `.group-downloads`: padding 10 12 12 / gap 6 (compact 4 8 8 / 3).
    let (bp_top, bp_x, bp_bot, gap) = if compact {
        (4.0, 8.0, 8.0, 3.0)
    } else {
        (10.0, 12.0, 12.0, 6.0)
    };
    let card_h = if compact { COMPACT_H } else { CARD_H };
    let n = gids.len() as f32;
    let body_h = if collapsed || n == 0.0 {
        0.0
    } else {
        bp_top + n * card_h + (n - 1.0) * gap + bp_bot
    };
    let total_h = header_h + body_h;
    let (rect, _) = ui.allocate_exact_size(vec2(w, total_h), Sense::hover());
    // Grab-mode drop zone: a card released anywhere over the container joins it.
    ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_group_rect").with(group_id), rect));
    if std::env::var("MOONHUNT_GRAB").is_ok() {
        eprintln!("[rect] group {group_id} {rect:?}");
    }
    let hovered = ui.rect_contains_pointer(rect);
    let radius = if compact { 8.0 } else { 12.0 };

    // `box-shadow: var(--shadow-sm)` → `--shadow-md` on hover.
    if p.is_light {
        let (dy, blur, a) = if hovered {
            (4.0, 14.0, 12u8)
        } else {
            (1.0, 2.0, 8u8)
        };
        ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
            rect,
            radius as u8,
            Color32::from_rgb(0x10, 0x18, 0x28),
            dy,
            blur,
            a,
        ));
    } else {
        let (dy, blur, a) = if hovered {
            (4.0, 16.0, 45u8)
        } else {
            (2.0, 8.0, 25u8)
        };
        ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
            rect,
            radius as u8,
            Color32::BLACK,
            dy,
            blur,
            a,
        ));
    }
    // `color-mix(... 5%, transparent)` / hover 7%, border 22% / 34%.
    paint_card(
        ui,
        rect,
        radius,
        group.tint(if hovered { 7.0 } else { 5.0 }),
        group.tint(if hovered { 34.0 } else { 22.0 }),
    );
    // `.drag-over`: accent ring on the container a dragged card would land in.
    if (state.drag_gid.is_some() || state.drag_group.is_some())
        && hovered
        && state.drag_from_group.as_deref() != Some(group_id)
    {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(radius as u8),
            egui::Stroke::new(2.0, gcol),
            egui::StrokeKind::Inside,
        );
    }

    let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_h));
    let header_hovered = ui.rect_contains_pointer(header);
    if header_hovered {
        ui.painter().rect_filled(
            header,
            CornerRadius {
                nw: radius as u8,
                ne: radius as u8,
                sw: 0,
                se: 0,
            },
            group.tint(4.0),
        );
    }
    // `border-bottom: 1px color-mix(... 14%)`.
    ui.painter().rect_filled(
        Rect::from_min_size(
            pos2(header.min.x, header.max.y - 1.0),
            vec2(header.width(), 1.0),
        ),
        CornerRadius::same(0),
        group.tint(14.0),
    );

    // ── Header top row ──────────────────────────────────────────────────────
    let top = Rect::from_min_size(
        pos2(rect.min.x + pad_x, rect.min.y + pad_y),
        vec2(rect.width() - 2.0 * pad_x, top_h),
    );
    let top_cy = top.center().y;
    let av_rect = Rect::from_min_size(
        pos2(top.min.x, top_cy - av * 0.5),
        vec2(av, av),
    );
    ui.painter()
        .rect_filled(av_rect, CornerRadius::same(av_r as u8), group.tint(12.0));
    ui.painter().rect_stroke(
        av_rect,
        CornerRadius::same(av_r as u8),
        Stroke::new(1.0, group.tint(26.0)),
        StrokeKind::Inside,
    );
    mi::paint_centered(ui.painter(), av_rect.center(), "folder", av_icon, gcol);

    let heading_x = top.min.x + av + if compact { 8.0 } else { 10.0 };
    let heading_w = (top.max.x - heading_x).max(40.0);
    let name_font = semibold(name_sz);
    let name_w = text_width(ui, &group.name, name_font.clone());
    let icon_x = if compact {
        // `.group-heading` is a row in compact: title row | meta row.
        heading_x + name_w.min(name_max) + 4.0
    } else {
        heading_x + name_w.min(heading_w - icon_sz - 6.0) + 6.0
    };
    let title_cy = top.min.y + title_h * 0.5;
    // `.group-name` — nowrap + ellipsis, so cap it at the heading width.
    let cap = if compact {
        name_max.min(heading_w)
    } else {
        heading_w - icon_sz - 6.0
    };
    draw_text_spaced(
        ui,
        pos2(heading_x, title_cy),
        Align2::LEFT_CENTER,
        ellipsize(ui, &group.name, name_font.clone(), cap.max(20.0)).as_str(),
        name_font,
        p.text,
        0.01 * name_sz,
        name_sz / 24.0,
    );
    // `.group-toggle-icon` — `expand_more` rotated -90° when collapsed.
    mi::paint_centered(
        ui.painter(),
        pos2(icon_x + icon_sz * 0.5, title_cy),
        if collapsed { "chevron_right" } else { "expand_more" },
        icon_sz,
        gcol,
    );

    // Meta pills: count (group tint), size / added / limit (7% tint).
    let (done, total) = state.group_totals(gids);
    let mut mx = heading_x;
    let mut my = top.min.y + title_h + 2.0 + pill_h * 0.5;
    if compact {
        // Compact keeps title + meta on one line: meta starts after the icon.
        mx = icon_x + icon_sz + 8.0;
        my = top_cy;
    }
    let pill_font = semibold(pill_sz);
    let count_text = format!("{}", gids.len());
    mx += group_pill(
        ui,
        mx,
        my,
        "inbox",
        &count_text,
        gcol,
        group.tint(12.0),
        pill_font.clone(),
        pill_pad,
        pill_icon,
        pill_gap,
    ) + meta_gap;
    let size_text = format!("{} / {}", format_bytes(done), format_bytes(total));
    mx += group_pill(
        ui,
        mx,
        my,
        "storage",
        &size_text,
        p.text_muted,
        group.tint(7.0),
        pill_font.clone(),
        pill_pad,
        pill_icon,
        pill_gap,
    ) + meta_gap;
    let date_text = group_date(date_added);
    mx += group_pill(
        ui,
        mx,
        my,
        "calendar_today",
        &date_text,
        p.text_dim,
        group.tint(7.0),
        pill_font.clone(),
        pill_pad,
        pill_icon,
        pill_gap,
    ) + meta_gap;
    if group.download_limit_enabled && group.download_limit_bytes > 0 {
        let limit_text = format!(
            "{} / {}",
            format_group_limit(group.download_limit_used_bytes),
            format_group_limit(group.download_limit_bytes)
        );
        let _ = group_pill(
            ui,
            mx,
            my,
            "speed",
            &limit_text,
            p.text,
            group.tint(7.0),
            pill_font,
            pill_pad,
            pill_icon,
            pill_gap,
        );
    }

    // Header click toggles the collapse. Register this before the edit/delete
    // buttons so those buttons stay on top of the header's hit area.
    let hdr = ui.interact(
        header,
        egui::Id::new("grp_hdr").with(group_id),
        if state.grab_enabled {
            Sense::click_and_drag()
        } else {
            Sense::click()
        },
    );
    if hdr.clicked() {
        state.group_toggle_collapse(group_id);
    }
    if state.grab_enabled && hdr.hovered() && state.drag_gid.is_none() && state.drag_group.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    // Grab mode: dragging a header moves the container (`reorderGroups`).
    if state.grab_enabled && hdr.drag_started() {
        state.drag_group = Some(group_id.to_owned());
    }
    if state.grab_enabled && hdr.drag_stopped() {
        state.drag_group = None;
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            if let Some(target) = group_at(ui, state, pos) {
                if target != group_id {
                    state.group_reorder(group_id, &target);
                }
            }
        }
    }

    // ── Header actions (right) ──────────────────────────────────────────────
    let mut edit_clicked = false;
    let mut delete_clicked = false;
    for (i, icon) in ["edit", "delete"].iter().enumerate() {
        let bx = top.max.x - (2 - i) as f32 * btn - (1 - i) as f32 * btn_gap;
        let b = Rect::from_min_size(pos2(bx, top_cy - btn * 0.5), vec2(btn, btn));
        let rr = ui.interact(
            b,
            egui::Id::new("grp_btn").with(group_id).with(icon),
            Sense::click(),
        );
        if rr.hovered() {
            ui.painter()
                .rect_filled(b, CornerRadius::same(btn_r as u8), group.tint(12.0));
            ui.painter().rect_stroke(
                b,
                CornerRadius::same(btn_r as u8),
                Stroke::new(1.0, group.tint(24.0)),
                StrokeKind::Inside,
            );
        }
        mi::paint_centered(
            ui.painter(),
            b.center(),
            icon,
            btn_icon,
            if rr.hovered() { gcol } else { p.text_muted },
        );
        if rr.clicked() {
            if *icon == "edit" {
                edit_clicked = true;
            } else {
                delete_clicked = true;
            }
        }
    }

    // ── Collapsed preview (list view only) ──────────────────────────────────
    if collapsed && !compact {
        let preview = Rect::from_min_size(
            pos2(rect.min.x + pad_x, top.max.y + 4.0),
            vec2(rect.width() - 2.0 * pad_x, (header_h - pad_y * 2.0 - top_h - 4.0).max(0.0)),
        );
        if gids.is_empty() {
            // `.group-preview-empty`: centred inbox + text.
            let cy = preview.center().y;
            let tf = semibold(10.5);
            let gmsg = state.t("No downloads in this group yet");
            let tw = text_width(ui, gmsg, tf.clone());
            let total_w = 14.0 + 6.0 + tw;
            let sx = preview.center().x - total_w * 0.5;
            mi::paint_centered(
                ui.painter(),
                pos2(sx + 7.0, cy),
                "inbox",
                14.0,
                p.text_dim,
            );
            draw_text(
                ui,
                pos2(sx + 14.0 + 6.0, cy),
                Align2::LEFT_CENTER,
                gmsg,
                tf,
                p.text_dim,
            );
        } else {
            let mut ry = preview.min.y;
            for gid in gids.iter().take(3) {
                let Some(vm) = state.downloads.iter().find(|d| &d.gid == gid) else {
                    continue;
                };
                let row = Rect::from_min_size(
                    pos2(preview.min.x, ry),
                    vec2(preview.width(), 15.0),
                );
                let name_f = semibold(10.0);
                draw_text(
                    ui,
                    pos2(row.min.x, row.min.y + 5.5),
                    Align2::LEFT_CENTER,
                    ellipsize(ui, &vm.filename, name_f.clone(), row.width() - 70.0).as_str(),
                    name_f,
                    p.text_muted,
                );
                let sz = if vm.size_text.is_empty() {
                    format!("{} / {}", format_bytes(vm.completed), format_bytes(vm.total))
                } else {
                    vm.size_text.clone()
                };
                draw_text(
                    ui,
                    pos2(row.max.x, row.min.y + 5.5),
                    Align2::RIGHT_CENTER,
                    &sz,
                    semibold(8.5),
                    p.text_dim,
                );
                // `.group-preview-track` 2px, 10% tint; fill group colour.
                let track = Rect::from_min_size(
                    pos2(row.min.x, row.min.y + 13.0),
                    vec2(row.width(), 2.0),
                );
                ui.painter()
                    .rect_filled(track, CornerRadius::same(1), group.tint(10.0));
                let pw = (track.width() * (vm.progress / 100.0).clamp(0.0, 1.0)).max(0.0);
                if pw > 0.0 {
                    ui.painter().rect_filled(
                        Rect::from_min_size(track.min, vec2(pw, 2.0)),
                        CornerRadius::same(1),
                        gcol,
                    );
                }
                ry += 15.0 + 2.0;
            }
            if gids.len() > 3 {
                draw_text(
                    ui,
                    pos2(preview.min.x, ry + 1.0),
                    Align2::LEFT_CENTER,
                    &state.t_of(&format!("+{} {}", gids.len() - 3, state.raw("more"))),
                    semibold(9.0),
                    gcol,
                );
            }
        }
    }

    // ── Progress strip (absolute, bottom of the header) ─────────────────────
    let track = Rect::from_min_size(
        pos2(rect.min.x + prog_inset, rect.min.y + header_h - prog_h),
        vec2(rect.width() - 2.0 * prog_inset, prog_h),
    );
    ui.painter()
        .rect_filled(track, CornerRadius::same(2), p.hover_veil);
    let pct = if total > 0 {
        ((done as f64 / total as f64) * 100.0).round().min(100.0) as f32
    } else {
        0.0
    };
    if pct > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(track.min, vec2(track.width() * pct / 100.0, prog_h)),
            CornerRadius::same(2),
            gcol,
        );
    }

    if edit_clicked {
        state.g_edit = Some(group_id.to_owned());
        state.g_edit_name = group.name.clone();
        state.g_edit_color = crate::groups::GROUP_COLORS
            .iter()
            .position(|c| *c == group.color)
            .unwrap_or(0);
        state.g_edit_options = false;
        state.g_edit_on_complete = match group.on_complete.as_str() {
            "close" => 1,
            "sleep" => 2,
            "shutdown" => 3,
            "restart" => 4,
            _ => 0,
        };
        // `renameGroupSpeedLimit = group.speedLimit || 0` (0 renders "Off").
        state.g_edit_speed = group.speed_limit;
        state.g_edit_dl_on = group.download_limit_enabled;
        state.g_edit_dl_used = group.download_limit_used_bytes;
        state.g_edit_members = group.member_gids.clone();
        let (val, unit) = dl_value_for_bytes(group.download_limit_bytes);
        state.g_edit_dl_value = val;
        state.g_edit_dl_unit = unit;
        state.g_speed_text = state.g_edit_speed.to_string();
        state.g_dl_value_text = val.to_string();
        state.g_dd_open = None;
    }
    if delete_clicked {
        if let Some(g) = state.group_delete(group_id) {
            let members = g.member_gids.clone();
            if !members.is_empty() {
                ctx.run(move |_| async move {
                    for gid in members {
                        let _ = moonhunt_core::aria2::apply_download_speed_limit(gid, 0).await;
                    }
                    Ok::<String, String>(String::new())
                });
            }
            state.toast(true, "Group deleted");
        }
    }

    // ── Body: the member cards, inset by the group padding ──────────────────
    if collapsed || gids.is_empty() {
        return;
    }
    let body = Rect::from_min_size(
        pos2(rect.min.x + 1.0 + bp_x, rect.min.y + header_h + bp_top),
        vec2(
            rect.width() - 2.0 - 2.0 * bp_x,
            total_h - header_h - bp_top,
        ),
    );
    let mut cui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    cui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    for (i, gid) in gids.iter().enumerate() {
        if i > 0 {
            cui.add_space(gap);
        }
        if compact {
            if let Some(vm) = state.downloads.iter().find(|d| &d.gid == gid) {
                let card = CardView::of(vm);
                let focused = card_focus(state, &cui, gid);
                draw_compact_card(state, &mut cui, ctx, &card, focused, true, Some(group_id));
            }
        } else if let Some(vm) = state.downloads.iter().find(|d| &d.gid == gid) {
            let card = CardView::of(vm);
            let focused = card_focus(state, &cui, gid);
            draw_card_top(state, ctx, &mut cui, &card, focused, true, Some(group_id));
        }
    }
}

/// One `.group-count` / `.group-size` / … pill; returns its width.
#[allow(clippy::too_many_arguments)]
fn group_pill(
    ui: &Ui,
    x: f32,
    cy: f32,
    icon: &str,
    text: &str,
    fg: Color32,
    bg: Color32,
    font: FontId,
    pad: f32,
    icon_sz: f32,
    gap: f32,
) -> f32 {
    let tw = text_width(ui, text, font.clone());
    let h = font.size.max(icon_sz) + 4.0;
    let w = pad + icon_sz + gap + tw + pad;
    let r = Rect::from_min_size(pos2(x, cy - h * 0.5), vec2(w, h));
    ui.painter()
        .rect_filled(r, CornerRadius::same(99), bg);
    mi::paint_centered(
        ui.painter(),
        pos2(r.min.x + pad + icon_sz * 0.5, cy),
        icon,
        icon_sz,
        fg,
    );
    draw_text(
        ui,
        pos2(r.min.x + pad + icon_sz + gap, cy),
        Align2::LEFT_CENTER,
        text,
        font,
        fg,
    );
    w
}

/// `formatGroupLimitDisplay` — 0 / KB / MB / GB with the reference rounding.
fn format_group_limit(bytes: u64) -> String {
    if bytes == 0 {
        return "0".to_owned();
    }
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.0} MB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    }
}

/// `groupDlUnitForBytes` + `groupDlValueForBytes` → (value, unit index).
fn dl_value_for_bytes(bytes: u64) -> (u64, usize) {
    if bytes >= 1_073_741_824 && bytes % 1_073_741_824 == 0 {
        (bytes / 1_073_741_824, 2)
    } else if bytes >= 1_048_576 && bytes % 1_048_576 == 0 {
        (bytes / 1_048_576, 1)
    } else if bytes >= 1024 && bytes % 1024 == 0 {
        (bytes / 1024, 0)
    } else if bytes == 0 {
        (1, 1)
    } else {
        ((bytes / 1_048_576).max(1), 1)
    }
}

/// `formatGroupDate` — `new Date(ms).toLocaleDateString()`.
/// shortcut: rendered as a UTC `M/D/YYYY` date; the reference uses the OS
/// timezone, which can differ by one day near midnight.
fn group_date(ms: i64) -> String {
    if ms <= 0 {
        return "-".to_owned();
    }
    let days = ms.div_euclid(86_400_000);
    // Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    format!("{m}/{d}/{y}")
}

fn draw_card_top(
    state: &mut UiState,
    ctx: &Ctx,
    ui: &mut Ui,
    card: &CardView,
    focused: bool,
    in_group: bool,
    group_id: Option<&str>,
) {
    let p = mk::pal();
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, CARD_H), Sense::hover());
    // Drop-target bookkeeping for the grab-mode drag (see `drop_card`).
    ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_card_rect").with(&card.gid), rect));
    if std::env::var("MOONHUNT_GRAB").is_ok() {
        eprintln!("[rect] card {} {rect:?}", card.gid);
    }
    let sense = if state.grab_enabled {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let card_r = ui.interact(rect, egui::Id::new("dlc").with(&card.gid), sense);
    if state.grab_enabled && card_r.hovered() && state.drag_gid.is_none() && state.drag_group.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    if state.grab_enabled && card_r.drag_started() {
        eprintln!("[drag] card start {}", card.gid);
        state.drag_gid = Some(card.gid.clone());
        state.drag_from_group = group_id.map(str::to_owned);
    }
    if state.grab_enabled && card_r.drag_stopped() {
        eprintln!("[drag] card stop {}", card.gid);
        state.drag_gid = None;
        drop_card(state, ctx, ui, &card.gid, group_id);
    }
    if card_r.secondary_clicked() {
        // Old `@contextmenu` â†’ `GlobalContextMenu` at the pointer.
        state.cm_open = true;
        state.cm_group_menu = false;
        state.cm_gid = Some(card.gid.clone());
        state.cm_pos = ui
            .ctx()
            .pointer_interact_pos()
            .unwrap_or_else(|| rect.center());
    }
    let press = card_press(ui, &card.gid, &card_r);
    let is_being_dragged = state.drag_gid.as_deref() == Some(card.gid.as_str());
    let is_drop_hover = state.drag_gid.as_deref().is_some_and(|g| g != card.gid)
        && ui.ctx().pointer_latest_pos().is_some_and(|q| rect.contains(q));
    let mut border = if is_being_dragged {
        p.accent.gamma_multiply(0.75)
    } else if is_drop_hover {
        p.accent.gamma_multiply(0.55)
    } else if card_r.hovered() || focused {
        mk::blend(p.bg_card, p.accent, if focused { 0.35 } else { 0.18 })
    } else {
        p.border
    };
    if press > 0.0 {
        border = mk::blend(border, p.accent, 0.45 * press);
    }
    // `.download-list-card:hover { transform: translateY(-1px) }`: paint (and
    // hit-test the inner wells) one pixel higher while hovered; a press nudges
    // it back down for click feedback.
    let rect = if card_r.hovered() {
        rect.translate(vec2(0.0, -1.0))
    } else {
        rect
    };
    let rect = rect.translate(vec2(0.0, 1.6 * press));
    paint_card(ui, rect, 12.0, p.bg_card, border);
    // `.download-list-card:hover { box-shadow: 0 4px 16px rgba(0,0,0,.1) }`.
    if card_r.hovered() {
        ui.painter().extend(crate::ui::shade::soft_shadow_shapes(
            rect,
            12,
            Color32::BLACK,
            4.0,
            16.0,
            26,
        ));
        paint_card(ui, rect, 12.0, p.bg_card, border);
    }
    card_ripple(ui, &card.gid, rect, &card_r, p.accent);
    if is_being_dragged {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(12),
            p.bg_surface.gamma_multiply(0.45),
        );
    } else if is_drop_hover {
        draw_card_drop_indicator(ui, rect, p.accent_strong);
    }
    if card_r.double_clicked() {
        // Old `@dblclick="handleDoubleClick"` â†’ open the file.
        run_card_action(state, ctx, card, "open");
    }

    let actions = card_actions(card);
    let n = actions.len() as f32;

    // â”€â”€ Top row: filename âœ¥ status actions (17px icons in 28Ã—28 r6 wells).
    // `.download-list-card` = padding 12px 16px + 1px border â†’ the content box
    // starts 13px below the card's top edge and 17px from its left edge.
    let name_max = (w - 32.0 - (n * 28.0 + (n - 1.0)) - 8.0).max(60.0);
    // `.dlc-top { align-items: center }`: the 28px-tall action wells set the row
    // height, so the filename's line box is centred in +13..+41 (ink lands at
    // card+21..+31, matching the reference). The old app also renders the name
    // with the Bold face (600 selects the loaded 700 file there), not SemiBold.
    // Chromium paints the 600 request on the bundled 400 face with a fake-bold
    // smear (13.5/24 = 0.56px); a real face tracks differently â€” see h1.
    draw_text_spaced(
        ui,
        pos2(rect.min.x + 17.0, rect.min.y + 27.0),
        Align2::LEFT_CENTER,
        ellipsize(ui, &card.filename, regular(13.5), name_max).as_str(),
        regular(13.5),
        p.text,
        0.0,
        13.5 / 24.0,
    );
    let mut ax = rect.max.x - 16.0 - n * 28.0 - (n - 1.0) * 1.0;
    for (icon, danger) in &actions {
        let b = Rect::from_min_size(pos2(ax, rect.min.y + 13.0), vec2(28.0, 28.0));
        let rr = ui.interact(b, egui::Id::new("dlcb").with(&card.gid).with(icon), Sense::click());
        if rr.hovered() {
            ui.painter().rect_filled(
                b,
                CornerRadius::same(6),
                p.hover_veil,
            );
        }
        let color = match danger {
            ActionHover::None if rr.hovered() => p.text,
            ActionHover::None => p.text_muted,
            ActionHover::Paused if rr.hovered() => p.status_paused,
            ActionHover::Paused => p.text_muted,
            ActionHover::Error if rr.hovered() => p.status_error,
            ActionHover::Error => p.text_muted,
        };
        mi::paint_centered(ui.painter(), b.center(), icon, 17.0, color);
        if rr.clicked() {
            run_card_action(state, ctx, card, action_of(icon));
        }
        rr.on_hover_text(state.t(action_tooltip(icon)));
        ax += 29.0;
    }

    // â”€â”€ Info row: status pill + source badge + size (+ speed/eta when live).
    let cy = rect.min.y + 47.0 + 10.0;
    let (glyph, color, label) = card.pill();
    let label = state.t(label);
    let up = label.to_uppercase();
    // .dlc-pill: 10.5px/700, letter-spacing .03em; the bundle has Nunito 400
    // only, so Chromium paints 700 as 400 advances + a size/24px smear
    // (Skia fake-bold) â€” a real Bold face here is visibly too wide.
    let pf = regular(10.5);
    let pill_spacing = 0.03 * 10.5;
    let tw = text_width(ui, &up, pf.clone()) + pill_spacing * (up.chars().count() as f32 - 1.0);
    let pill_w = 16.0 + 13.0 + 3.0 + tw;
    let pill = Rect::from_min_size(pos2(rect.min.x + 16.0, cy - 9.5), vec2(pill_w, 19.0));
    ui.painter().rect_filled(
        pill,
        CornerRadius::same(99),
        mk::blend(p.bg_card, card.pill_bg(), 0.12),
    );
    mi::paint_centered(ui.painter(), pos2(pill.min.x + 8.0 + 6.5, cy), glyph, 13.0, color);
    draw_text_spaced(
        ui,
        pos2(pill.min.x + 8.0 + 13.0 + 3.0, cy),
        Align2::LEFT_CENTER,
        &up,
        pf,
        color,
        pill_spacing,
        10.5 / 24.0,
    );
    let mut x = pill.max.x + 6.0;

    // Source badge: yt-dlp (red) or HTTP (accent tint).
    if card.is_yt {
        // .dlc-badge.yt: 10px/600 (400 face + Skia fake-bold smear), y 2px 7px.
        let bf = regular(10.0);
        let bw = 15.0 + 12.0 + 3.0 + text_width(ui, "yt-dlp", bf.clone());
        let badge = Rect::from_min_size(pos2(x, cy - 8.0), vec2(bw, 18.0));
        ui.painter()
            .rect_filled(badge, CornerRadius::same(4), mk::blend(p.bg_card, p.status_error, 0.08));
        mi::paint_centered(ui.painter(), pos2(x + 7.0 + 6.0, cy), "smart_display", 12.0, p.status_error);
        draw_text_spaced(
            ui,
            pos2(x + 7.0 + 12.0 + 3.0, cy),
            Align2::LEFT_CENTER,
            "yt-dlp",
            bf,
            p.status_error,
            0.0,
            10.0 / 24.0,
        );
        x += bw + 6.0;
    } else {
        let bf = regular(10.0);
        let bw = 15.0 + text_width(ui, "HTTP", bf.clone());
        let badge = Rect::from_min_size(pos2(x, cy - 8.0), vec2(bw, 18.0));
        ui.painter()
            .rect_filled(badge, CornerRadius::same(4), mk::blend(p.bg_card, p.accent, 0.08));
        draw_text_spaced(
            ui,
            pos2(x + 8.0, cy),
            Align2::LEFT_CENTER,
            "HTTP",
            bf,
            p.accent_strong,
            0.0,
            10.0 / 24.0,
        );
        x += bw + 6.0;
    }

    // `.dlc-file-state`: 10/700, pad 2 7, r4, gap 3, 12px icon; `Moved` is
    // amber-tinted, `Removed` red-tinted. It sits after the source badge.
    if let Some((moved, icon, label)) = card.file_state_badge() {
        let label = state.t(label);
        let (fg, bg) = if moved {
            (
                p.status_paused,
                Color32::from_rgba_unmultiplied(255, 167, 38, 31),
            )
        } else {
            (
                p.status_error,
                Color32::from_rgba_unmultiplied(239, 83, 80, 26),
            )
        };
        let up = label.to_uppercase();
        let ff = regular(10.0);
        let sp = 0.03 * 10.0;
        let tw = text_width(ui, &up, ff.clone()) + sp * (up.chars().count() as f32 - 1.0);
        let fw = 7.0 + 12.0 + 3.0 + tw + 7.0;
        let fs_rect = Rect::from_min_size(pos2(x, cy - 9.0), vec2(fw, 18.0));
        ui.painter()
            .rect_filled(fs_rect, CornerRadius::same(4), bg);
        mi::paint_centered(
            ui.painter(),
            pos2(fs_rect.min.x + 7.0 + 6.0, cy),
            icon,
            12.0,
            fg,
        );
        draw_text_spaced(
            ui,
            pos2(fs_rect.min.x + 7.0 + 12.0 + 3.0, cy),
            Align2::LEFT_CENTER,
            &up,
            ff,
            fg,
            sp,
            10.0 / 24.0,
        );
        // `.dlc-file-state` title — hover only, never an `interact` (the card's
        // own click covers this rect).
        if ui.rect_contains_pointer(fs_rect) {
            let tip = state.t(if moved {
                "File was moved from its recorded location"
            } else {
                "File no longer exists"
            });
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                egui::Id::new(("fs-tip", &card.gid)),
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.label(tip);
            });
        }
        x += fw + 6.0;
    }
    // runs and a single total once the download is finished.
    let size_s = card.size_label();
    let size_w = text_width(ui, &size_s, regular(11.0));
    draw_text(ui, pos2(x, cy), Align2::LEFT_CENTER, &size_s, regular(11.0), p.text_dim);
    x += size_w;

    // `.dlc-info` is a wrap-capable flow row: speed and ETA follow the size.
    if card.status == Status::Active && !card.speed.is_empty() {
        x += 6.0;
        mi::paint_centered(
            ui.painter(),
            pos2(x + 7.0, cy),
            "arrow_downward",
            14.0,
            p.accent_strong,
        );
        x += 14.0 + 3.0;
        let sf = regular(11.0);
        let r = draw_text_spaced(
            ui,
            pos2(x, cy),
            Align2::LEFT_CENTER,
            &card.speed,
            sf,
            p.accent_strong,
            0.0,
            11.0 / 24.0,
        );
        x = r.max.x;
    }
    if card.status == Status::Active && !card.eta.is_empty() {
        x += 6.0;
        mi::paint_centered(ui.painter(), pos2(x + 7.0, cy), "schedule", 14.0, p.text_dim);
        x += 14.0 + 3.0;
        let r = draw_text_spaced(
            ui,
            pos2(x, cy),
            Align2::LEFT_CENTER,
            &card.eta,
            regular(11.0),
            p.text_dim,
            0.0,
            11.0 / 24.0,
        );
        let _ = r;
    }

    // â”€â”€ Progress track: 5px, hover-bg, r4; fill per status.
    let track = Rect::from_min_size(
        pos2(rect.min.x + 16.0, rect.min.y + 74.0),
        vec2(rect.width() - 32.0, 5.0),
    );
    ui.painter()
        .rect_filled(track, CornerRadius::same(4), p.hover_veil);
    if card.progress > 0.0 {
        let fw = track.width() * (card.progress / 100.0).clamp(0.0, 1.0);
        let fill = Rect::from_min_size(track.min, vec2(fw, 5.0));
        ui.painter()
            .rect_filled(fill, CornerRadius::same(4), card.progress_color());
        // yt-dlp fills carry a 6px glow (`.dlc-progress-fill` boxShadow).
        if card.is_yt {
            let c = card.progress_color();
            ui.painter().rect_filled(
                fill.expand(2.0),
                CornerRadius::same(4),
                Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 40),
            );
            ui.painter()
                .rect_filled(fill, CornerRadius::same(4), card.progress_color());
        }
    }

    // ── Bottom row: layers ✥ connections (+ the group chip when grouped).
    // aria2 resets `connections` to 0 once a transfer stops; the old app keeps
    // the last known count (1 for a finished file), so fall back to 1.
    let conn = card.connections.filter(|c| *c > 0).unwrap_or(1);
    let cf = regular(10.5);
    // Order the finished sentence, not the fragment: gluing a number to an
    // already-visual word puts the count on the left, where a Persian sentence
    // ends, so the label read back to front.
    let conn_text = state.t_of(&format!("{conn} {}", state.raw("connections")));
    let cw = text_width(ui, &conn_text, cf.clone());
    // `.dlc-bottom` spans 85.3..107.6 inside the border box: centre 96.45.
    let bottom_cy = rect.min.y + 96.45;
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 16.0 + 6.0, bottom_cy),
        "layers",
        12.0,
        p.text_muted,
    );
    draw_text(
        ui,
        pos2(rect.min.x + 16.0 + 12.0 + 3.0, bottom_cy),
        Align2::LEFT_CENTER,
        &conn_text,
        cf,
        p.text_muted,
    );
    if card.status == Status::Error && !card.error.is_empty() {
        // `.dlc-btn-sm.error-sm`: hover-bg chip, 10.5/600 muted, pad 4 8.
        let ef = semibold(10.5);
        let err_label = state.t("Error");
        let bw = 16.0 + 13.0 + 4.0 + text_width(ui, err_label, ef.clone());
        let b = Rect::from_min_size(
            pos2(rect.min.x + 16.0 + 12.0 + 3.0 + cw + 8.0, bottom_cy - 11.15),
            vec2(bw, 22.3),
        );
        let rr = ui.interact(b, egui::Id::new("dlc_err").with(&card.gid), Sense::click());
        ui.painter().rect_filled(
            b,
            CornerRadius::same(6),
            if rr.hovered() {
                Color32::from_rgba_unmultiplied(239, 83, 80, 26)
            } else {
                p.hover_veil
            },
        );
        mi::paint_centered(ui.painter(), pos2(b.min.x + 8.0 + 6.5, b.center().y), "receipt_long", 13.0, p.text_muted);
        draw_text(
            ui,
            pos2(b.min.x + 16.0 + 13.0 + 4.0, b.center().y),
            Align2::LEFT_CENTER,
            err_label,
            ef,
            p.text_muted,
        );
        if rr.clicked() {
            state.err_open = true;
            state.err_filename = card.filename.clone();
            state.err_message = card.error.clone();
        }
    }

    // `.dlc-group-wrap`: a `Group` chip at the bottom-right (only cards that
    // are already inside a group get one — the old template passes
    // `:groups="[]"` to ungrouped cards).
    if in_group && !state.groups.is_empty() && card.status != Status::Complete {
        let gf = semibold(10.5);
        let grp_label = state.t("Group");
        let bw = 8.0 + 13.0 + 4.0 + text_width(ui, grp_label, gf.clone()) + 8.0;
        let b = Rect::from_min_size(
            pos2(rect.max.x - 16.0 - bw, bottom_cy - 11.15),
            vec2(bw, 22.3),
        );
        let rr = ui.interact(b, egui::Id::new("dlc_grp").with(&card.gid), Sense::click());
        ui.painter().rect_filled(
            b,
            CornerRadius::same(6),
            if rr.hovered() {
                Color32::from_rgba_unmultiplied(
                    p.accent_strong.r(),
                    p.accent_strong.g(),
                    p.accent_strong.b(),
                    20,
                )
            } else {
                p.hover_veil
            },
        );
        mi::paint_centered(
            ui.painter(),
            pos2(b.min.x + 8.0 + 6.5, b.center().y),
            "group_add",
            13.0,
            if rr.hovered() { p.accent_strong } else { p.text_muted },
        );
        draw_text(
            ui,
            pos2(b.min.x + 8.0 + 13.0 + 4.0, b.center().y),
            Align2::LEFT_CENTER,
            grp_label,
            gf,
            if rr.hovered() { p.accent_strong } else { p.text_muted },
        );
        if rr.clicked() {
            ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_group_anchor"), b));
            state.g_menu_gid = if state.g_menu_gid.as_deref() == Some(card.gid.as_str()) {
                None
            } else {
                Some(card.gid.clone())
            };
        }
    }
}

enum ActionHover {
    None,
    Paused,
    Error,
}

type CardAction = (&'static str, ActionHover);

/// Top-row action buttons (right-aligned) for a card status.
fn card_actions(card: &CardView) -> Vec<CardAction> {
    let mut v: Vec<CardAction> = Vec::new();
    match card.status {
        Status::Active | Status::Waiting => v.push(("pause", ActionHover::None)),
        Status::Paused => v.push(("play_arrow", ActionHover::None)),
        Status::Error => v.push(("refresh", ActionHover::None)),
        Status::Complete => v.push(("download", ActionHover::None)),
        Status::Removed => v.push(("refresh", ActionHover::None)),
    }
    v.push(("folder_open", ActionHover::None));
    v.push(("description", ActionHover::None));
    match card.status {
        Status::Active | Status::Waiting | Status::Paused => {
            v.push(("cancel", ActionHover::Paused))
        }
        _ => v.push(("close", ActionHover::Error)),
    }
    v
}

fn action_of(icon: &str) -> &'static str {
    match icon {
        "pause" => "pause",
        "play_arrow" => "resume",
        "refresh" => "retry",
        "download" => "redownload",
        "folder_open" => "folder",
        // Old `DownloadListCard`: the `description` button is *Details*
        // (`showDownloadDetails`) â€” double-click stays "open the file".
        "description" => "details",
        "cancel" => "cancel",
        _ => "remove",
    }
}

fn action_tooltip(icon: &str) -> &'static str {
    match icon {
        "pause" => "Pause",
        "play_arrow" => "Resume",
        "refresh" => "Retry",
        "download" => "Redownload",
        "folder_open" => "Show in Folder",
        "description" => "Details",
        "cancel" => "Cancel",
        "group_add" => "Add to Group",
        _ => "Remove",
    }
}

fn run_card_action(state: &mut UiState, ctx: &Ctx, card: &CardView, action: &str) {
    let gid = card.gid.clone();
    let path = card.path.clone();
    let filename = card.filename.clone();
    match action {
        // Vue's `pauseDownload`/`resumeDownload`/`retryDownload` are silent on
        // success and prefix their failures with the action name.
        "pause" => ctx.run(move |_| async move {
            moonhunt_core::aria2::pause_download(gid)
                .await
                .map_err(|e| format!("Failed to pause: {e}"))
                .map(|_| String::new())
        }),
        "resume" => ctx.run(move |_| async move {
            moonhunt_core::aria2::resume_download(gid)
                .await
                .map_err(|e| format!("Failed to resume: {e}"))
                .map(|_| String::new())
        }),
        "retry" | "redownload" => ctx.run(move |app| async move {
            moonhunt_core::aria2::retry_download(app, gid)
                .await
                .map_err(|e| format!("Failed to retry: {e}"))
        }),
        "folder" => ctx.run(move |app| async move {
            let p = if path.is_empty() { None } else { Some(path.clone()) };
            moonhunt_core::fs_ops::show_download_in_folder(app, gid, p, Some(filename))
                .await
                .map_err(|e| format!("Failed to open folder: {e}"))
        }),
        "open" => ctx.run(move |app| async move {
            let p = if path.is_empty() { None } else { Some(path.clone()) };
            moonhunt_core::fs_ops::open_download_file(app, gid, p, Some(filename))
                .await
                .map_err(|e| format!("Failed to open file: {e}"))
        }),
        // Old `showDownloadDetails(download)` â†’ open the details modal. The
        // fetches fire from the modal's open-edge tick in `modals.rs`.
        "details" => {
            state.dm_open = true;
            state.dm_gid = Some(card.gid.clone());
        }
        // Card-menu results (`MainLayout`): "Download cancelled" /
        // "Download removed successfully"; failures surface as `Err`.
        "cancel" => ctx.run(move |app| async move {
            moonhunt_core::aria2::remove_download(app, gid)
                .await
                .map_err(|e| format!("Failed to cancel: {e}"))
                .map(|_| "Download cancelled".to_string())
        }),
        "remove" => ctx.run(move |app| async move {
            moonhunt_core::aria2::remove_download(app, gid)
                .await
                .map(|_| "Download removed successfully".to_string())
        }),
        _ => {}
    }
}

/// Context-menu dispatch: re-resolve the live row for `gid` and run the same
/// action the card buttons use (`GlobalContextMenu` â†’ `handleDownloadAction`).
pub(crate) fn run_action_for_gid(
    state: &mut UiState,
    ctx: &Ctx,
    gid: &str,
    action: &str,
) {
    let Some(vm) = state.downloads.iter().find(|d| d.gid == gid).cloned() else {
        return;
    };
    let card = CardView::of(&vm);
    run_card_action(state, ctx, &card, action);
}

/// Truncate with an ellipsis so the ellipsized `.dlc-filename` stays on one line.
/// Truncated to a max width; returns the string (callers add `&`).
fn ellipsize(ui: &Ui, text: &str, font: FontId, max_w: f32) -> String {
    crate::rtl::truncate(text, |s| text_width(ui, s, font.clone()) <= max_w)
}

/// Stable collision-free helper: same as `ellipsize` (kept for clarity).

// â”€â”€ Compact view (`DownloadListCardCompact`) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Compact card: pad 6 12 + row 34.01 + pad 6 + 2px border = 48 (the 2px
/// progress strip is `position: absolute` and adds no height).
const COMPACT_H: f32 = 48.0;

/// Draws one list row, skipping the body when the row sits outside the scroll
/// viewport. Tall rows cost ~0.2 ms each (gradients, soft shadows, galleys), so
/// a 40-row list dropped from ~10 ms to ~4 ms per frame with this in place.
/// Heights come from the previous frame; while off-screen they only feed
/// `add_space`, so the scroll extent is unchanged.
fn culled_row(ui: &mut Ui, index: usize, force: bool, mut draw: impl FnMut(&mut Ui)) {
    const PAD: f32 = 96.0;
    let id = egui::Id::new("dl_row_heights");
    let clip = ui.clip_rect();
    let top = ui.cursor().min.y;
    // `get_temp_mut_or_default` hands out the cache in place; a clone per row
    // made this O(n²) and dominated the frame once the list got long.
    let cached = ui.data_mut(|d| d.get_temp_mut_or_default::<Vec<f32>>(id).get(index).copied());
    if !force {
        if let Some(h) = cached {
            if top + h < clip.min.y - PAD || top > clip.max.y + PAD {
                ui.add_space(h);
                return;
            }
        }
    }
    let before = ui.cursor().min.y;
    draw(ui);
    let h = ui.cursor().min.y - before;
    ui.data_mut(|d| {
        let heights = d.get_temp_mut_or_default::<Vec<f32>>(id);
        if heights.len() <= index {
            heights.resize(index + 1, 0.0);
        }
        heights[index] = h;
    });
}

fn compact_list(state: &mut UiState, ctx: &Ctx, ui: &mut Ui, _w: f32) {
    let entries = state.grouped_entries();
    for (i, entry) in entries.iter().enumerate() {
        // `.downloads-compact { gap: 6px }` — between rows only.
        if i > 0 {
            ui.add_space(6.0);
        }
        let force = state.focus_gid.is_some();
        culled_row(ui, i, force, |ui| match entry {
            crate::state::DlEntry::Group {
                id,
                gids,
                date_added,
            } => draw_group(state, ctx, ui, id, gids, *date_added, true),
            crate::state::DlEntry::Download { gid } => {
                if let Some(vm) = state.downloads.iter().find(|d| &d.gid == gid) {
                    let card = CardView::of(vm);
                    let focused = card_focus(state, ui, gid);
                    draw_compact_card(state, ui, ctx, &card, focused, false, None);
                }
            }
        });
    }
}

fn draw_compact_card(
    state: &mut UiState,
    ui: &mut Ui,
    ctx: &Ctx,
    card: &CardView,
    focused: bool,
    in_group: bool,
    group_id: Option<&str>,
) {
    let p = mk::pal();
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, COMPACT_H), Sense::hover());
    ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_card_rect").with(&card.gid), rect));
    let sense = if state.grab_enabled {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let row_r = ui.interact(rect, egui::Id::new("dlcc").with(&card.gid), sense);
    if state.grab_enabled && row_r.hovered() && state.drag_gid.is_none() && state.drag_group.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    if state.grab_enabled && row_r.drag_started() {
        state.drag_gid = Some(card.gid.clone());
        state.drag_from_group = group_id.map(str::to_owned);
    }
    if state.grab_enabled && row_r.drag_stopped() {
        state.drag_gid = None;
        drop_card(state, ctx, ui, &card.gid, group_id);
    }
    let hovered = row_r.hovered();
    if row_r.secondary_clicked() {
        state.cm_open = true;
        state.cm_group_menu = false;
        state.cm_gid = Some(card.gid.clone());
        state.cm_pos = ui
            .ctx()
            .pointer_interact_pos()
            .unwrap_or_else(|| rect.center());
    }
    let press = card_press(ui, &card.gid, &row_r);
    let is_being_dragged = state.drag_gid.as_deref() == Some(card.gid.as_str());
    let is_drop_hover = state.drag_gid.as_deref().is_some_and(|g| g != card.gid)
        && ui.ctx().pointer_latest_pos().is_some_and(|q| rect.contains(q));
    let mut border = if is_being_dragged {
        p.accent.gamma_multiply(0.75)
    } else if is_drop_hover {
        p.accent.gamma_multiply(0.55)
    } else if hovered || focused {
        mk::blend(p.bg_card, p.accent, if focused { 0.35 } else { 0.18 })
    } else {
        p.border
    };
    if press > 0.0 {
        border = mk::blend(border, p.accent, 0.45 * press);
    }
    // `.download-list-card-compact:hover { transform: translateY(-1px) }`:
    // paint one pixel higher while hovered (the row keeps its layout slot); a
    // press nudges it back down for click feedback.
    let rect = if hovered {
        rect.translate(vec2(0.0, -1.0))
    } else {
        rect
    };
    let rect = rect.translate(vec2(0.0, 1.6 * press));
    paint_card(ui, rect, 8.0, p.bg_card, border);
    card_ripple(ui, &card.gid, rect, &row_r, p.accent);
    if is_being_dragged {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(8),
            p.bg_surface.gamma_multiply(0.45),
        );
    } else if is_drop_hover {
        draw_card_drop_indicator(ui, rect, p.accent_strong);
    }

    // ── `.dlc-row`: `dlc-main` | size (min 80) | spacer 4 | speed (min 90) |
    // spacer 4 | actions, gap 8. Title centre 15.5, meta centre 33.5, the rest
    // centred on the 34px row (y 24).
    let row_cy = rect.min.y + 24.0;
    let title_cy = rect.min.y + 15.5;
    let meta_cy = rect.min.y + 33.5;
    let x0 = rect.min.x + 12.0;
    let mut actions = card_compact_actions(card);
    if in_group && !state.groups.is_empty() && card.status != Status::Complete {
        // `group-card-btn-wrapper` sits between `folder_open` and the
        // cancel/remove well in the old DOM order.
        let at = actions.len() - 1;
        actions.insert(at, ("group_add", ActionHover::None));
    }
    let n = actions.len() as f32;
    let actions_w = n * 26.0 + (n - 1.0) * 2.0;
    let actions_x = rect.max.x - 12.0 - actions_w;
    let speed_right = actions_x - 20.0;
    let speed_x = speed_right - 90.0;
    let size_right = speed_x - 20.0;

    // `.dlc-title`: 12.5px weight 500 (Nunito 400 face + Skia fake-bold).
    draw_text_spaced(
        ui,
        pos2(x0, title_cy),
        Align2::LEFT_CENTER,
        ellipsize(
            ui,
            &card.filename,
            regular(12.5),
            (size_right - 80.0 - 8.0 - x0).max(80.0),
        )
        .as_str(),
        regular(12.5),
        p.text,
        0.0,
        12.5 / 48.0,
    );

    // `.dlc-meta`: status pill (no background — only the text is tinted),
    // then the optional `Media` tag.
    let (glyph, color, label) = card.pill();
    let label = state.t(label);
    const PILL_H: f32 = 1.0 + 12.96; // pad 1 + 9.5px line box
    let pfont = regular(9.5);
    let smear = 9.5 / 24.0;
    let pw = 6.0 + 11.0 + 3.0 + text_width(ui, label, pfont.clone()) + 6.0;
    let pill = Rect::from_min_size(pos2(x0, meta_cy - PILL_H * 0.5), vec2(pw, PILL_H));
    mi::paint_centered(
        ui.painter(),
        pos2(pill.min.x + 6.0 + 5.5, meta_cy),
        glyph,
        11.0,
        color,
    );
    draw_text_spaced(
        ui,
        pos2(pill.min.x + 6.0 + 11.0 + 3.0, meta_cy),
        Align2::LEFT_CENTER,
        label,
        pfont,
        color,
        0.0,
        smear,
    );
    let mut mx = pill.max.x + 6.0;

    // `.dlc-source-tag.dlc-source-media`: 8.5/600, pad 1 5, r3, 1px border.
    if card.is_yt {
        let tf = regular(8.5);
        const TAG_H: f32 = 1.0 + 11.6 + 1.0 + 2.0;
        let tw2 = 5.0 + 10.0 + 2.0 + text_width(ui, state.t("Media"), tf.clone()) + 5.0 + 2.0;
        let tag = Rect::from_min_size(pos2(mx, meta_cy - TAG_H * 0.5), vec2(tw2, TAG_H));
        ui.painter().rect_filled(
            tag,
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 31),
        );
        ui.painter().rect_stroke(
            tag,
            CornerRadius::same(3),
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 51),
            ),
            StrokeKind::Inside,
        );
        mi::paint_centered(ui.painter(), pos2(mx + 1.0 + 5.0 + 5.0, meta_cy), "smart_display", 10.0, p.accent_strong);
        draw_text_spaced(
            ui,
            pos2(mx + 1.0 + 5.0 + 10.0 + 2.0, meta_cy),
            Align2::LEFT_CENTER,
            state.t("Media"),
            tf,
            p.accent_strong,
            0.03 * 8.5,
            8.5 / 24.0,
        );
        mx += tw2 + 6.0;
    }

    // `.dlc-file-state`: 8.5/700, pad 1 5, r3, gap 2, 10px icon, 1px tinted
    // border — same tints as the list card.
    if let Some((moved, icon, label)) = card.file_state_badge() {
        let label = state.t(label);
        let (fg, tint) = if moved {
            (p.status_paused, (255u8, 167u8, 38u8))
        } else {
            (p.status_error, (239u8, 83u8, 80u8))
        };
        let up = label.to_uppercase();
        let ff = regular(8.5);
        let sp = 0.03 * 8.5;
        let tw2 = text_width(ui, &up, ff.clone()) + sp * (up.chars().count() as f32 - 1.0);
        let fw = 1.0 + 5.0 + 10.0 + 2.0 + tw2 + 5.0 + 1.0;
        let fh = 1.0 + 11.6 + 1.0 + 2.0;
        let fs_rect = Rect::from_min_size(pos2(mx, meta_cy - fh * 0.5), vec2(fw, fh));
        ui.painter().rect_filled(
            fs_rect,
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(tint.0, tint.1, tint.2, if moved { 31 } else { 26 }),
        );
        ui.painter().rect_stroke(
            fs_rect,
            CornerRadius::same(3),
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(tint.0, tint.1, tint.2, 51),
            ),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(mx + 1.0 + 5.0 + 5.0, meta_cy),
            icon,
            10.0,
            fg,
        );
        draw_text_spaced(
            ui,
            pos2(mx + 1.0 + 5.0 + 10.0 + 2.0, meta_cy),
            Align2::LEFT_CENTER,
            &up,
            ff,
            fg,
            sp,
            8.5 / 24.0,
        );
        // Compact card: same `.dlc-file-state` title, hover-only.
        if ui.rect_contains_pointer(fs_rect) {
            let tip = state.t(if moved {
                "File was moved from its recorded location"
            } else {
                "File no longer exists"
            });
            egui::Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                egui::Id::new(("fs-tip", &card.gid)),
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.label(tip);
            });
        }
    }
    // `.dlc-size` — 10.5px, min-width 80, right-aligned.
    let size_s = card.size_label();
    draw_text(
        ui,
        pos2(size_right, row_cy),
        Align2::RIGHT_CENTER,
        &size_s,
        regular(10.5),
        p.text_dim,
    );

    // `.dlc-speed`: arrow 12px + gap 3 + stack(speed, connections pill, eta).
    if card.status == Status::Active && !card.speed.is_empty() {
        mi::paint_centered(
            ui.painter(),
            pos2(speed_x + 6.0, row_cy),
            "arrow_downward",
            12.0,
            p.accent_strong,
        );
        let mut sx = speed_x + 12.0 + 3.0;
        let sf = regular(11.0);
        let r = draw_text_spaced(
            ui,
            pos2(sx, row_cy),
            Align2::LEFT_CENTER,
            &card.speed,
            sf.clone(),
            p.accent_strong,
            0.0,
            11.0 / 48.0,
        );
        sx = r.max.x + 4.0;

        let conn = card.connections.filter(|c| *c > 0).unwrap_or(1);
        let nf = regular(9.5);
        let num = conn.to_string();
        let nw = text_width(ui, &num, nf.clone()) + 9.5 / 24.0 * (num.chars().count() as f32 - 1.0).max(0.0);
        let cw = 5.0 + nw + 2.0 + 10.0 + 5.0;
        let pill_r = Rect::from_min_size(pos2(sx, row_cy - 5.75), vec2(cw, 11.5));
        ui.painter().rect_filled(
            pill_r,
            CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 15),
        );
        draw_text_spaced(
            ui,
            pos2(pill_r.min.x + 5.0, row_cy),
            Align2::LEFT_CENTER,
            &num,
            nf,
            p.accent_strong,
            0.0,
            9.5 / 24.0,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(pill_r.min.x + 5.0 + nw + 2.0 + 5.0, row_cy),
            "layers",
            10.0,
            p.accent_strong,
        );
        sx = pill_r.max.x + 4.0;

        if !card.eta.is_empty() {
            draw_text_spaced(
                ui,
                pos2(sx, row_cy),
                Align2::LEFT_CENTER,
                &card.eta,
                regular(9.5),
                p.text_dim,
                0.0,
                9.5 / 24.0,
            );
        }
    } else {
        // `.dlc-speed.dlc-muted` — an em dash when the row is not live.
        draw_text(
            ui,
            pos2(speed_x, row_cy),
            Align2::LEFT_CENTER,
            "\u{2014}",
            regular(11.0),
            p.text_dim,
        );
    }

    // `.dlc-actions`: 26×26 wells, gap 2, icon 15px, radius 5.
    let mut ax = actions_x;
    for (icon, danger) in &actions {
        let b = Rect::from_min_size(pos2(ax, rect.min.y + 11.0), vec2(26.0, 26.0));
        let rr = ui.interact(b, egui::Id::new("dlccb").with(&card.gid).with(icon), Sense::click());
        if rr.hovered() {
            ui.painter().rect_filled(b, CornerRadius::same(5), p.hover_veil);
        }
        let color = match danger {
            ActionHover::None if rr.hovered() => p.text,
            ActionHover::None => p.text_muted,
            ActionHover::Paused if rr.hovered() => p.status_paused,
            ActionHover::Paused => p.text_muted,
            ActionHover::Error if rr.hovered() => p.status_error,
            ActionHover::Error => p.text_muted,
        };
        mi::paint_centered(ui.painter(), b.center(), icon, 15.0, color);
        if rr.clicked() {
            if *icon == "group_add" {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("dl_group_anchor"), b));
                state.g_menu_gid = if state.g_menu_gid.as_deref() == Some(card.gid.as_str()) {
                    None
                } else {
                    Some(card.gid.clone())
                };
            } else {
                run_card_action(state, ctx, card, action_of(icon));
            }
        }
        rr.on_hover_text(state.t(action_tooltip(icon)));
        ax += 28.0;
    }

    // â”€â”€ 2px progress strip along the bottom (inset 8px).
    let track = Rect::from_min_size(
        pos2(rect.min.x + 8.0, rect.max.y - 2.0),
        vec2(rect.width() - 16.0, 2.0),
    );
    ui.painter()
        .rect_filled(track, CornerRadius::same(1), p.hover_veil);
    if card.progress > 0.0 {
        let fw = track.width() * (card.progress / 100.0).clamp(0.0, 1.0);
        ui.painter()
            .rect_filled(Rect::from_min_size(track.min, vec2(fw, 2.0)), CornerRadius::same(1), card.progress_color());
    }
}

fn card_compact_actions(card: &CardView) -> Vec<CardAction> {
    let mut v: Vec<CardAction> = Vec::new();
    match card.status {
        Status::Active | Status::Waiting => v.push(("pause", ActionHover::None)),
        Status::Paused => v.push(("play_arrow", ActionHover::None)),
        Status::Error | Status::Removed => v.push(("refresh", ActionHover::None)),
        Status::Complete => v.push(("download", ActionHover::None)),
    }
    v.push(("folder_open", ActionHover::None));
    match card.status {
        Status::Active | Status::Waiting | Status::Paused => {
            v.push(("cancel", ActionHover::Paused))
        }
        _ => v.push(("close", ActionHover::Error)),
    }
    v
}

// â”€â”€ Empty state (downloads-page scoped: 60 20 pad, 48px icon @30%) â”€â”€â”€â”€â”€â”€

fn empty_state(state: &UiState, ui: &mut Ui, w: f32) {
    let p = mk::pal();
    let h = 224.0; // 60 + 48 + 12 + 2 text lines + 4 + 60
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    // Empty-state bg is the content surface; icon = text-dim @ 30% over it.
    let icon_c = mk::blend(p.bg_surface, p.text_dim, 0.3);
    mi::paint_centered(
        ui.painter(),
        pos2(rect.center().x, rect.min.y + 60.0 + 24.0),
        "cloud_download",
        48.0,
        icon_c,
    );
    draw_text(
        ui,
        pos2(rect.center().x, rect.min.y + 60.0 + 48.0 + 12.0 + 11.0),
        Align2::CENTER_CENTER,
        state.t("No downloads yet"),
        semibold(16.0),
        p.text,
    );
    draw_text(
        ui,
        pos2(rect.center().x, rect.min.y + 60.0 + 48.0 + 12.0 + 22.0 + 4.0 + 9.0),
        Align2::CENTER_CENTER,
        state.t("Click \"+\" to add your first download"),
        regular(13.0),
        p.text_dim,
    );
}
