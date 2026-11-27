//! Custom frameless title bar (port of `TitleBar.vue`): drag/move region,
//! centered search with live preview dropdown, download actions, window
//! controls — plus the invisible resize strips the native frame provided.

use crate::state::{Screen, UiState};
use crate::ui::icons as mi;
use crate::ui::widgets::{self as mk};
use egui::{
    pos2, vec2, Align2, CornerRadius, FontId, Key, Rect, Sense, Stroke, StrokeKind,
    TextEdit, ViewportCommand,
};

pub const HEIGHT: f32 = 50.0;
/// Title-bar search field id (`preview_search` + `search_preview` focus gate).
const SEARCH_ID: &str = "tb-search";
const SEARCH_PREVIEW_LIMIT: usize = 6;

/// `.search-input-wrap` fill: `rgba(255,255,255,0.04)` over bg-card.
fn search_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)
}

/// Panel layout order: called after the rail but before the central screen.
pub fn show(state: &mut UiState, ui: &mut egui::Ui) {
    // 0.35: TopBottomPanel was removed; the unified Panel shows inside the
    // root Ui. The floating overlays below still want a Context.
    let ctx = ui.ctx().clone();
    let ctx = &ctx;
    let p = mk::pal();
    let tb_resp = egui::Panel::top("titlebar")
        // `.titlebar { height: 50px; border-bottom: 1px }` — paint the 1px
        // border inside the panel's own `bg_card` fill rather than using
        // egui's separator strip (which leaves a 1px gap over the clear color).
        .show_separator_line(false)
        .exact_size(HEIGHT + 1.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg_card)
                .inner_margin(egui::Margin::symmetric(16, 0)),
        )
        .show(ui, |ui| {
            // Lay the 50px of content against the panel's top edge.
            let mr = ui.max_rect();
            let bar = Rect::from_min_size(mr.min, vec2(mr.width(), HEIGHT));
            ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                // `.titlebar-controls`/`.titlebar-actions { gap: 4px }`.
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.horizontal_centered(|ui| {
                    // `.titlebar-title`: 16px/500 text. Chromium has no 500 face and
                    // doesn't synthesize under 600, so the request renders the
                    // bundled 400 face plain — use it, not our real SemiBold.
                    let title = state.t(page_title(state.screen));
                    let tfont = egui::FontId::new(16.0, egui::FontFamily::Proportional);
                    let title_w = ui.painter().layout_no_wrap(title.to_string(), tfont.clone(), p.text).size().x;

                    // Window controls: 3 × 36px buttons + 2 × 4px gaps
                    // (`.titlebar-controls { gap: 4px }`). The optional
                    // `.update-badge` (auto width: 10 + icon 18 + gap 4 +
                    // label + 10) sits before them, with one more 4px gap.
                    let badge_w = if state.update_available {
                        let bf = egui::FontId::new(
                            11.0,
                            crate::ui::tokens::typography::semibold_family(),
                        );
                        let tw = ui
                            .painter()
                            .layout_no_wrap(state.t("Update").to_string(), bf, p.text)
                            .size()
                            .x;
                        10.0 + 18.0 + 4.0 + tw + 10.0
                    } else {
                        0.0
                    };
                    let controls_w = 36.0 * 3.0 + 4.0 * 2.0
                        + if state.update_available { badge_w + 4.0 } else { 0.0 };

                    // Title (left). The ref's glyph ink lands 0.5-1px lower
                    // than egui's galley centering, so bias +1px.
                    let (trect, _) = ui.allocate_exact_size(vec2(title_w, HEIGHT), Sense::hover());
                    ui.painter().text(
                        pos2(trect.min.x, trect.center().y + 1.0),
                        Align2::LEFT_CENTER,
                        title,
                        tfont,
                        p.text,
                    );

                    // Middle group: the 320px search (Downloads) or the two
                    // action buttons (Home). The old app absolutely centres the
                    // search on the titlebar itself
                    // (`.titlebar-search-center { left: 50%; translateX(-50%) }`).
                    let group_x = if state.screen == Screen::Downloads {
                        bar.center().x - 320.0 / 2.0
                    } else {
                        let group_w = 32.0 * 2.0 + 4.0;
                        let free = bar.width() - title_w - group_w - controls_w;
                        bar.min.x + title_w + free / 2.0
                    };
                    drag_to(ui, state, group_x);

                    // Middle group.
                    if state.screen == Screen::Downloads {
                        preview_search(ui, state, 320.0);
                    } else {
                        // `.titlebar-action`s, LTR [playlist_add][add].
                        let batch_btn = mk::icon_btn(
                            ui,
                            "playlist_add",
                            32.0,
                            18.0,
                            p.text_dim,
                            Some(p.hover_veil),
                            Some(p.text),
                        );
                        if batch_btn.clicked() {
                            state.show_batch_dialog = true;
                        }
                        batch_btn.on_hover_text(state.t("Batch Download"));
                        let add_btn = mk::icon_btn(
                            ui,
                            "add",
                            32.0,
                            18.0,
                            p.accent_strong,
                            Some(p.accent.gamma_multiply(0.2)),
                            Some(p.accent_strong),
                        );
                        if add_btn.clicked() {
                            state.show_new_download = true;
                            state.nd_focus_next = true;
                        }
                        add_btn.on_hover_text(state.t("New Download"));
                    }

                    // Drag filler up to the window controls (right): [-][□][✕].
                    drag_to(ui, state, bar.max.x - controls_w);

                    // `.update-badge` — 36px tall, green-tinted pill with a
                    // 6px notification dot at top 6 / right 6.
                    if state.update_available {
                        let (brect, bresp) = ui.allocate_exact_size(
                            vec2(badge_w, 36.0),
                            Sense::click(),
                        );
                        let bhov = bresp.hovered();
                        let green = |a: f32| {
                            egui::Color32::from_rgba_unmultiplied(
                                129,
                                199,
                                132,
                                (a * 255.0) as u8,
                            )
                        };
                        ui.painter().rect_filled(
                            brect,
                            CornerRadius::same(6),
                            green(if bhov { 0.16 } else { 0.10 }),
                        );
                        ui.painter().rect_stroke(
                            brect,
                            CornerRadius::same(6),
                            Stroke::new(1.0, green(if bhov { 0.32 } else { 0.22 })),
                            StrokeKind::Inside,
                        );
                        mi::paint_centered(
                            ui.painter(),
                            egui::pos2(brect.min.x + 10.0 + 9.0, brect.center().y),
                            "check_circle",
                            18.0,
                            p.status_complete,
                        );
                        ui.painter().text(
                            egui::pos2(
                                brect.min.x + 10.0 + 18.0 + 4.0,
                                brect.center().y,
                            ),
                            Align2::LEFT_CENTER,
                            state.t("Update"),
                            egui::FontId::new(
                                11.0,
                                crate::ui::tokens::typography::semibold_family(),
                            ),
                            p.status_complete,
                        );
                        // `.update-badge-dot`: 6px, 1.5px bg-card border.
                        let dot = egui::pos2(brect.max.x - 6.0 - 3.0, brect.min.y + 6.0 + 3.0);
                        ui.painter().circle_filled(dot, 3.0, p.bg_card);
                        ui.painter().circle_filled(dot, 1.5, p.status_complete);
                        if bresp.clicked() {
                            state.show_update_modal = true;
                        }
                    }

                    let min = mk::icon_btn(ui, "remove", 36.0, 18.0, p.text_dim, None, None);
                let max = mk::icon_btn(ui, "crop_square", 36.0, 18.0, p.text_dim, None, None);
                let close = mk::icon_btn(
                    ui,
                    "close",
                    36.0,
                    18.0,
                    p.text_dim,
                    Some(p.hover_veil),
                    Some(p.status_error),
                );
                if close.clicked() {
                    // `.titlebar-btn.close-btn` → `appWindow.close()`, which the
                    // reference intercepts into `close-requested` → CloseDialog.
                    state.show_close_dialog = true;
                }
                if max.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Maximized(!state.maximized));
                    state.maximized = !state.maximized;
                }
                if min.clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                min.on_hover_text(state.t("Minimize"));
                max.on_hover_text(state.t(if state.maximized { "Restore" } else { "Maximize" }));
                close.on_hover_text(state.t("Close"));

                // `.titlebar`: border-bottom 1px var(--border). egui's
                // TopBottomPanel already draws that separator line with
                // `visuals.widgets.noninteractive.bg_stroke` (set to p.border
                // in theme::visuals_for) — painting it here as well doubled
                // the alpha versus the reference.
                });
            });
        });
    let tr = tb_resp.response.rect;
    ui.painter()
        .hline(tr.x_range(), tr.max.y - 0.5, Stroke::new(1.0, p.border));

    // Floating resize overlays (N/S/E/W) — same for both strips.
    resize_overlays(ctx);

    // Floating preview under the search input.
    search_preview(ctx, state);
}

