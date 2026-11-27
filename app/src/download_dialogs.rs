//! Download dialogs ported from the Vue component layer:
//! `BatchUrlDialog.vue`, `ErrorDetailsDialog.vue`, `HashVerifyDialog.vue` and
//! `GlobalContextMenu.vue` (including its "Add to Group" submenu).
//!
//! Rendered at the very end of `modals::show`, so the context menu and these
//! dialogs stack above the other modals.

use crate::modals::nd_button;
use crate::screens::downloads;
use crate::screens::Ctx;
use crate::state::{Status, UiState};
use crate::ui::icons as mi;
use crate::ui::tokens::{self, Palette};
use crate::ui::widgets as mk;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Margin, Pos2, Rect, RichText, Sense, Ui,
};

/// `.nd-modal { max-width: 480px }`.
const BATCH_W: f32 = 480.0;

pub fn show(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    if state.show_batch_dialog {
        batch_modal(state, ctx, cctx);
    }
    if state.err_open {
        error_modal(state, ctx);
    }
    if state.hash_open {
        hash_modal(state, ctx, cctx);
    }
    // Last, so it sits above every dialog it may have opened from.
    if state.cm_open {
        context_menu(state, ctx, cctx);
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────

fn text_w(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x
}

/// Truncate with a trailing ellipsis to fit `max_w`.
fn ellipsize(ui: &Ui, text: &str, font: &FontId, max_w: f32) -> String {
    crate::rtl::truncate(text, |s| text_w(ui, s, font) <= max_w)
}

/// Full-width row measured against the *card* width (scroll bar compensation),
/// inset by `pad` on both sides — same trick as `modals::row_pad`.
fn row_pad(ui: &mut Ui, h: f32, card_w: f32, pad: f32) -> Rect {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    Rect::from_min_size(pos2(r.min.x + pad, r.min.y), vec2(card_w - 2.0 * pad, h))
}

/// `.nd-input` shell: faint white fill, MoonKit border, accent while focused.
pub(crate) fn input_shell(ui: &Ui, p: &Palette, rect: Rect, focused: bool, hovered: bool, radius: u8) {
    let fill = if focused {
        mk::blend(p.bg_card, p.accent, 0.03)
    } else if hovered {
        mk::blend(p.bg_card, Color32::WHITE, 0.035)
    } else {
        mk::blend(p.bg_card, Color32::WHITE, 0.025)
    };
    let stroke = if focused {
        p.accent_strong
    } else if hovered {
        // `.nd-input:hover { border-color: rgba(255,255,255,.1) }`
        mk::blend(p.bg_card, Color32::WHITE, 0.1)
    } else {
        p.border
    };
    ui.painter().rect_filled(rect, CornerRadius::same(radius), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(radius),
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
}

/// Single-line `TextEdit` placed into an explicit rect, vertically centered.
pub(crate) fn text_input(
    ui: &mut Ui,
    id: egui::Id,
    rect: Rect,
    value: &mut String,
    hint: &str,
    font: FontId,
    pad: i8,
    hint_gamma: f32,
) {
    let size = font.size;
    let hint_color = mk::pal().text_dim.gamma_multiply(hint_gamma);
    let line_h = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
    let text_rect = Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(text_rect).id(id.with("scope")),
        |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .desired_width(text_rect.width())
                    .font(font)
                    .text_color(mk::pal().text)
                    .hint_text(RichText::new(hint).size(size).color(hint_color))
                    .frame(egui::Frame::NONE.inner_margin(Margin {
                        left: pad,
                        right: pad,
                        top: 0,
                        bottom: 0,
                    })),
            );
        },
    );
    // The scoped editor shrinks the parent cursor to the text line; restore.
    ui.advance_cursor_after_rect(rect);
}

/// `.nd-icon-btn` / `.error-close` style 28×28 close affordance.
pub(crate) fn close_button(ui: &Ui, p: &Palette, rect: Rect, id: &str, danger_hover: bool) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    if rr.hovered() {
        let bg = if danger_hover {
            mk::blend(p.bg_card, Color32::from_rgb(0xef, 0x53, 0x50), 0.10)
        } else {
            mk::blend(p.bg_card, Color32::WHITE, 0.06)
        };
        ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
    }
    let col = if rr.hovered() && danger_hover {
        p.status_error
    } else if rr.hovered() {
        p.text
    } else {
        p.text_dim
    };
    mi::paint_centered(ui.painter(), rect.center(), "close", 16.0, col);
    rr.clicked()
}

/// Dim overlay; returns true when the user clicked it.
fn dim_overlay(ctx: &egui::Context, screen: Rect, id: &str, alpha: u8) -> bool {
    let mut clicked = false;
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                screen,
                CornerRadius::same(0),
                Color32::from_black_alpha(alpha),
            );
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                clicked = true;
            }
        });
    clicked
}

// ── BatchUrlDialog ──────────────────────────────────────────────────────

const BATCH_HINT: &str = "https://example.com/file001.zip\nhttps://example.com/file[002-010].zip\nhttps://example.com/video.[mp4,webm]";