fn page_title(s: Screen) -> &'static str {
    match s {
        Screen::Home => "Dashboard",
        Screen::Downloads => "Downloads",
        Screen::Media => "Media",
        Screen::Addon => "Addon",
        Screen::Settings => "Settings",
    }
}

/// Thin invisible overlay **Area** at a window edge that starts native resize
/// drags. Unlike panels, areas do not consume layout — the content area keeps
/// its full height (fixed the CSS-parity gap between the data rows).
fn resize_area(ctx: &egui::Context, id: &str, rect: Rect, dir: egui::ResizeDirection) {
    egui::Area::new(egui::Id::new(id))
        .fixed_pos(rect.min)
        .constrain_to(rect)
        .interactable(true)
        .show(ctx, |ui| {
            let resp = ui.allocate_rect(rect, Sense::click_and_drag());
            if resp.drag_started_by(egui::PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
            let cursor = match dir {
                egui::ResizeDirection::North | egui::ResizeDirection::South => {
                    egui::CursorIcon::ResizeVertical
                }
                _ => egui::CursorIcon::ResizeHorizontal,
            };
            resp.on_hover_cursor(cursor);
        });
}

/// All four window-edge resize overlays (N/S/E/W, 5px, floating).
fn resize_overlays(ctx: &egui::Context) {
    let sr = ctx.content_rect();
    resize_area(ctx, "rs-n", Rect::from_min_size(sr.min, vec2(sr.width(), 5.0)), egui::ResizeDirection::North);
    resize_area(ctx, "rs-s", Rect::from_min_size(pos2(sr.min.x, sr.max.y - 5.0), vec2(sr.width(), 5.0)), egui::ResizeDirection::South);
    resize_area(ctx, "rs-e", Rect::from_min_size(pos2(sr.max.x - 5.0, sr.min.y), vec2(5.0, sr.height())), egui::ResizeDirection::East);
    resize_area(ctx, "rs-w", Rect::from_min_size(sr.min, vec2(5.0, sr.height())), egui::ResizeDirection::West);
}

/// Drag region with a fixed width (fillers between titlebar groups).
fn drag_fill(ui: &mut egui::Ui, state: &mut UiState, width: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(width.max(1.0), HEIGHT), Sense::click_and_drag());
    let _ = rect;
    if resp.double_clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!state.maximized));
        state.maximized = !state.maximized;
    }
    // `dragged_by` stays true for every frame of the drag, so StartDrag was
    // re-sent ~60x/s and each one re-entered the OS move loop mid-drag (the
    // visible stutter). Start it once, when the drag begins.
    if resp.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
}