fn batch_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let w = BATCH_W.min(screen.width() - 48.0).max(300.0);

    let mut close = dim_overlay(ctx, screen, "batch-overlay", 140);
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if close {
        state.show_batch_dialog = false;
        batch_reset(state);
        return;
    }

    let urls = expanded_urls(&state.batch_url_text);
    let count = urls.len();
    let submitting = state.batch_submitting;
    let can_submit = !submitting && count > 0;

    // Header metrics: avatar 36, title 14/1.2, subtitle 11 wrapped.
    let title_font = FontId::new(14.0, tokens::typography::semibold_family());
    let sub_font = FontId::new(11.0, egui::FontFamily::Proportional);
    let close_rect_w = 28.0;
    let title_x = 16.0 + 36.0 + 10.0;
    let title_w = (w - 16.0 - close_rect_w - 12.0 - title_x).max(80.0);
    let sub = state.t("Paste multiple URLs, one per line. Supports templates like [1-100] and {mp4,webm}");
    let sub_h = ctx.fonts_mut(|f| {
        f.layout(
            sub.to_owned(),
            sub_font.clone(),
            p.text_muted,
            title_w,
        )
        .size()
        .y
    });
    let title_h = 16.8;
    let header_h = 14.0 + (36.0_f32).max(title_h + 2.0 + sub_h) + 10.0;
    let footer_h = 10.0 + 38.0 + 14.0;

    // Body content height (used only to size the scroll viewport).
    let preview_items = count.min(10);
    let mut preview_lines = preview_items as f32 * 13.0
        + (preview_items.saturating_sub(1)) as f32 * 2.0;
    if count > 10 {
        preview_lines += 2.0 + 13.0;
    }
    let preview_h = if count > 0 {
        (14.0 + 6.0 + preview_lines + 20.0).min(150.0)
    } else {
        0.0
    };
    let mut body_h = 12.0 + 14.0 + 6.0 + 120.0 + 8.0;
    if count > 0 {
        body_h += 12.0 + preview_h;
    }
    body_h += 12.0 + 58.0;
    if !state.batch_error.is_empty() {
        body_h += 12.0 + 31.0;
    }
    let body_max = (screen.height() * 0.85 - header_h - footer_h - 1.0).max(120.0);

    let mut want_close = false;
    let mut want_submit = false;

    egui::Area::new(egui::Id::new("batch-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    // ── Header ──
                    let (h_rect, _) = ui.allocate_exact_size(vec2(w, header_h), Sense::hover());
                    let painter = ui.painter();
                    let avatar = Rect::from_min_size(pos2(h_rect.min.x + 16.0, h_rect.min.y + 14.0), vec2(36.0, 36.0));
                    painter.rect_filled(
                        avatar,
                        CornerRadius::same(10),
                        mk::blend(p.bg_card, p.accent, 0.10),
                    );
                    painter.rect_stroke(
                        avatar,
                        CornerRadius::same(10),
                        egui::Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.12)),
                        egui::StrokeKind::Inside,
                    );
                    mi::paint_centered(painter, avatar.center(), "playlist_add", 18.0, p.accent_strong);
                    painter.text(
                        pos2(h_rect.min.x + title_x, h_rect.min.y + 14.0),
                        Align2::LEFT_TOP,
                        state.t("Batch Download"),
                        title_font,
                        p.text,
                    );
                    let sub_galley = painter.layout(
                        sub.to_owned(),
                        sub_font,
                        p.text_muted,
                        title_w,
                    );
                    painter.galley(
                        pos2(h_rect.min.x + title_x, h_rect.min.y + 14.0 + title_h + 2.0),
                        sub_galley,
                        p.text_muted,
                    );
                    let close_r = Rect::from_min_size(
                        pos2(h_rect.min.x + w - 16.0 - close_rect_w, h_rect.min.y + 14.0),
                        vec2(close_rect_w, 28.0),
                    );
                    if close_button(ui, &p, close_r, "batch-close", true) {
                        want_close = true;
                    }
                    // `.nd-divider` — solid approximation of the gradient rule.
                    painter.rect_filled(
                        Rect::from_min_size(
                            pos2(h_rect.min.x + 16.0, h_rect.max.y),
                            vec2(w - 32.0, 1.0),
                        ),
                        CornerRadius::same(0),
                        p.border_strong,
                    );

                    // ── Body ──
                    // `.batch-textarea` uses the dialog's solid scroll bar.
                    ui.spacing_mut().scroll.bar_width = 4.0;
                    ui.spacing_mut().scroll.floating = false;
                    egui::ScrollArea::vertical()
                        .id_salt("batch-body")
                        .auto_shrink([false, true])
                        .max_height(body_max.min(body_h))
                        .show(ui, |ui| {
                            ui.set_width(w);
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            ui.add_space(12.0);

                            // Section header: "URLs" + count badge.
                            let sr = row_pad(ui, 14.0, w, 16.0);
                            ui.painter().text(
                                pos2(sr.min.x, sr.center().y),
                                Align2::LEFT_CENTER,
                                state.t("URLS"),
                                FontId::new(10.0, tokens::typography::semibold_family()),
                                p.text_dim.gamma_multiply(0.7),
                            );
                            if count > 0 {
                                let badge =
                                    state.t_of(&format!("{count} {}", state.raw("URLs")));
                                let bf = FontId::new(9.0, tokens::typography::semibold_family());
                                let bw = text_w(ui, &badge, &bf) + 12.0;
                                let pill = Rect::from_min_size(
                                    pos2(sr.max.x - bw, sr.min.y + 1.0),
                                    vec2(bw, 15.0),
                                );
                                ui.painter().rect_filled(
                                    pill,
                                    CornerRadius::same(8),
                                    mk::blend(p.bg_card, p.accent, 0.08),
                                );
                                ui.painter().rect_stroke(
                                    pill,
                                    CornerRadius::same(8),
                                    egui::Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.12)),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    pill.center(),
                                    Align2::CENTER_CENTER,
                                    badge,
                                    bf,
                                    p.accent_strong,
                                );
                            }
                            ui.add_space(6.0);

                            // Textarea (min-height 120, padding 10 12).
                            let ta = row_pad(ui, 120.0, w, 16.0);
                            let ta_id = egui::Id::new("batch-urls");
                            let focused = ui.ctx().memory(|m| m.has_focus(ta_id));
                            let hovered = ui.rect_contains_pointer(ta);
                            input_shell(ui, &p, ta, focused, hovered, 8);
                            let inner = ta.shrink2(vec2(12.0, 10.0));
                            ui.scope_builder(
                                egui::UiBuilder::new().max_rect(inner).id(ta_id.with("scope")),
                                |ui| {
                                    ui.set_clip_rect(inner);
                                    egui::ScrollArea::vertical()
                                        .id_salt(ta_id.with("scroll"))
                                        .max_height(inner.height())
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut state.batch_url_text)
                                                    .id(ta_id)
                                                    .desired_width(inner.width())
                                                    .desired_rows(7)
                                                    .font(FontId::monospace(12.0))
                                                    .text_color(p.text)
                                                    .hint_text(
                                                        RichText::new(BATCH_HINT)
                                                            .size(12.0)
                                                            .color(p.text_dim.gamma_multiply(0.55)),
                                                    )
                                                    .frame(egui::Frame::NONE),
                                            );
                                        });
                                },
                            );
                            ui.advance_cursor_after_rect(ta);

                            // Preview (only while count > 0).
                            if count > 0 {
                                ui.add_space(12.0);
                                let pb = row_pad(ui, preview_h, w, 16.0);
                                ui.painter().rect_filled(
                                    pb,
                                    CornerRadius::same(10),
                                    mk::blend(p.bg_card, Color32::WHITE, 0.02),
                                );
                                ui.painter().rect_stroke(
                                    pb,
                                    CornerRadius::same(10),
                                    egui::Stroke::new(1.0, p.border),
                                    egui::StrokeKind::Inside,
                                );
                                let inner = pb.shrink2(vec2(12.0, 10.0));
                                let ip = ui.painter().with_clip_rect(inner);
                                mi::paint_centered(
                                    &ip,
                                    pos2(inner.min.x + 7.0, inner.min.y + 7.0),
                                    "visibility",
                                    14.0,
                                    p.accent_strong,
                                );
                                ip.text(
                                    pos2(inner.min.x + 20.0, inner.min.y + 7.0),
                                    Align2::LEFT_CENTER,
                                    state.t("Preview (first 10)"),
                                    FontId::new(11.0, tokens::typography::semibold_family()),
                                    p.text_muted,
                                );
                                let list_w = inner.width();
                                let mut y = inner.min.y + 14.0 + 6.0;
                                let mono = FontId::new(11.0, egui::FontFamily::Monospace);
                                for u in urls.iter().take(10) {
                                    ip.text(
                                        pos2(inner.min.x, y + 6.5),
                                        Align2::LEFT_CENTER,
                                        ellipsize(ui, u, &mono, list_w),
                                        mono.clone(),
                                        p.text,
                                    );
                                    y += 13.0 + 2.0;
                                }
                                if count > 10 {
                                    ip.text(
                                        pos2(inner.min.x, y + 6.5),
                                        Align2::LEFT_CENTER,
                                        state.t_of(&format!(
                                            "... {} {} {}",
                                            state.raw("and"),
                                            count - 10,
                                            state.raw("more")
                                        )),
                                        FontId::new(11.0, egui::FontFamily::Proportional),
                                        p.text_dim,
                                    );
                                }
                                ui.advance_cursor_after_rect(pb);
                            }

                            // Save folder + connections.
                            ui.add_space(12.0);
                            let rr = row_pad(ui, 58.0, w, 16.0);
                            let conn_w = 92.0;
                            let left_w = rr.width() - conn_w - 12.0;
                            let left = Rect::from_min_size(rr.min, vec2(left_w, rr.height()));
                            let right = Rect::from_min_size(
                                pos2(rr.max.x - conn_w, rr.min.y),
                                vec2(conn_w, rr.height()),
                            );
                            field_label(ui, &p, left.min.into(), "folder", state.t("Save Folder"), Some(state.t("(optional)")));
                            field_label(ui, &p, right.min.into(), "layers", state.t("Connections"), None);
                            let lr = Rect::from_min_size(
                                pos2(left.min.x, left.min.y + 22.0),
                                vec2(left.width(), 36.0),
                            );
                            let cr = Rect::from_min_size(
                                pos2(right.min.x, right.min.y + 22.0),
                                vec2(right.width(), 36.0),
                            );
                            input_into(ui, &p, lr, "batch-dir", &mut state.batch_dir, "C:\\Downloads");
                            input_into(
                                ui,
                                &p,
                                cr,
                                "batch-connections",
                                &mut state.batch_connections,
                                "",
                            );
                            ui.advance_cursor_after_rect(rr);

                            // Error banner.
                            if !state.batch_error.is_empty() {
                                ui.add_space(12.0);
                                let eb = row_pad(ui, 31.0, w, 16.0);
                                ui.painter().rect_filled(
                                    eb,
                                    CornerRadius::same(8),
                                    mk::blend(p.bg_card, p.status_error, 0.08),
                                );
                                ui.painter().rect_stroke(
                                    eb,
                                    CornerRadius::same(8),
                                    egui::Stroke::new(1.0, mk::blend(p.bg_card, p.status_error, 0.2)),
                                    egui::StrokeKind::Inside,
                                );
                                mi::paint_centered(
                                    ui.painter(),
                                    pos2(eb.min.x + 10.0 + 7.5, eb.center().y),
                                    "warning",
                                    15.0,
                                    p.status_error,
                                );
                                ui.painter().with_clip_rect(eb).text(
                                    pos2(eb.min.x + 10.0 + 15.0 + 6.0, eb.center().y),
                                    Align2::LEFT_CENTER,
                                    &state.batch_error,
                                    FontId::new(11.0, egui::FontFamily::Proportional),
                                    p.status_error,
                                );
                                ui.advance_cursor_after_rect(eb);
                            }
                            ui.add_space(8.0);
                        });

                    // ── Footer ──
                    let (f_rect, _) = ui.allocate_exact_size(vec2(w, footer_h), Sense::hover());
                    ui.painter().rect_filled(
                        Rect::from_min_size(f_rect.min, vec2(w, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );
                    let bw = (w - 32.0 - 8.0) / 2.0;
                    let by = f_rect.min.y + 10.0;
                    let cancel = Rect::from_min_size(pos2(f_rect.min.x + 16.0, by), vec2(bw, 38.0));
                    let add = Rect::from_min_size(pos2(cancel.max.x + 8.0, by), vec2(bw, 38.0));
                    if nd_button(ui, &p, "batch-cancel", cancel, state.t("Cancel"), Some("close"), false, true)
                    {
                        want_close = true;
                    }
                    let add_label = if submitting {
                        state.t("Adding...").to_owned()
                    } else {
                        state.t_of(&format!(
                            "{} {count} {}",
                            state.raw("Add"),
                            state.raw("Downloads")
                        ))
                    };
                    let add_icon = if submitting { "hourglass_top" } else { "download" };
                    if nd_button(
                        ui,
                        &p,
                        "batch-add",
                        add,
                        &add_label,
                        Some(add_icon),
                        true,
                        can_submit,
                    ) {
                        want_submit = true;
                    }
                });
        });

    if want_close {
        state.show_batch_dialog = false;
        batch_reset(state);
    }
    if want_submit {
        batch_submit(state, cctx, urls);
    }
}