/// Drag filler that ends exactly at `x` (the horizontal layout advances the
/// cursor by item_spacing after every allocation, so subtract it).
fn drag_to(ui: &mut egui::Ui, state: &mut UiState, x: f32) {
    let spacing = ui.spacing().item_spacing.x;
    drag_fill(ui, state, x - spacing - ui.cursor().min.x);
}

/// Centered search field; typing filters the list + populates the preview.
fn preview_search(ui: &mut egui::Ui, state: &mut UiState, width: f32) {
    let p = mk::pal();
    let id = egui::Id::new(SEARCH_ID);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    state.search_rect = Some(rect);

    // `.search-input-wrap`: 32px, 8r, `rgba(255,255,255,.04)` fill, border
    // → accent when focused (`:focus-within`). egui keys focus on the
    // TextEdit's own widget id (`.id_salt` only salts it), so the frame has
    // to be stroked after the field is added — that also fixes the accent
    // ring, which never lit up while this read the salt id.
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), search_fill());

    // `.search-input-wrap { padding: 0 10px; gap: 6px }` → the 16px icon box
    // starts at +11 and the field at +33 (the clear button takes the tail).
    // The icon gets the same +1 glyph bias as the window controls.
    let clear = !state.search.is_empty();
    crate::ui::icons::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 11.0 + 8.0, rect.center().y + 1.0),
        "search",
        16.0,
        p.text_dim,
    );
    let field = Rect::from_min_max(
        pos2(rect.min.x + 33.0, rect.center().y - 8.5),
        pos2(
            rect.max.x - 10.0 - if clear { 20.0 } else { 0.0 },
            rect.center().y + 8.5,
        ),
    );
    // `t()` before the `&mut state.search` borrow below.
    let placeholder = state.t("Search by name or URL...");
    let field_resp = ui
        .scope_builder(egui::UiBuilder::new().max_rect(field), |ui| {
            ui.add(
                TextEdit::singleline(&mut state.search)
                    .desired_width(field.width())
                    .font(egui::TextStyle::Small)
                    .hint_text(egui::RichText::new(placeholder).size(12.0))
                    .id_salt(id)
                    // egui defaults to `Margin::symmetric(4, 2)`; the old input
                    // has no padding of its own (it lives on the 10px wrap), so
                    // text starts exactly at +33 with 1px off the top.
                    .margin(egui::Margin::symmetric(0, 1))
                    .frame(egui::Frame::NONE),
            )
        })
        .inner;
    // `Response::has_focus` also requires `input.focused` (the *window* has OS
    // focus); the dropdown only cares about the widget.
    state.search_focused = ui.memory(|m| m.has_focus(field_resp.id));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(
            1.0,
            if state.search_focused {
                p.accent_strong
            } else {
                p.border
            },
        ),
        StrokeKind::Inside,
    );
    // `.search-clear-btn`
    if clear {
        let c = Rect::from_center_size(
            pos2(rect.max.x - 10.0 - 10.0, rect.center().y),
            vec2(20.0, 20.0),
        );
        let rr = ui.interact(c, egui::Id::new("tb-search-clear"), Sense::click());
        crate::ui::icons::paint_centered(
            ui.painter(),
            c.center(),
            "close",
            14.0,
            if rr.hovered() { p.text } else { p.text_dim },
        );
        if rr.clicked() {
            state.search.clear();
        }
    }
    // Ctrl+F focuses the search box (TitleBar global key handler).
    if ui.input(|i| (i.modifiers.command) && i.key_pressed(Key::F)) {
        ui.memory_mut(|m| m.request_focus(id));
    }
    if ui.input(|i| i.key_pressed(Key::Escape)) && state.search_focused {
        state.search.clear();
    }
}