/// `.nd-label` row: muted icon + 12/600 label + optional 11px tail.
fn field_label(ui: &Ui, p: &Palette, at: Pos2, icon: &str, label: &str, tail: Option<&str>) {
    mi::paint_centered(
        ui.painter(),
        pos2(at.x + 7.0, at.y + 7.0),
        icon,
        14.0,
        p.text_muted.gamma_multiply(0.7),
    );
    let font = FontId::new(12.0, tokens::typography::semibold_family());
    ui.painter().text(
        pos2(at.x + 18.0, at.y + 7.0),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        p.text_muted,
    );
    if let Some(tail) = tail {
        let x = at.x + 18.0 + text_w(ui, label, &font) + 5.0;
        ui.painter().text(
            pos2(x, at.y + 7.0),
            Align2::LEFT_CENTER,
            tail,
            FontId::new(11.0, egui::FontFamily::Proportional),
            p.text_dim,
        );
    }
}

/// Input shell + single-line editor for the batch fields.
fn input_into(ui: &mut Ui, p: &Palette, rect: Rect, id: &str, value: &mut String, hint: &str) {
    let eid = egui::Id::new(id);
    let focused = ui.ctx().memory(|m| m.has_focus(eid));
    let hovered = ui.rect_contains_pointer(rect);
    input_shell(ui, p, rect, focused, hovered, 8);
    text_input(
        ui,
        eid,
        rect,
        value,
        hint,
        FontId::new(12.0, egui::FontFamily::Proportional),
        10,
        0.55,
    );
}

/// Vue `watch(visible)` close reset.
pub(crate) fn batch_reset(state: &mut UiState) {
    state.batch_url_text.clear();
    state.batch_dir.clear();
    state.batch_connections = "1".to_owned();
    state.batch_error.clear();
    state.batch_submitting = false;
}

fn batch_submit(state: &mut UiState, cctx: &Ctx, urls: Vec<String>) {
    if urls.is_empty() || state.batch_submitting {
        return;
    }
    // `Math.max(1, Number(connections.value) || 1)`.
    let connections = state
        .batch_connections
        .trim()
        .parse::<f64>()
        .ok()
        .map(|n| n.max(1.0) as u32)
        .unwrap_or(1)
        .max(1);
    let dir = {
        let d = state.batch_dir.trim().to_owned();
        if d.is_empty() {
            None
        } else {
            Some(d)
        }
    };
    // `rememberRequestedConnections(url, connections)` for every URL.
    for url in &urls {
        let norm = url.trim().to_lowercase();
        state.nd_conn_memory.retain(|(u, _)| *u != norm);
        state.nd_conn_memory.push((norm, connections));
    }
    state.batch_submitting = true;
    state.batch_error.clear();

    cctx.emit("batch-add-done", move |app| async move {
        let mut added = 0u64;
        let mut first_err: Option<String> = None;
        for url in urls {
            match moonhunt_core::aria2::add_download(
                app.clone(),
                url,
                None,
                Some(connections),
                dir.clone(),
                None,
                None,
                None,
                None,
            )
            .await
            {
                Ok(_) => added += 1,
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(serde_json::json!({ "count": added })),
        }
    });
}

// ── ErrorDetailsDialog ──────────────────────────────────────────────────

fn error_modal(state: &mut UiState, ctx: &egui::Context) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let w = 600.0_f32.min(screen.width() * 0.9);
    let pad = 32.0;
    let content_w = w - 2.0 * pad;

    let mut close = dim_overlay(ctx, screen, "err-overlay", 178);
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if close {
        state.err_open = false;
        return;
    }

    let filename = state.err_filename.clone();
    let message = state.err_message.clone();
    let title_font = FontId::new(20.0, tokens::typography::bold_family());
    let label_font = FontId::new(12.0, egui::FontFamily::Proportional);
    let name_font = FontId::new(14.0, egui::FontFamily::Proportional);
    let msg_font = FontId::new(13.0, egui::FontFamily::Monospace);

    let mut want_close = false;

    egui::Area::new(egui::Id::new("err-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(12))
                .stroke(egui::Stroke::new(1.0, p.border))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.add_space(pad);

                    // Header: warning + title, close on the right.
                    let (hr, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
                    mi::paint_centered(
                        ui.painter(),
                        pos2(hr.min.x + pad + 10.0, hr.center().y),
                        "warning",
                        20.0,
                        p.status_error,
                    );
                    ui.painter().text(
                        pos2(hr.min.x + pad + 20.0 + 8.0, hr.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Download Error"),
                        title_font,
                        p.status_error,
                    );
                    let close_r = Rect::from_min_size(
                        pos2(hr.max.x - pad - 32.0, hr.min.y),
                        vec2(32.0, 24.0),
                    );
                    // `.error-close` is a bare 24px Material glyph (no hover
                    // veil or color shift in the reference CSS).
                    let crr = ui.interact(close_r, egui::Id::new("err-close"), Sense::click());
                    mi::paint_centered(ui.painter(), close_r.center(), "close", 24.0, p.text_dim);
                    if crr.clicked() {
                        want_close = true;
                    }
                    ui.add_space(24.0);

                    // Filename section.
                    let (lr, _) = ui.allocate_exact_size(vec2(w, 14.4), Sense::hover());
                    ui.painter().text(
                        pos2(lr.min.x + pad, lr.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Filename:"),
                        label_font.clone(),
                        p.text_dim,
                    );
                    ui.add_space(8.0);
                    let (fr, _) = ui.allocate_exact_size(vec2(w, 16.8), Sense::hover());
                    let name = if filename.is_empty() { "\u{2014}" } else { filename.as_str() };
                    ui.painter().text(
                        pos2(fr.min.x + pad, fr.center().y),
                        Align2::LEFT_CENTER,
                        ellipsize(ui, name, &name_font, content_w),
                        name_font,
                        p.text,
                    );
                    ui.add_space(20.0);

                    // Error message section.
                    let (lr2, _) = ui.allocate_exact_size(vec2(w, 14.4), Sense::hover());
                    ui.painter().text(
                        pos2(lr2.min.x + pad, lr2.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Error Details:"),
                        label_font,
                        p.text_dim,
                    );
                    ui.add_space(8.0);
                    let galley = ui.painter().layout(
                        message.clone(),
                        msg_font.clone(),
                        p.status_error,
                        content_w - 32.0,
                    );
                    let box_h = (galley.size().y + 32.0).min(300.0).max(49.0);
                    let (bx, _) = ui.allocate_exact_size(vec2(w, box_h), Sense::hover());
                    let bw = bx.min.x + pad;
                    let box_r = Rect::from_min_size(pos2(bw, bx.min.y), vec2(content_w, box_h));
                    ui.painter().rect_filled(
                        box_r,
                        CornerRadius::same(6),
                        p.bg_card,
                    );
                    ui.painter().rect_stroke(
                        box_r,
                        CornerRadius::same(6),
                        egui::Stroke::new(1.0, p.border),
                        egui::StrokeKind::Inside,
                    );
                    let inner = box_r.shrink2(vec2(16.0, 16.0));
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(inner).id(egui::Id::new("err-msg")),
                        |ui| {
                            ui.set_clip_rect(inner);
                            egui::ScrollArea::vertical()
                                .id_salt("err-msg-scroll")
                                .auto_shrink([false, true])
                                .max_height(inner.height())
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(message.clone())
                                                .font(msg_font)
                                                .color(p.status_error),
                                        )
                                        .wrap(),
                                    );
                                });
                        },
                    );
                    ui.add_space(20.0);

                    // Footer: right-aligned accent Close button.
                    let (fr2, _) = ui.allocate_exact_size(vec2(w, 37.0), Sense::hover());
                    let bw2 = text_w(ui, "Close", &FontId::new(14.0, tokens::typography::semibold_family()))
                        + 48.0;
                    let btn = Rect::from_min_size(
                        pos2(fr2.max.x - pad - bw2, fr2.min.y),
                        vec2(bw2, 37.0),
                    );
                    let rr = ui.interact(btn, egui::Id::new("err-close-btn"), Sense::click());
                    ui.painter().rect_filled(
                        btn,
                        CornerRadius::same(8),
                        if rr.hovered() {
                            mk::blend(p.accent, Color32::WHITE, 0.08)
                        } else {
                            p.accent
                        },
                    );
                    ui.painter().text(
                        btn.center(),
                        Align2::CENTER_CENTER,
                        state.t("Close"),
                        FontId::new(14.0, tokens::typography::semibold_family()),
                        Color32::from_rgb(0x0a, 0x0e, 0x1a),
                    );
                    if rr.clicked() {
                        want_close = true;
                    }
                    ui.add_space(pad);
                });
        });

    if want_close {
        state.err_open = false;
    }
}

// ── HashVerifyDialog ────────────────────────────────────────────────────