/// Status icon + tag color per row (ported `statusIcon()` helper).
fn status_icon(status: crate::state::Status) -> &'static str {
    use crate::state::Status::*;
    match status {
        Active => "download",
        Waiting => "hourglass_top",
        Paused => "pause",
        Complete => "done",
        Error => "error_outline",
        Removed => "inbox",
    }
}

fn status_color(status: crate::state::Status) -> egui::Color32 {
    status.color()
}

/// Floating dropdown with matching downloads (title-bar search preview).
fn search_preview(ctx: &egui::Context, state: &mut UiState) {
    let p = mk::pal();
    let Some(rect) = state.search_rect else { return };
    let id = egui::Id::new(SEARCH_ID);
    let q = state.search.trim().to_lowercase();
    // `showPreview = searchEnabled && searchPreviewEnabled && searchFocused
    // && query.trim()` — the Settings → Search toggle was never consulted.
    // Enter inside the TextEdit surrenders focus before this runs, so the
    // previous frame's state keeps it open just long enough to act on it.
    let enabled = !q.is_empty() && state.settings.search_preview_enabled;
    if !(enabled && (state.search_focused || state.search_open)) {
        state.search_active = None;
        state.search_open = false;
        return;
    }

    let matches: Vec<&crate::state::DownloadVm> = state
        .downloads
        .iter()
        .filter(|d| d.filename.to_lowercase().contains(&q) || d.url.to_lowercase().contains(&q))
        .collect();
    let rows: Vec<(String, String, crate::state::Status, f32)> = matches
        .iter()
        .take(SEARCH_PREVIEW_LIMIT)
        .map(|d| (d.gid.clone(), d.filename.clone(), d.status, d.progress))
        .collect();
    let more = matches.len().saturating_sub(SEARCH_PREVIEW_LIMIT);

    // `onSearchKeydown`: ↓/↑ wrap through the rows, Enter opens the active one
    // and otherwise only dismisses the dropdown (the list keeps filtering).
    let n = rows.len();
    if n > 0 && ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
        state.search_active = Some(state.search_active.map_or(0, |a| (a + 1) % n));
    }
    if n > 0 && ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
        state.search_active = Some(match state.search_active {
            Some(0) | None => n - 1,
            Some(a) => a - 1,
        });
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        match state.search_active.and_then(|a| rows.get(a)) {
            Some(row) => state.start_flash(row.0.clone()),
            None => ctx.memory_mut(|m| m.surrender_focus(id)),
        }
        state.search_active = None;
        state.search_open = false;
    }

    let name_font = FontId::proportional(12.5);
    let sub_font = FontId::proportional(10.5);
    egui::Area::new(egui::Id::new("search-preview"))
        .order(egui::Order::Foreground)
        // `.search-preview { top: calc(100% + 5px); left: -1px; right: -1px }` —
        // sized to the search field, not to whatever the Area can reach.
        .fixed_pos(egui::pos2(rect.min.x - 1.0, rect.max.y + 5.0))
        .show(ctx, |ui| {
            ui.set_width(rect.width() + 2.0);
            // `.search-preview-item` has no margin; egui would insert 8px
            // of default item spacing between the stacked rows.
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(10))
                .stroke(Stroke::new(1.0, p.border))
                .inner_margin(egui::Margin::same(4))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 12],
                    blur: 32,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(90),
                })
                .show(ui, |ui| {
                    if rows.is_empty() {
                        // `.search-preview-empty { gap: 6px; padding: 12px 8px }`
                        let font = FontId::proportional(12.0);
                        let msg = state.t_of(&format!(
                            "{} “{}”",
                            state.raw("No matches for"),
                            state.search.trim()
                        ));
                        let tw = ui
                            .painter()
                            .layout_no_wrap(msg.clone(), font.clone(), p.text_dim)
                            .rect
                            .width();
                        let (r, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 40.0),
                            Sense::hover(),
                        );
                        let x = r.center().x - (16.0 + 6.0 + tw) / 2.0;
                        crate::ui::icons::paint_centered(
                            ui.painter(),
                            pos2(x + 8.0, r.center().y),
                            "search_off",
                            16.0,
                            p.text_dim,
                        );
                        ui.painter().text(
                            pos2(x + 22.0, r.center().y + 1.0),
                            Align2::LEFT_CENTER,
                            msg,
                            font,
                            p.text_dim,
                        );
                        return;
                    }
                    for (idx, (gid, name, status, progress)) in rows.iter().enumerate() {
                        let (row, resp) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                        let active = state.search_active == Some(idx);
                        if resp.hovered() || active {
                            ui.painter().rect_filled(row, CornerRadius::same(7), p.hover_veil);
                        }
                        // `.preview-status-icon`
                        crate::ui::icons::paint_centered(
                            ui.painter(),
                            row.min + vec2(16.0, 16.0),
                            status_icon(*status),
                            16.0,
                            status_color(*status),
                        );
                        // `.preview-name`: one ellipsized line with `<mark>` on
                        // the first match (`highlightMatch`).
                        let fitted = fit_text(ui, name, &name_font, row.width() - 40.0);
                        let (pre, mid, post) = split_match(&fitted, &q);
                        let mut x = row.min.x + 32.0;
                        let y = row.min.y + 11.0;
                        for (text, marked) in [(pre, false), (mid, true), (post, false)] {
                            if text.is_empty() {
                                continue;
                            }
                            let w = ui
                                .painter()
                                .layout_no_wrap(text.to_owned(), name_font.clone(), p.text)
                                .rect
                                .width();
                            if marked {
                                ui.painter().rect_filled(
                                    Rect::from_min_max(
                                        pos2(x - 1.0, y - 7.0),
                                        pos2(x + w + 1.0, y + 7.0),
                                    ),
                                    CornerRadius::same(2),
                                    egui::Color32::from_rgba_unmultiplied(
                                        p.accent.r(),
                                        p.accent.g(),
                                        p.accent.b(),
                                        89,
                                    ),
                                );
                            }
                            ui.painter().text(
                                pos2(x, y),
                                Align2::LEFT_CENTER,
                                text,
                                name_font.clone(),
                                p.text,
                            );
                            x += w;
                        }
                        let token = status.token();
                        let tw = ui
                            .painter()
                            .layout_no_wrap(
                                token.to_owned(),
                                sub_font.clone(),
                                egui::Color32::WHITE,
                            )
                            .rect
                            .width();
                        ui.painter().text(
                            row.min + vec2(32.0, 23.0),
                            Align2::LEFT_CENTER,
                            token,
                            sub_font.clone(),
                            status_color(*status),
                        );
                        ui.painter().text(
                            row.min + vec2(38.0 + tw, 23.0),
                            Align2::LEFT_CENTER,
                            format!("{:.0}%", progress),
                            sub_font.clone(),
                            p.text_dim,
                        );
                        if resp.clicked() {
                            state.start_flash(gid.clone());
                            state.search_active = None;
                        }
                    }
                    if more > 0 {
                        // `.search-preview-footer { padding: 6px 8px 4px; 11px; centered }`
                        let (f, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 21.0),
                            Sense::hover(),
                        );
                        ui.painter().text(
                            f.center() + vec2(0.0, 1.0),
                            Align2::CENTER_CENTER,
                            state.t_of(&format!(
                                "{more} {}",
                                state.raw("more — press Enter to filter list")
                            )),
                            FontId::proportional(11.0),
                            p.text_dim,
                        );
                    }
                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        state.search.clear();
                    }
                });
        });
    // One-frame latch: only survives while the field keeps focus.
    state.search_open = state.search_focused;
}

/// `.highlightMatch`: first case-insensitive occurrence only.
fn split_match<'a>(text: &'a str, q: &str) -> (&'a str, &'a str, &'a str) {
    let lower = text.to_lowercase();
    // Case folding can change byte lengths (rare non-ASCII); skip the mark then
    // rather than slice on the wrong boundary.
    if q.is_empty() || lower.len() != text.len() {
        return (text, "", "");
    }
    match lower.find(q) {
        Some(i) => (&text[..i], &text[i..i + q.len()], &text[i + q.len()..]),
        None => (text, "", ""),
    }
}

/// Single-line fit with a trailing ellipsis (`.preview-name` is
/// `white-space: nowrap; text-overflow: ellipsis`).
fn fit_text(ui: &egui::Ui, text: &str, font: &FontId, max_w: f32) -> String {
    let width = |s: String| {
        ui.painter()
            .layout_no_wrap(s, font.clone(), egui::Color32::WHITE)
            .rect
            .width()
    };
    if width(text.to_owned()) <= max_w {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let cand: String = chars[..mid].iter().collect::<String>() + "…";
        if width(cand) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + "…"
}