fn hash_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let w = 440.0_f32.min(screen.width() * 0.9);
    let pad = 24.0;
    let content_w = w - 2.0 * pad;

    let mut close = dim_overlay(ctx, screen, "hash-overlay", 140);
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }
    if close {
        hash_close(state);
        return;
    }

    let algo = state.hash_algo.clone();
    let computed = state.hash_computed.clone();
    let expected = state.hash_expected.clone();
    let match_result: Option<bool> = if computed.is_empty() || expected.trim().is_empty() {
        None
    } else {
        Some(computed.to_lowercase() == expected.trim().to_lowercase())
    };
    let busy = state.hash_busy;
    let can_verify = !busy && !state.hash_path.is_empty();
    let has_error = !state.hash_error.is_empty();

    let label_font = FontId::new(12.0, tokens::typography::semibold_family());
    let title_font = FontId::new(16.0, tokens::typography::bold_family());
    let sub_font = FontId::new(12.0, egui::FontFamily::Proportional);

    let mut want_close = false;
    let mut want_verify = false;
    let mut picked_algo: Option<String> = None;

    // Result block metrics (computed before layout so rows know their height).
    let value_font = FontId::new(11.0, egui::FontFamily::Monospace);
    let result_h: f32 = if computed.is_empty() {
        0.0
    } else if expected.trim().is_empty() {
        10.0 + 20.0_f32.max(15.6) + 10.0
    } else {
        let vh = ctx.fonts_mut(|f| {
            f.layout(
                computed.clone(),
                value_font.clone(),
                p.text_dim,
                content_w - 24.0 - 20.0 - 10.0,
            )
            .size()
            .y
        });
        10.0 + (15.6 + 2.0 + vh).max(20.0) + 10.0
    };

    let algo_font = FontId::new(12.0, tokens::typography::semibold_family());
    let algo_h: f32 = (6.0_f32 + 6.0 + 14.4).max(27.0);

    egui::Area::new(egui::Id::new("hash-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(16))
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .inner_margin(Margin::same(0))
                // `box-shadow: 0 16px 48px rgba(0,0,0,.25)` — soft underlay.
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(64),
                })
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.add_space(pad);

                    // Header: 40px accent tile + title/filename.
                    let (hr, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
                    let tile = Rect::from_min_size(pos2(hr.min.x + pad, hr.min.y), vec2(40.0, 40.0));
                    ui.painter().rect_filled(
                        tile,
                        CornerRadius::same(10),
                        mk::blend(p.bg_card, p.accent, 0.10),
                    );
                    mi::paint_centered(
                        ui.painter(),
                        tile.center(),
                        "verified_user",
                        20.0,
                        p.accent_strong,
                    );
                    let tx = tile.max.x + 12.0;
                    ui.painter().text(
                        pos2(tx, hr.min.y + 9.6),
                        Align2::LEFT_CENTER,
                        state.t("Verify File Hash"),
                        title_font,
                        p.text,
                    );
                    let sub_name = if state.hash_name.is_empty() {
                        "\u{2014}".to_owned()
                    } else {
                        state.hash_name.clone()
                    };
                    ui.painter().text(
                        pos2(tx, hr.min.y + 19.2 + 2.0 + 7.2),
                        Align2::LEFT_CENTER,
                        ellipsize(
                            ui,
                            &sub_name,
                            &sub_font,
                            320.0_f32.min((w - 2.0 * pad - 40.0 - 12.0).max(40.0)),
                        ),
                        sub_font,
                        p.text_dim,
                    );
                    ui.add_space(16.0);

                    // Algorithm row.
                    let (lr, _) = ui.allocate_exact_size(vec2(w, 14.4), Sense::hover());
                    ui.painter().text(
                        pos2(lr.min.x + pad, lr.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Algorithm"),
                        label_font.clone(),
                        p.text_muted,
                    );
                    ui.add_space(6.0);
                    let (ar, _) = ui.allocate_exact_size(vec2(w, algo_h), Sense::hover());
                    let mut ax = ar.min.x + pad;
                    for name in ["md5", "sha1", "sha256", "sha512"] {
                        let label = name.to_uppercase();
                        let bw = text_w(ui, &label, &algo_font) + 28.0;
                        let b = Rect::from_min_size(pos2(ax, ar.min.y), vec2(bw, algo_h));
                        let rr = ui.interact(
                            b,
                            egui::Id::new("hash-algo").with(name),
                            Sense::click(),
                        );
                        let active = algo == name;
                        let (bg, fg, border) = if active {
                            (
                                mk::blend(p.bg_card, p.accent, 0.10),
                                p.accent_strong,
                                mk::blend(p.bg_card, p.accent, 0.30),
                            )
                        } else {
                            (p.hover_veil, p.text_dim, p.border)
                        };
                        ui.painter()
                            .rect_filled(b, CornerRadius::same(8), bg);
                        ui.painter().rect_stroke(
                            b,
                            CornerRadius::same(8),
                            egui::Stroke::new(
                                1.0,
                                if rr.hovered() && !active {
                                    mk::blend(p.border, p.accent, 0.20)
                                } else {
                                    border
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            b.center(),
                            Align2::CENTER_CENTER,
                            label,
                            algo_font.clone(),
                            fg,
                        );
                        if rr.clicked() {
                            picked_algo = Some(name.to_owned());
                        }
                        ax += bw + 6.0;
                    }
                    ui.add_space(16.0);

                    // Expected hash.
                    let (lr2, _) = ui.allocate_exact_size(vec2(w, 14.4), Sense::hover());
                    ui.painter().text(
                        pos2(lr2.min.x + pad, lr2.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Expected Hash"),
                        label_font.clone(),
                        p.text_muted,
                    );
                    let opt_x = lr2.min.x + pad + text_w(ui, state.t("Expected Hash"), &label_font) + 4.0;
                    ui.painter().text(
                        pos2(opt_x, lr2.center().y),
                        Align2::LEFT_CENTER,
                        state.t("(paste if you have one)"),
                        FontId::new(11.0, egui::FontFamily::Proportional),
                        p.text_dim,
                    );
                    ui.add_space(6.0);
                    let (ir, _) = ui.allocate_exact_size(vec2(w, 38.0), Sense::hover());
                    let input = Rect::from_min_size(
                        pos2(ir.min.x + pad, ir.min.y),
                        vec2(content_w, 38.0),
                    );
                    let hid = egui::Id::new("hash-expected");
                    let focused = ui.ctx().memory(|m| m.has_focus(hid));
                    // `.hash-input { background: rgba(255,255,255,.03) }`
                    ui.painter().rect_filled(
                        input,
                        CornerRadius::same(8),
                        if focused {
                            mk::blend(p.bg_card, p.accent, 0.03)
                        } else {
                            mk::blend(p.bg_card, Color32::WHITE, 0.03)
                        },
                    );
                    ui.painter().rect_stroke(
                        input,
                        CornerRadius::same(8),
                        egui::Stroke::new(
                            1.0,
                            if focused { p.accent_strong } else { p.border },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    // Hoisted: `text_input` takes `&mut state.hash_expected`.
                    let hash_hint = state.t("Paste expected hash here...");
                    text_input(
                        ui,
                        hid,
                        input,
                        &mut state.hash_expected,
                        hash_hint,
                        FontId::new(12.0, egui::FontFamily::Monospace),
                        12,
                        1.0,
                    );
                    ui.advance_cursor_after_rect(ir);
                    ui.add_space(16.0);

                    // Result box.
                    if !computed.is_empty() {
                        let (br, _) = ui.allocate_exact_size(vec2(w, result_h), Sense::hover());
                        let box_r = Rect::from_min_size(
                            pos2(br.min.x + pad, br.min.y),
                            vec2(content_w, result_h),
                        );
                        let (bg, border, icon, icon_col, title, title_col) = match match_result {
                            Some(true) => (
                                mk::blend(p.bg_card, Color32::from_rgb(0x4c, 0xaf, 0x50), 0.08),
                                mk::blend(p.bg_card, Color32::from_rgb(0x4c, 0xaf, 0x50), 0.25),
                                "check_circle",
                                Color32::from_rgb(0x4c, 0xaf, 0x50),
                                state.t("Hash matches!"),
                                Color32::from_rgb(0x4c, 0xaf, 0x50),
                            ),
                            Some(false) => (
                                mk::blend(p.bg_card, p.status_error, 0.08),
                                mk::blend(p.bg_card, p.status_error, 0.25),
                                "error",
                                p.status_error,
                                state.t("Hash mismatch!"),
                                p.status_error,
                            ),
                            None => (
                                mk::blend(p.bg_card, Color32::WHITE, 0.02),
                                p.border,
                                "info",
                                p.text_muted,
                                state.t("Computing..."),
                                p.text_muted,
                            ),
                        };
                        ui.painter()
                            .rect_filled(box_r, CornerRadius::same(10), bg);
                        ui.painter().rect_stroke(
                            box_r,
                            CornerRadius::same(10),
                            egui::Stroke::new(1.0, border),
                            egui::StrokeKind::Inside,
                        );
                        mi::paint_centered(
                            ui.painter(),
                            pos2(box_r.min.x + 12.0 + 10.0, box_r.center().y),
                            icon,
                            20.0,
                            icon_col,
                        );
                        let tx = box_r.min.x + 12.0 + 20.0 + 10.0;
                        let ty = box_r.min.y + 10.0;
                        ui.painter().text(
                            pos2(tx, ty + 7.8),
                            Align2::LEFT_CENTER,
                            title,
                            FontId::new(13.0, tokens::typography::semibold_family()),
                            title_col,
                        );
                        if match_result.is_some() {
                            let galley = ui.painter().layout(
                                computed.clone(),
                                value_font.clone(),
                                p.text_dim,
                                content_w - 24.0 - 20.0 - 10.0,
                            );
                            ui.painter().galley(pos2(tx, ty + 15.6 + 2.0), galley, p.text_dim);
                        }
                        ui.advance_cursor_after_rect(br);
                        ui.add_space(16.0);
                    }

                    // Error banner.
                    if has_error {
                        let (er, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
                        let box_r = Rect::from_min_size(
                            pos2(er.min.x + pad, er.min.y),
                            vec2(content_w, 32.0),
                        );
                        ui.painter()
                            .rect_filled(box_r, CornerRadius::same(8), mk::blend(p.bg_card, p.status_error, 0.08));
                        mi::paint_centered(
                            ui.painter(),
                            pos2(box_r.min.x + 12.0 + 8.0, box_r.center().y),
                            "warning",
                            16.0,
                            p.status_error,
                        );
                        ui.painter().with_clip_rect(box_r).text(
                            pos2(box_r.min.x + 12.0 + 16.0 + 6.0, box_r.center().y),
                            Align2::LEFT_CENTER,
                            &state.hash_error,
                            FontId::new(12.0, egui::FontFamily::Proportional),
                            p.status_error,
                        );
                        ui.advance_cursor_after_rect(er);
                        ui.add_space(16.0);
                    }

                    // Footer: Cancel (ghost) + Verify (primary), right aligned.
                    ui.add_space(4.0);
                    let (fr, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
                    let bf = FontId::new(13.0, tokens::typography::semibold_family());
                    let verify_label = if busy { state.t("Computing...") } else { state.t("Verify Hash") };
                    let verify_w = text_w(ui, verify_label, &bf) + 36.0 + if busy { 22.0 } else { 0.0 };
                    let verify = Rect::from_min_size(
                        pos2(fr.max.x - pad - verify_w, fr.min.y),
                        vec2(verify_w, 32.0),
                    );
                    let cancel_w = text_w(ui, state.t("Cancel"), &bf) + 36.0;
                    let cancel = Rect::from_min_size(
                        pos2(verify.min.x - 8.0 - cancel_w, fr.min.y),
                        vec2(cancel_w, 32.0),
                    );
                    let cr = ui.interact(cancel, egui::Id::new("hash-cancel"), Sense::click());
                    ui.painter().rect_filled(
                        cancel,
                        CornerRadius::same(8),
                        if cr.hovered() {
                            mk::blend(p.bg_card, Color32::WHITE, 0.08)
                        } else {
                            p.hover_veil
                        },
                    );
                    ui.painter().text(
                        cancel.center(),
                        Align2::CENTER_CENTER,
                        state.t("Cancel"),
                        bf.clone(),
                        p.text,
                    );
                    if cr.clicked() {
                        want_close = true;
                    }

                    let vr = ui.interact(verify, egui::Id::new("hash-verify"), Sense::click());
                    let fill = if can_verify {
                        p.accent
                    } else {
                        mk::blend(p.bg_card, p.accent, 0.5)
                    };
                    ui.painter()
                        .rect_filled(verify, CornerRadius::same(8), fill);
                    if busy {
                        mi::paint_centered(
                            ui.painter(),
                            pos2(verify.center().x - verify_w * 0.5 + 18.0, verify.center().y),
                            "hourglass_top",
                            16.0,
                            Color32::from_rgb(0x0a, 0x0e, 0x1a),
                        );
                    }
                    let text_off = if busy { 11.0 } else { 0.0 };
                    ui.painter().text(
                        pos2(verify.center().x + text_off, verify.center().y),
                        Align2::CENTER_CENTER,
                        verify_label,
                        bf,
                        Color32::from_rgb(0x0a, 0x0e, 0x1a),
                    );
                    if vr.clicked() && can_verify {
                        want_verify = true;
                    }
                    ui.add_space(pad);
                });
        });

    if let Some(name) = picked_algo {
        state.hash_algo = name;
    }
    if want_close {
        hash_close(state);
    }
    if want_verify {
        let path = state.hash_path.clone();
        let algo = state.hash_algo.clone();
        state.hash_busy = true;
        state.hash_error.clear();
        state.hash_computed.clear();
        cctx.emit("hash-computed", move |_app| async move {
            moonhunt_core::hash::compute_file_hash(path, algo)
                .await
                .map(|s| serde_json::json!(s))
        });
    }
}

/// Vue `watch(visible)` close reset (algorithm persists).
fn hash_close(state: &mut UiState) {
    state.hash_open = false;
    state.hash_computed.clear();
    state.hash_expected.clear();
    state.hash_error.clear();
    state.hash_busy = false;
}

// ── GlobalContextMenu ───────────────────────────────────────────────────

struct CmItem {
    action: &'static str,
    label: &'static str,
    icon: &'static str,
    shortcut: bool,
}

fn cm_items(status: Status, show_group: bool) -> Vec<CmItem> {
    let mut items = Vec::new();
    match status {
        Status::Active | Status::Waiting => {
            items.push(CmItem { action: "pause", label: "Pause", icon: "pause", shortcut: false });
            items.push(CmItem { action: "cancel", label: "Cancel", icon: "cancel", shortcut: false });
        }
        Status::Paused => {
            items.push(CmItem { action: "resume", label: "Resume", icon: "play_arrow", shortcut: false });
            items.push(CmItem { action: "cancel", label: "Cancel", icon: "cancel", shortcut: false });
        }
        Status::Error | Status::Removed => {
            items.push(CmItem { action: "retry", label: "Retry", icon: "refresh", shortcut: false });
        }
        Status::Complete => {
            items.push(CmItem { action: "redownload", label: "Redownload", icon: "download", shortcut: false });
            items.push(CmItem { action: "verifyHash", label: "Verify Hash", icon: "verified_user", shortcut: false });
        }
    }
    // Only offered while the download is still running and groups exist.
    if show_group {
        items.push(CmItem { action: "addToGroup", label: "Add to Group", icon: "playlist_add", shortcut: true });
    }
    items.push(CmItem { action: "folder", label: "Show in Folder", icon: "folder_open", shortcut: false });
    items.push(CmItem { action: "copyUrl", label: "Copy URL", icon: "link", shortcut: false });
    items.push(CmItem { action: "copyFilename", label: "Copy Filename", icon: "content_copy", shortcut: false });
    items.push(CmItem { action: "details", label: "Details", icon: "info", shortcut: true });
    items
}

fn context_menu(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let Some(gid) = state.cm_gid.clone() else {
        state.cm_open = false;
        return;
    };
    let Some(vm) = state.downloads.iter().find(|d| d.gid == gid).cloned() else {
        state.cm_open = false;
        return;
    };
    let p = mk::pal();
    let screen = ctx.content_rect();
    let show_group = vm.status != Status::Complete && !state.groups.is_empty();
    let items = cm_items(vm.status, show_group);
    let danger = matches!(
        vm.status,
        Status::Complete | Status::Error | Status::Paused | Status::Removed
    );

    // `.global-context-menu`: width auto, `min-width:200px; max-width:280px`,
    // so the header's filename sets the width up to the cap.
    let name_font = FontId::new(12.0, tokens::typography::semibold_family());
    let name = if vm.filename.is_empty() {
        state.t("Download").to_owned()
    } else {
        vm.filename.clone()
    };
    let name_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(name.clone(), name_font.clone(), p.text)
            .size()
            .x
    });
    let menu_w = (name_w + 24.0).clamp(200.0, 280.0);
    const HEADER_H: f32 = 46.0;
    // `.context-menu-item`: 9px padding around a ~15.6px line.
    const ITEM_H: f32 = 34.0;
    let mut menu_h = HEADER_H + 1.0 + items.len() as f32 * ITEM_H;
    if danger {
        menu_h += 1.0 + ITEM_H;
    }
    let vw = screen.width();
    let vh = screen.height();
    let x = state.cm_pos.x.clamp(8.0, (vw - menu_w - 8.0).max(8.0));
    let y = state
        .cm_pos
        .y
        .clamp(8.0, (vh - menu_h - 8.0).max(8.0));

    let mut act: Option<&'static str> = None;
    let mut close = ctx.input(|i| i.key_pressed(Key::Escape));
    let menu_rect = Rect::from_min_size(pos2(x, y), vec2(menu_w, menu_h));
    // Submenu rect from the previous frame (same geometry, so the click that
    // lands on a group row counts as "inside").
    let sub_rect = if state.cm_group_menu {
        ctx.data(|d| d.get_temp::<Rect>(egui::Id::new("cm_sub_rect")))
    } else {
        None
    };

    // The old `handleClickOutside` only reacts to a primary click outside the
    // menu (right-clicks are ignored). A full-screen backdrop Area would
    // steal hover/clicks from the menu itself, so test the pointer instead.
    if !close && ctx.input(|i| i.pointer.primary_clicked()) {
        let inside = ctx.pointer_interact_pos().is_some_and(|q| {
            menu_rect.contains(q) || sub_rect.is_some_and(|s| s.contains(q))
        });
        if !inside {
            close = true;
        }
    }

    let mut open_sub: Option<Rect> = None;
    if !close {
        egui::Area::new(egui::Id::new("cm-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(x, y))
            .show(ctx, |ui| {
                let (menu, _) = ui.allocate_exact_size(vec2(menu_w, menu_h), Sense::click());
                ui.painter().rect_filled(
                    menu,
                    CornerRadius::same(10),
                    p.bg_card,
                );
                ui.painter().rect_stroke(
                    menu,
                    CornerRadius::same(10),
                    egui::Stroke::new(1.0, p.border),
                    egui::StrokeKind::Inside,
                );

                // Header: filename + first 8 gid chars.
                ui.painter().with_clip_rect(menu).text(
                    pos2(menu.min.x + 12.0, menu.min.y + 10.0 + 7.2),
                    Align2::LEFT_CENTER,
                    ellipsize(ui, &name, &name_font, menu_w - 24.0),
                    name_font,
                    p.text,
                );
                let gid8: String = vm.gid.chars().take(8).collect();
                ui.painter().text(
                    pos2(menu.min.x + 12.0, menu.min.y + 10.0 + 14.4 + 2.0 + 6.0),
                    Align2::LEFT_CENTER,
                    gid8,
                    FontId::new(10.0, egui::FontFamily::Monospace),
                    p.text_dim,
                );
                // `.context-menu-header { border-bottom }` + the template's `<hr>`.
                let mut yy = menu.min.y + HEADER_H;
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(menu.min.x, yy), vec2(menu_w, 1.0)),
                    CornerRadius::same(0),
                    p.border,
                );
                yy += 1.0;
                ui.painter().rect_filled(
                    Rect::from_min_size(pos2(menu.min.x, yy), vec2(menu_w, 1.0)),
                    CornerRadius::same(0),
                    p.border,
                );

                let label_font = FontId::new(13.0, egui::FontFamily::Proportional);
                for (i, item) in items.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(menu.min.x, yy), vec2(menu_w, ITEM_H));
                    let rr = ui.interact(
                        r,
                        egui::Id::new("cm-item").with(i),
                        Sense::click(),
                    );
                    if rr.hovered() {
                        ui.painter()
                            .rect_filled(r, CornerRadius::same(0), p.hover_veil);
                    }
                    mi::paint_centered(
                        ui.painter(),
                        pos2(r.min.x + 12.0 + 11.0, r.center().y),
                        item.icon,
                        18.0,
                        p.text,
                    );
                    ui.painter().text(
                        pos2(r.min.x + 12.0 + 22.0 + 10.0, r.center().y),
                        Align2::LEFT_CENTER,
                        state.t(item.label),
                        label_font.clone(),
                        p.text,
                    );
                    if item.shortcut {
                        mi::paint_centered(
                            ui.painter(),
                            pos2(r.max.x - 12.0 - 7.0, r.center().y),
                            "keyboard_arrow_right",
                            14.0,
                            p.text_dim.gamma_multiply(0.6),
                        );
                    }
                    if rr.clicked() {
                        if item.action == "addToGroup" {
                            open_sub = Some(r);
                        } else {
                            act = Some(item.action);
                            close = true;
                        }
                    }
                    yy += ITEM_H;
                }

                if danger {
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(menu.min.x, yy), vec2(menu_w, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );
                    yy += 1.0;
                    let r = Rect::from_min_size(pos2(menu.min.x, yy), vec2(menu_w, ITEM_H));
                    let rr = ui.interact(r, egui::Id::new("cm-remove"), Sense::click());
                    if rr.hovered() {
                        ui.painter().rect_filled(
                            r,
                            CornerRadius::same(0),
                            mk::blend(p.bg_card, Color32::from_rgb(0xef, 0x53, 0x50), 0.12),
                        );
                    }
                    mi::paint_centered(
                        ui.painter(),
                        pos2(r.min.x + 12.0 + 11.0, r.center().y),
                        "delete",
                        18.0,
                        p.status_error,
                    );
                    ui.painter().text(
                        pos2(r.min.x + 12.0 + 22.0 + 10.0, r.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Remove"),
                        label_font,
                        p.status_error,
                    );
                    if rr.clicked() {
                        act = Some("remove");
                        close = true;
                    }
                }
            });
    }

    // ── "Add to Group" submenu (second `.global-context-menu`) ─────────────────────
    // Anchored to the right of the row that opened it, like `.submenuAnchor`
    // (`rect.right + 4, rect.top`), and clamped the same way.
    if let Some(r) = open_sub {
        state.cm_group_menu = true;
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("cm_sub_anchor"), pos2(r.max.x + 4.0, r.min.y))
        });
    }
    if state.cm_group_menu && !close {
        {
            let anchor = ctx
                .data(|d| d.get_temp::<egui::Pos2>(egui::Id::new("cm_sub_anchor")))
                .unwrap_or_else(|| {
                    // Harness path: no row was clicked, so anchor at the row's slot.
                    let idx = items
                        .iter()
                        .position(|i| i.action == "addToGroup")
                        .unwrap_or(0);
                    pos2(
                        menu_rect.max.x + 4.0,
                        menu_rect.min.y + HEADER_H + 1.0 + idx as f32 * ITEM_H,
                    )
                });
            let title_font = FontId::new(12.0, tokens::typography::semibold_family());
            let item_font = FontId::new(13.0, egui::FontFamily::Proportional);
            let mut w = 200.0f32;
            for g in &state.groups {
                let tw = ctx.fonts_mut(|f| {
                    f.layout_no_wrap(g.name.clone(), item_font.clone(), p.text)
                        .size()
                        .x
                });
                w = w.max(12.0 + 22.0 + 10.0 + tw + 12.0);
            }
            let w = w.min(280.0);
            let h = 36.0 + 1.0 + state.groups.len() as f32 * ITEM_H;
            let sx = anchor.x.clamp(8.0, (vw - w - 8.0).max(8.0));
            let sy = anchor.y.clamp(8.0, (vh - h - 8.0).max(8.0));
            let sub = Rect::from_min_size(pos2(sx, sy), vec2(w, h));
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("cm_sub_rect"), sub));

            let mut pick: Option<String> = None;
            let mut back = false;
            let groups: Vec<(String, String, Color32)> = state
                .groups
                .iter()
                .map(|g| (g.id.clone(), g.name.clone(), g.egui_color()))
                .collect();
            egui::Area::new(egui::Id::new("cm-submenu"))
                .order(egui::Order::Foreground)
                .fixed_pos(sub.min)
                .show(ctx, |ui| {
                    // The Area's clip rect starts at cursor size; painting the
                    // full panel (borders included) needs it allocated first.
                    ui.set_min_size(vec2(w, h));
                    ui.painter()
                        .rect_filled(sub, CornerRadius::same(10), p.bg_card);
                    ui.painter().rect_stroke(
                        sub,
                        CornerRadius::same(10),
                        egui::Stroke::new(1.0, p.border),
                        egui::StrokeKind::Inside,
                    );
                    // `.submenu-back` (arrow_back 18px) + title, gap 8.
                    let bk = ui.interact(
                        Rect::from_min_size(pos2(sub.min.x + 12.0, sub.min.y + 9.0), vec2(18.0, 18.0)),
                        egui::Id::new("cm-sub-back"),
                        Sense::click(),
                    );
                    mi::paint_centered(
                        ui.painter(),
                        bk.rect.center(),
                        "arrow_back",
                        18.0,
                        if bk.hovered() { p.text } else { p.text_dim },
                    );
                    if bk.clicked() {
                        back = true;
                    }
                    ui.painter().text(
                        pos2(sub.min.x + 12.0 + 18.0 + 8.0, sub.min.y + 18.0),
                        Align2::LEFT_CENTER,
                        state.t("Add to Group"),
                        title_font.clone(),
                        p.text,
                    );
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            pos2(sub.min.x, sub.min.y + 36.0),
                            vec2(sub.width(), 1.0),
                        ),
                        CornerRadius::same(0),
                        p.border,
                    );
                    let mut yy = sub.min.y + 37.0;
                    for (i, (id, name, color)) in groups.iter().enumerate() {
                        let row = Rect::from_min_size(pos2(sub.min.x, yy), vec2(sub.width(), ITEM_H));
                        let rr = ui.interact(
                            row,
                            egui::Id::new("cm-sub-item").with(i),
                            Sense::click(),
                        );
                        if rr.hovered() {
                            ui.painter()
                                .rect_filled(row, CornerRadius::same(0), p.hover_veil);
                        }
                        mi::paint_centered(
                            ui.painter(),
                            pos2(row.min.x + 12.0 + 11.0, row.center().y),
                            "folder",
                            18.0,
                            *color,
                        );
                        ui.painter().text(
                            pos2(row.min.x + 12.0 + 22.0 + 10.0, row.center().y),
                            Align2::LEFT_CENTER,
                            ellipsize(ui, name, &item_font, sub.width() - 56.0),
                            item_font.clone(),
                            p.text,
                        );
                        if rr.clicked() {
                            pick = Some(id.clone());
                        }
                        yy += ITEM_H;
                    }
                });

            if back {
                state.cm_group_menu = false;
            }
            if let Some(group_id) = pick {
                let limit = state
                    .group_by_id(&group_id)
                    .map(|g| g.speed_limit)
                    .unwrap_or(0);
                let name = state
                    .group_by_id(&group_id)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                state.group_add_member(&group_id, &gid);
                state.toast(true, format!("Added to: \"{name}\""));
                let gid = gid.clone();
                cctx.run(move |_| async move {
                    let _ = moonhunt_core::aria2::apply_download_speed_limit(
                        gid,
                        limit.saturating_mul(1024),
                    )
                    .await;
                    Ok::<String, String>(String::new())
                });
                close = true;
            }
        }
    }

    if act.is_none() && !close && ctx.input(|i| i.key_pressed(Key::Enter)) {
        act = Some("details");
        close = true;
    }

    if let Some(action) = act {
        match action {
            "copyUrl" => {
                ctx.copy_text(vm.url.clone());
                state.toast(true, "URL copied");
            }
            "copyFilename" => {
                ctx.copy_text(vm.filename.clone());
                state.toast(true, "Filename copied");
            }
            "verifyHash" => {
                state.hash_open = true;
                state.hash_path = vm.path.clone();
                state.hash_name = vm.filename.clone();
                state.hash_expected.clear();
                state.hash_computed.clear();
                state.hash_error.clear();
                state.hash_busy = false;
            }
            other => downloads::run_action_for_gid(state, cctx, &gid, other),
        }
    }

    if close {
        state.cm_open = false;
        state.cm_group_menu = false;
    }
}

// ── `batchExpand.ts` port ───────────────────────────────────────────────

/// `urlText.split('\n').flatMap(line => expandTemplate(line.trim()))`.
fn expanded_urls(text: &str) -> Vec<String> {
    text.split('\n')
        .flat_map(|line| expand_template(line.trim()))
        .collect()
}

/// JS `parseInt(s, 10)`: leading whitespace, optional sign, then digits.
fn parse_int_js(s: &str) -> Option<i64> {
    let t = s.trim_start();
    let mut chars = t.chars().peekable();
    let mut sign = 1i64;
    match chars.peek() {
        Some('-') => {
            sign = -1;
            chars.next();
        }
        Some('+') => {
            chars.next();
        }
        _ => {}
    }
    let mut digits = String::new();
    while let Some(c) = chars.peek() {
        if c.is_ascii_digit() {
            digits.push(*c);
            chars.next();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse::<i64>().ok().map(|n| sign * n)
}

/// JS `String(i).padStart(pad, '0')`.
fn pad_start(i: i64, pad: usize) -> String {
    let mut s = i.to_string();
    while s.len() < pad {
        s.insert(0, '0');
    }
    s
}

fn expand_range(spec: &str) -> Vec<String> {
    let parts: Vec<&str> = spec.split('-').collect();
    if parts.len() != 2 {
        return vec![format!("[{spec}]")];
    }
    let (Some(start), Some(end)) = (parse_int_js(parts[0]), parse_int_js(parts[1])) else {
        return vec![format!("[{spec}]")];
    };
    let pad = parts[0].chars().count();
    let mut out = Vec::new();
    let mut i = start;
    while i <= end {
        out.push(pad_start(i, pad));
        i += 1;
    }
    out
}

fn cartesian(segments: &[Vec<String>]) -> Vec<String> {
    let Some((first, rest)) = segments.split_first() else {
        return vec![String::new()];
    };
    let rest_product = cartesian(rest);
    let mut out = Vec::with_capacity(first.len() * rest_product.len());
    for f in first {
        for r in &rest_product {
            out.push(format!("{f}{r}"));
        }
    }
    out
}

/// Expand `[start-end]` and `{a,b,c}` patterns, matching `batchExpand.ts`.
fn expand_template(template: &str) -> Vec<String> {
    // `template.split(/(\[[^\]]+\]|\{[^}]+\})/)`
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let close = match c {
            '[' => ']',
            '{' => '}',
            _ => {
                cur.push(c);
                i += 1;
                continue;
            }
        };
        // Find the next `close`; the token body must be non-empty.
        let mut j = i + 1;
        while j < chars.len() && chars[j] != close {
            j += 1;
        }
        if j < chars.len() && j > i + 1 {
            tokens.push(std::mem::take(&mut cur));
            tokens.push(chars[i..=j].iter().collect());
            i = j + 1;
        } else {
            cur.push(c);
            i += 1;
        }
    }
    tokens.push(cur);

    if tokens.len() == 1 {
        let t = template.trim();
        return if t.is_empty() {
            Vec::new()
        } else {
            vec![t.to_owned()]
        };
    }

    let mut segments: Vec<Vec<String>> = Vec::new();
    for token in &tokens {
        let tc: Vec<char> = token.chars().collect();
        if tc.len() >= 3 && tc[0] == '[' && tc[tc.len() - 1] == ']' {
            let spec: String = tc[1..tc.len() - 1].iter().collect();
            segments.push(expand_range(&spec));
            continue;
        }
        if tc.len() >= 3 && tc[0] == '{' && tc[tc.len() - 1] == '}' {
            let body: String = tc[1..tc.len() - 1].iter().collect();
            segments.push(
                body.split(',')
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned)
                    .collect(),
            );
            continue;
        }
        if token.is_empty() {
            segments.push(Vec::new());
        } else {
            segments.push(vec![token.clone()]);
        }
    }
    cartesian(&segments)
}

#[cfg(test)]
mod tests {
    use super::expand_template;

    #[test]
    fn plain_url_is_trimmed() {
        assert_eq!(
            expand_template("  https://a.example/file.zip  "),
            vec!["https://a.example/file.zip"]
        );
    }

    #[test]
    fn empty_and_blank_expand_to_nothing() {
        assert!(expand_template("").is_empty());
        assert!(expand_template("   ").is_empty());
    }

    #[test]
    fn numeric_range_zero_pads_to_start_length() {
        assert_eq!(
            expand_template("f[002-004].zip"),
            vec!["f002.zip", "f003.zip", "f004.zip"]
        );
    }

    #[test]
    fn list_expands_and_filters() {
        assert_eq!(
            expand_template("video.{mp4, webm ,,mov}.x"),
            vec!["video.mp4.x", "video.webm.x", "video.mov.x"]
        );
    }

    #[test]
    fn range_and_list_cartesian_product() {
        assert_eq!(
            expand_template("f[1-2].{a,b}.x"),
            vec!["f1.a.x", "f1.b.x", "f2.a.x", "f2.b.x"]
        );
    }

    #[test]
    fn invalid_ranges_stay_literal() {
        assert_eq!(expand_template("f[x-y].zip"), vec!["f[x-y].zip"]);
        assert_eq!(expand_template("f[1-2-3].zip"), vec!["f[1-2-3].zip"]);
        assert_eq!(expand_template("f[].zip"), vec!["f[].zip"]);
        assert_eq!(expand_template("x{}y"), vec!["x{}y"]);
    }

    #[test]
    fn range_uses_js_parse_int() {
        // parseInt("01a") === 1, padStart length is parts[0].length (3).
        assert_eq!(
            expand_template("f[01a-03].z"),
            vec!["f001.z", "f002.z", "f003.z"]
        );
    }

    #[test]
    fn edge_tokens_match_vue_split_quirk() {
        // `String.split` keeps the empty segments around an edge token, and the
        // old cartesian product then returns [] — preserve that exactly.
        assert!(expand_template("[1-2]x").is_empty());
        assert!(expand_template("f[1-2]").is_empty());
        assert!(expand_template("[1-3]").is_empty());
        assert!(expand_template("{a,b}").is_empty());
    }

    #[test]
    fn reversed_range_is_empty() {
        assert!(expand_template("f[5-1]").is_empty());
    }
}
