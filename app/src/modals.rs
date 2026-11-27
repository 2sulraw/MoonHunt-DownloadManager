//! Overlay modals (ports of the Vue `*Modal.vue` layer): About dialog plus the
//! new-download dialog rework; update/confirm/category/hash/schedule dialogs
//! follow in their milestones.

use crate::screens::Ctx;
use crate::state::{
    formatted_category_folder, packaged_category_folder, resolve_category_dir,
    sanitize_category_folder, with_duplicate_suffix, DownloadVm, NdTorrentFile, Status, UiState,
};
use egui_taffy::taffy;
// `tui.style(..)` / `.add(..)` / `.ui(..)` builders on `&mut Tui`.
use egui_taffy::TuiBuilderLogic;
use crate::ui::icons as mi;
use crate::ui::widgets as mk;
use crate::ui::tokens::{self, Palette};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Margin, Pos2, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui,
};
use std::time::{Duration, Instant};

pub fn show(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    // `MainLayout.checkDueSchedules` — executes due rows every 10 seconds.
    nd_due_tick(state, cctx);
    // Global `.torrent` drop (old `torrentDropHandler` / `tauri://drag-drop`).
    nd_drop_tick(state, ctx, cctx);
    // Download-limit side effects (pause-all owed by the tracker + the 10s
    // `persistUsedBytes` interval of `useDownloadLimit`).
    limit_tick(state, cctx);
    // `MainLayout.checkPendingDownloads` — browser-extension queue → dialog.
    crate::extension_pending::tick(state, ctx, cctx);
    // `useDownloads.scheduleFileStateCheck` — completed-row file badges.
    crate::file_state::tick(state, ctx, cctx);

    // `watch(showAddDialog)` open work — runs exactly once per open: clipboard
    // prefill, saved categories, proxy-badge reset and `allowDuplicate = false`.
    if state.show_new_download && !state.nd_was_open {
        nd_on_open(state);
    }
    state.nd_was_open = state.show_new_download;

    if state.show_new_download {
        new_download_modal(state, ctx, cctx);
    }
    // `watch(showDetailsModal)` open work — runs exactly once per open: the
    // tab resets to General, prior fetch results clear, and the three detail
    // fetches fire (`get_download_details` + `get_download_by_gid`, plus
    // `get_yt_download_health` for `yt-dlp-direct-*` gids).
    if state.dm_open && !state.dm_was_open {
        dm_on_open(state, cctx);
    }
    state.dm_was_open = state.dm_open;
    if state.dm_open {
        details_modal(state, ctx, cctx);
    }
    // Sub-dialogs stack above the new-download modal (created later ⇒ higher
    // area index ⇒ painted/clicked first). Order mirrors the old z-indexes:
    // `.sm-overlay` 10000 below `.cm-overlay` 10010.
    if state.nd_sched_open {
        nd_schedule_modal(state, ctx);
    }
    if state.nd_catmgr_open {
        nd_category_manager(state, ctx, cctx);
    }
    // Group dialogs (`showCreateGroupDialog` / `renameGroupId` / the group
    // Downloads sub-modal) open from the Downloads page and stack in that
    // order, matching the old page's z-indexes.
    crate::group_dialogs::show(state, ctx, cctx);
    if state.show_about && !state.show_new_download {
        crate::overlays::about_modal(state, ctx, cctx);
    }
    // `UpdateModal.vue` — opened by the titlebar update badge or the About
    // dialog's check (`onUpdateAvailable`).
    if state.show_update_modal {
        crate::overlays::update_modal(state, ctx, cctx);
    }
    // `CloseDialog.vue` — modal-fade overlay, 440px card.
    if state.show_close_dialog {
        crate::overlays::close_dialog(state, ctx);
    }
    // `.dla-overlay` carries `z-index: 9999` — drawn last so the blocking
    // limit alert stacks above every other dialog.
    if state.dla_open {
        download_limit_alert(state, ctx);
    }
    // Batch add / error details / hash verify / card context menu — drawn last
    // so they stack above every other dialog.
    crate::download_dialogs::show(state, ctx, cctx);

    // `ClipboardToast` — `z-index: 9999`, fixed 100px above the bottom edge.
    crate::overlays::clipboard_toasts(state, ctx, cctx);
}

/// The old dialog's `watch(showAddDialog, open => …)` body.
fn nd_on_open(state: &mut UiState) {
    state.nd_allow_duplicate = false;
    // A hand-typed / toolbar-opened dialog carries no extension headers. Keep
    // the ones a queued extension download just staged: `extension_pending`
    // opens this dialog (setting `nd_pending` + `nd_headers`) right before
    // this open-edge runs, so clearing unconditionally dropped its cookies/UA
    // and the download then failed at the origin.
    if !state.nd_pending {
        state.nd_headers = None;
    }
    state.nd_cat_error.clear();
    state.nd_sched_error.clear();
    state.nd_proxy_msg.clear();
    state.nd_proxy_detecting = false;
    state.nd_dd_open = None;
    // `loadDownloadCategories()` (backend + normalize/sanitize pass)
    state.nd_categories = nd_load_categories();
    // Proxy badge: re-check on every open (`check_aria2_proxy` on mount).
    state.nd_pcheck_done = "\u{0}".to_owned();
    state.nd_pcheck_at = Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap_or_else(Instant::now);
    state.nd_pcheck_text = None;
    state.nd_pcheck_busy = false;
    state.nd_pcheck_direct = false;
    // Clipboard prefill: only when the URL field is empty.
    if state.add_url.trim().is_empty() {
        state.nd_pasted = false;
        if let Ok(text) = moonhunt_core::clipboard::read_clipboard_text() {
            if let Some(url) = moonhunt_core::clipboard::detect_and_extract_url(text.trim()) {
                state.add_url = url;
                state.nd_pasted = true;
            }
        }
    } else {
        state.nd_pasted = false;
    }
    state.nd_autofill_filename();
    // The old dialog's `autofocus` on the URL input.
    state.nd_focus_next = true;
    state.nd_focus_frames = 3;
}

// ─── New-download modal (rework of the old `DownloadsPage` add dialog) ──────

/// Row metrics of the modal body flow: 60px header + divider, a 40px tab
/// strip, four 18/6/36px sections and a 52px footer.
const ND_FONT: f32 = 12.0;
const ND_W: f32 = 480.0;
const ND_HEADER: f32 = 60.0;
const ND_FOOTER: f32 = 52.0;
/// `.nd-body` side padding.
const ND_PAD: f32 = 16.0;
/// Schedule sub-modal panel metrics (`.sm-panel` 380px).
const SM_W: f32 = 380.0;
const SM_PAD: f32 = 14.0;

/// Allocate a full-width flow row and return it inset by `pad`, measured
/// against the *card* width rather than the available width — the body
/// ScrollArea's solid 4px bar shrinks the content rect, and the visual
/// padding must not follow it inward.
fn row_pad(ui: &mut Ui, h: f32, card_w: f32, pad: f32) -> egui::Rect {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    egui::Rect::from_min_size(pos2(r.min.x + pad, r.min.y), vec2(card_w - 2.0 * pad, h))
}

/// Body row of the new-download dialog (16px padding).
fn nd_row(ui: &mut Ui, h: f32) -> egui::Rect {
    row_pad(ui, h, ND_W, ND_PAD)
}

/// Solid 4px scroll bar for the dialog's scroll areas (non-floating, so the
/// bar allocates space and shrinks the content rect — `row_pad` compensates
/// by measuring against the card width).
fn nd_scroll_style(ui: &mut Ui) {
    let s = &mut ui.spacing_mut().scroll;
    s.bar_width = 4.0;
    s.floating = false;
}

/// Source tab of the dialog (the old `addDialogTab`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NdTab {
    Link,
    Torrent,
}

/// Section label row (`.nd-section-title` 10px uppercase) + optional right-end
/// pill badge (`.nd-source-badge`), both painted inside an allocated 18px row.
fn nd_section_row(ui: &mut Ui, p: &Palette, title: &str, badge: Option<&str>) {
    // 16px side padding (the old `.nd-body`): `nd_row` insets against the
    // card width, so rows line up with the divider and footer.
    let rect = nd_row(ui, 18.0);
    let painter = ui.painter();
    painter.text(
        pos2(rect.min.x, rect.center().y),
        Align2::LEFT_CENTER,
        title.to_uppercase(),
        FontId::new(10.0, tokens::typography::semibold_family()),
        p.text_dim,
    );
    if let Some(badge) = badge {
        let font = FontId::new(9.0, tokens::typography::semibold_family());
        let g = painter.layout_no_wrap(badge.to_owned(), font.clone(), Color32::WHITE);
        let w = g.rect.width() + 12.0;
        let pill = egui::Rect::from_min_size(
            pos2(rect.max.x - w, rect.min.y + 2.0),
            vec2(w, 14.0),
        );
        painter.rect_filled(
            pill,
            CornerRadius::same(7),
            mk::blend(p.bg_card, p.accent, 0.08),
        );
        painter.rect_stroke(
            pill,
            CornerRadius::same(7),
            egui::Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.16)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            badge,
            font,
            p.accent_strong,
        );
    }
}

/// Themed text field (rework of `.nd-input`): 36px box, faint white inset fill,
/// MoonKit border — accent stroke while focused; a `.nd-input-clear` cross
/// button at the right edge appears for non-empty values; the Save Location
/// field instead hosts the `.nd-input-action` folder button.
fn nd_input(
    ui: &mut Ui,
    p: &Palette,
    id: egui::Id,
    value: &mut String,
    hint: &str,
    right: Option<&'static str>,
    request_focus: bool,
) {
    let rect = nd_row(ui, 36.0);
    nd_input_at(ui, p, id, rect, value, hint, right, false, request_focus);
    // The TextEdit scope inside `nd_input_at` moves the cursor back to the
    // text's own (shorter) rect; advance to the box bottom so the next
    // section keeps the CSS rhythm (12px body gap + 18px title row).
    ui.advance_cursor_after_rect(rect);
}

/// The same field painted into an explicit rect (used inside the option
/// cards, which lay out at absolute positions).
fn nd_input_at(
    ui: &mut Ui,
    p: &Palette,
    id: egui::Id,
    rect: egui::Rect,
    value: &mut String,
    hint: &str,
    right: Option<&'static str>,
    secret: bool,
    request_focus: bool,
) {
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
    // `.nd-input:hover` / `.nd-input:focus`.
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
        mk::blend(p.border, Color32::WHITE, 0.4)
    } else {
        p.border
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    // Vertically center the single text row inside the 36px box. The box is
    // painted above; here we only place the editable row so its measured line
    // height sits centered. Horizontal inset (`padding: 0 10px`) comes from the
    // frame's inner_margin — egui 0.37 ignores `.margin()` whenever `.frame()`
    // is set (it replaces the whole style frame), so the padding must live on
    // the frame itself.
    let font = FontId::proportional(ND_FONT);
    let line_h =
        ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
    let text_rect =
        egui::Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
    // The Save Location field reserves room for its browse button on the right
    // (`.nd-path-input { padding-right: 36px }`); the clear-button fields keep
    // the plain 10px right inset.
    let pad_r: i8 = if right == Some("folder_open") { 36 } else { 10 };
    // Derived scope id: sharing the TextEdit's own id made the focus
    // dead-man's switch drop it at end_pass.
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(text_rect).id(id.with("nd-field")),
        |ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(value)
                // Explicit id so `has_focus(id)` above and the clear-button
                // `interact` below talk about this exact widget.
                .id(id)
                .desired_width(text_rect.width())
                .font(font)
                // egui defaults to `visuals.text_color()` = the accent
                // fg_stroke (our scrollbar/thumb color); the old inputs
                // use plain `color: var(--text)`.
                .text_color(p.text)
                .hint_text(RichText::new(hint).size(ND_FONT).color(p.text_dim))
                .password(secret)
                .frame(egui::Frame::NONE.inner_margin(Margin {
                    left: 10,
                    right: pad_r,
                    top: 0,
                    bottom: 0,
                })),
        );
        // The old `<input autofocus>`: focus the widget we just drew, so the
        // id never has to be guessed before it exists.
        if request_focus {
            resp.request_focus();
        }
    });
    // Right edge affordance: clear (URL/filename/magnet) or the save-path
    // folder button. `interact` doesn't advance the layout cursor.
    if let Some(icon) = right {
        let is_folder = icon == "folder_open";
        // `.nd-input-clear` (24px @ right:8) / `.nd-input-action`
        // (26px @ right:10). The clear affordance only exists while the
        // wrap is hovered or the field is focused (the old
        // `opacity: 0; pointer-events: none`); the folder button is always on.
        let size = if is_folder { 26.0 } else { 24.0 };
        let c = egui::Rect::from_center_size(
            pos2(
                rect.max.x - if is_folder { 23.0 } else { 20.0 },
                rect.center().y,
            ),
            vec2(size, size),
        );
        let visible = is_folder || (!value.is_empty() && (hovered || focused));
        if visible {
            let rr = ui.interact(c, id.with(icon), Sense::click());
            if rr.hovered() {
                ui.painter().rect_filled(
                    c,
                    CornerRadius::same(7),
                    mk::blend(p.bg_card, p.accent, if is_folder { 0.10 } else { 0.15 }),
                );
            }
            mi::paint_centered(
                ui.painter(),
                c.center(),
                icon,
                if is_folder { 18.0 } else { 13.0 },
                if rr.hovered() {
                    if is_folder { p.accent_strong } else { p.text }
                } else {
                    p.text_muted
                },
            );
            if rr.clicked() {
                if is_folder {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        *value = dir.display().to_string();
                    }
                } else {
                    value.clear();
                }
            }
        }
    }
}

/// One footer button (`.nd-btn`): full half width, 38px, 10r. Optional leading
/// material icon (the old button is `icon + label` with a 6px gap).
pub(crate) fn nd_button(
    ui: &mut Ui,
    p: &Palette,
    id_name: &str,
    rect: egui::Rect,
    label: &str,
    icon: Option<&str>,
    primary: bool,
    enabled: bool,
) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id_name).with("btn"), Sense::click());
    let painter = ui.painter();
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new(id_name).with("t"),
        if rr.hovered() && enabled { 1.0 } else { 0.0 },
        0.08,
    );
    let mut clicked = false;
    let font = FontId::new(12.5, tokens::typography::bold_family());
    let label_w = painter.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE).rect.width();
    let icon_w = if icon.is_some() { 22.0 } else { 0.0 };
    let mut tx = rect.center().x - (label_w + icon_w) * 0.5;
    let (fg, glyph) = if primary {
        // `.nd-btn-primary`: `linear-gradient(135deg, accent, accent@85%)`
        // plus `0 2px 6px rgba(accent,.2)` and a 1px top highlight. CSS
        // `opacity: .5` fades fill and shadow together, so the disabled fill
        // is a 50%-alpha gradient over the card (not a pre-blended solid).
        let (c0, c1) = (p.accent, mk::blend(p.bg_card, p.accent, 0.85));
        crate::ui::shade::soft_shadow(
            ui,
            rect,
            10,
            p.accent,
            2.0,
            6.0,
            // The ring stack accumulates to ~2x the CSS alpha.
            if enabled { 26 } else { 13 },
        );
        painter.rect_filled(rect, CornerRadius::same(10), p.bg_card);
        let a = if enabled { 255 } else { 128 };
        let fade = |c: Color32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
        crate::ui::shade::grad135(ui, rect, 10, fade(c0), fade(c1));
        painter.line_segment(
            [
                pos2(rect.min.x + 10.0, rect.min.y + 1.0),
                pos2(rect.max.x - 10.0, rect.min.y + 1.0),
            ],
            egui::Stroke::new(1.0, Color32::from_white_alpha(if enabled { 38 } else { 19 })),
        );
        let dark = Color32::from_rgb(0x0a, 0x0e, 0x1a);
        let c = if enabled { dark } else { mk::blend(p.bg_card, dark, 0.5) };
        (c, c)
    } else {
        let fill = if enabled {
            mk::blend(
                mk::blend(p.bg_card, Color32::WHITE, 0.03),
                mk::blend(p.bg_card, p.accent, 0.08),
                t,
            )
        } else {
            // CSS `opacity: .5` fades the fill and border with the label.
            mk::blend(mk::blend(p.bg_card, Color32::WHITE, 0.03), p.bg_card, 0.5)
        };
        painter.rect_filled(rect, CornerRadius::same(10), fill);
        let stroke = mk::blend(p.border, mk::blend(p.bg_card, p.accent, 0.2), t);
        painter.rect_stroke(
            rect,
            CornerRadius::same(10),
            egui::Stroke::new(
                1.0,
                if enabled { stroke } else { mk::blend(stroke, p.bg_card, 0.5) },
            ),
            egui::StrokeKind::Inside,
        );
        let mut c = mk::blend(p.text_muted, p.accent_strong, t);
        if !enabled {
            c = mk::blend(p.bg_card, c, 0.5);
        }
        (c, c)
    };
    if let Some(icon) = icon {
        mi::paint_centered(
            painter,
            pos2(tx + 8.0, rect.center().y),
            icon,
            16.0,
            glyph,
        );
        tx += icon_w;
    }
    painter.text(
        pos2(tx + label_w * 0.5, rect.center().y),
        Align2::CENTER_CENTER,
        label,
        font,
        fg,
    );
    if rr.clicked() && enabled {
        clicked = true;
    }
    clicked
}

/// Galley width of `label` at `size` (for centering icon+text groups).
fn text_w(ui: &Ui, label: &str, size: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(
            label.to_owned(),
            FontId::new(size, egui::FontFamily::default()),
            Color32::WHITE,
        )
        .rect
        .width()
}

/// 42×24 MoonKit switch at an absolute rect (`.sp-toggle`/`.pf-toggle`/
///`.cp-toggle` share one metric; `mk::switch` only lays out in flow).
fn nd_switch(ui: &mut Ui, p: &Palette, rect: egui::Rect, on: bool, id: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new(id).with("t"),
        if on { 1.0 } else { 0.0 },
        0.12,
    );
    let bg = mk::blend(p.bg_card_inset, p.accent.gamma_multiply(0.25), t);
    ui.painter().rect_filled(rect, CornerRadius::same(12), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        egui::Stroke::new(1.0, mk::blend(p.border_strong, p.accent.gamma_multiply(0.4), t)),
        egui::StrokeKind::Inside,
    );
    let x = rect.min.x + 11.0 + 18.0 * t;
    ui.painter().circle_filled(
        pos2(x, rect.center().y),
        9.0,
        mk::blend(p.text_dim, p.accent, t),
    );
    rr.clicked()
}

/// Accent text button with a leading icon (`.sp-configure` / `.cp-manage`).
fn nd_link_button(
    ui: &mut Ui,
    p: &Palette,
    rect: egui::Rect,
    icon: &str,
    label: &str,
    id: &str,
) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    if rr.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent.gamma_multiply(0.10));
    }
    let font = FontId::new(11.5, tokens::typography::semibold_family());
    let lw = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width();
    let total = 18.0 + lw;
    let x = rect.center().x - total * 0.5;
    mi::paint_centered(ui.painter(), pos2(x + 7.0, rect.center().y), icon, 14.0, p.accent_strong);
    ui.painter().text(
        pos2(x + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        p.accent_strong,
    );
    rr.clicked()
}

/// N-way segmented control inside `rect` (CategoryPicker's mode dropdown and
/// the ScheduleModal action/repeat rows, as segmented controls).
fn nd_seg(
    ui: &mut Ui,
    p: &Palette,
    rect: egui::Rect,
    id: &str,
    options: &[&str],
    sel: usize,
) -> Option<usize> {
    let n = options.len().max(1);
    let sel = sel.min(n - 1);
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), p.hover_veil.gamma_multiply(0.5));
    let inner = rect.shrink2(vec2(3.0, 3.0));
    let cell = inner.width() / n as f32;
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new(id).with("t"),
        sel as f32,
        0.12,
    );
    let ind = egui::Rect::from_min_size(
        pos2(inner.min.x + t * cell, inner.min.y),
        vec2(cell, inner.height()),
    );
    ui.painter().rect_filled(ind, CornerRadius::same(6), p.bg_card);
    ui.painter().rect_stroke(
        ind,
        CornerRadius::same(6),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
    for (i, opt) in options.iter().enumerate() {
        let col = if i == sel {
            p.accent_strong
        } else if rr.hovered() {
            p.text_muted
        } else {
            p.text_dim
        };
        ui.painter().text(
            pos2(inner.min.x + (i as f32 + 0.5) * cell, rect.center().y),
            Align2::CENTER_CENTER,
            *opt,
            FontId::new(11.5, tokens::typography::semibold_family()),
            col,
        );
    }
    if rr.clicked() {
        if let Some(px) = rr.interact_pointer_pos() {
            let idx = (((px.x - inner.min.x) / cell).floor() as usize).min(n - 1);
            if idx != sel {
                return Some(idx);
            }
        }
    }
    None
}

/// Option card (`.sp-schedule` / `.cp-category` / `.pf-card`): 12r bordered
/// box with an icon+label header row; the caller paints the right-hand
/// controls and the expanded body inside the returned rect.
fn nd_card(
    ui: &mut Ui,
    p: &Palette,
    icon: &str,
    label: &str,
    enabled: bool,
    body_h: f32,
) -> egui::Rect {
    let r = nd_row(ui, 44.0 + body_h);
    let (bg, border) = if enabled {
        (
            mk::blend(p.bg_card, p.accent, 0.04),
            mk::blend(p.border, p.accent, 0.18),
        )
    } else {
        (mk::blend(p.bg_card, Color32::WHITE, 0.02), p.border)
    };
    ui.painter().rect_filled(r, CornerRadius::same(12), bg);
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(12),
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let iy = r.min.y + 22.0;
    mi::paint_centered(
        ui.painter(),
        pos2(r.min.x + 23.0, iy),
        icon,
        18.0,
        if enabled { p.accent_strong } else { p.text_dim },
    );
    ui.painter().text(
        pos2(r.min.x + 40.0, iy),
        Align2::LEFT_CENTER,
        label,
        FontId::new(13.0, egui::FontFamily::default()),
        if enabled { p.text } else { p.text_muted },
    );
    r
}

/// `.nd-label` — field label row with a muted icon and an "(optional)" tail.
fn nd_field_label(ui: &mut Ui, p: &Palette, icon: &str, label: &str, optional: Option<&str>) {
    let r = nd_row(ui, 18.0);
    mi::paint_centered(
        ui.painter(),
        pos2(r.min.x + 7.0, r.center().y),
        icon,
        14.0,
        p.text_muted.gamma_multiply(0.7),
    );
    let mut x = r.min.x + 18.0;
    ui.painter().text(
        pos2(x, r.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(12.0, egui::FontFamily::default()),
        p.text_muted,
    );
    x += text_w(ui, label, 12.0) + 5.0;
    if let Some(opt) = optional {
        ui.painter().text(
            pos2(x, r.center().y),
            Align2::LEFT_CENTER,
            opt,
            FontId::new(11.0, egui::FontFamily::default()),
            p.text_dim,
        );
    }
}

/// `.nd-torrent-hint` / `.nd-field-hint`: accent info icon + 11px dim text.
fn nd_hint(ui: &mut Ui, p: &Palette, text: &str) {
    let r = nd_row(ui, 16.0);
    mi::paint_centered(ui.painter(), pos2(r.min.x + 7.0, r.center().y), "info_outline", 14.0, p.accent_strong);
    ui.painter().text(
        pos2(r.min.x + 18.0, r.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(11.0, egui::FontFamily::default()),
        p.text_dim,
    );
}

/// The full new-download dialog. Layout mirrors the old modal's flow: header
/// with icon tile/title/proxy badge/close, divider, URL|Torrent tab strip, the
/// body sections (source, duplicate warning, save-as, save-location,
/// connections plus the category/schedule/proxy option cards) and the footer
/// buttons — restyled against the current MoonKit tokens.
fn new_download_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let tab = if state.nd_torrent_tab {
        NdTab::Torrent
    } else {
        NdTab::Link
    };
    let adding = state.nd_is_adding;
    // Footer disable rules (`.nd-btn:disabled`): URL needs a URL, the torrent
    // tab needs at least one .torrent file or a magnet.
    let can_submit = !adding
        && match tab {
            NdTab::Link => !state.add_url.trim().is_empty(),
            NdTab::Torrent => !state.nd_torrents.is_empty() || !state.nd_magnet.trim().is_empty(),
        };
    let focus_req = state.nd_focus_next;
    // `.nd-source-badge` hides once a filename was detected from the URL, and
    // `detectedFilename` drives the Save As autofill (`watch(newUrl)`).
    let detected_filename = state.nd_detected_filename();
    let host = if detected_filename.is_empty() {
        state.nd_detected_host()
    } else {
        None
    };
    let dup_gid = state.nd_duplicate_gid();
    // `matchedAddon` — an enabled addon claiming this URL via `url_patterns`.
    let addon_hint = state
        .addon
        .match_url(&state.add_url)
        .map(|a| (a.id.clone(), a.name.clone()));
    // Sub-dialogs swallow this frame's Enter/Esc (they handle their own).
    let sub_open = state.nd_sched_open || state.nd_catmgr_open;

    // ProxyUrlCheckBadge: debounced re-check whenever the URL changes
    // (`watch(() => props.url)` → `scheduleCheck`).
    let badge_url = if matches!(tab, NdTab::Link) {
        state.add_url.trim().to_owned()
    } else {
        String::new()
    };
    nd_proxy_check_tick(state, cctx, &badge_url);
    let badge = NdProxyBadge {
        busy: state.nd_pcheck_busy,
        direct: state.nd_pcheck_direct,
        ok: state.nd_pcheck_ok,
        text: state.nd_pcheck_text.clone(),
    };

    let url_before = state.add_url.clone();

    let mut want_close = false;
    let mut want_submit = false;
    let mut want_show_dup = false;
    let mut want_open_sched = false;
    let mut want_open_catmgr = false;
    let mut want_pick_torrents = false;
    let mut want_open_addon = false;
    let mut remove_torrent: Option<usize> = None;

    // Dim overlay + click-outside close (ported `useOverlayClose`).
    {
        let open = &mut state.show_new_download;
        egui::Area::new(egui::Id::new("nd-overlay"))
            .order(egui::Order::Middle)
            .anchor(Align2::LEFT_TOP, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(140));
                if ui.allocate_rect(screen, Sense::click()).clicked() {
                    *open = false;
                }
            });
    }

    egui::Area::new(egui::Id::new("nd-modal"))
        .order(egui::Order::Foreground)
        // Dead-center of the window: the old `.nd-overlay` is
        // `position:fixed; inset:0` + flex centering in the full
        // viewport (the title bar is part of the page there too).
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    // No fixed height: the footer is the last row, so the
                    // Frame hugs the flow exactly (the old modal also grew
                    // and shrank with its contents).
                    ui.set_width(ND_W);

                    // ── Header ──
                    let (h_rect, _) =
                        ui.allocate_exact_size(vec2(ND_W, ND_HEADER), Sense::hover());
                    let painter = ui.painter();
                    // `.nd-avatar`: accent-tinted tile.
                    let avatar = egui::Rect::from_min_size(
                        pos2(h_rect.min.x + 16.0, h_rect.min.y + 12.0),
                        vec2(36.0, 36.0),
                    );
                    painter.rect_filled(
                        avatar,
                        CornerRadius::same(10),
                        mk::blend(p.bg_card, p.accent, 0.10),
                    );
                    painter.rect_stroke(
                        avatar,
                        CornerRadius::same(10),
                        egui::Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.18)),
                        egui::StrokeKind::Inside,
                    );
                    mi::paint_centered(
                        painter,
                        avatar.center(),
                        "cloud_download",
                        18.0,
                        p.accent_strong,
                    );
                    // `.nd-title` + `.nd-subtitle`
                    painter.text(
                        pos2(avatar.max.x + 10.0, h_rect.min.y + 16.0),
                        Align2::LEFT_TOP,
                        state.t("New Download"),
                        FontId::new(14.0, tokens::typography::semibold_family()),
                        p.text,
                    );
                    painter.text(
                        pos2(avatar.max.x + 10.0, h_rect.min.y + 36.0),
                        Align2::LEFT_TOP,
                        state.t("Paste a link and start downloading"),
                        FontId::new(11.0, egui::FontFamily::default()),
                        p.text_muted,
                    );
                    // `.nd-close`
                    let close = egui::Rect::from_center_size(
                        pos2(
                            h_rect.max.x - 16.0 - 14.0,
                            h_rect.min.y + 12.0 + 18.0,
                        ),
                        vec2(28.0, 28.0),
                    );
                    // ProxyUrlCheckBadge sits in `.nd-header-actions`
                    // between the title and the close button.
                    nd_proxy_badge(ui, &p, h_rect, close.min.x - 8.0, &badge, &state.settings.language);
                    let crr = ui.interact(
                        close,
                        egui::Id::new("nd-close"),
                        Sense::click(),
                    );
                    if crr.hovered() {
                        painter.rect_filled(
                            close,
                            CornerRadius::same(7),
                            mk::blend(p.bg_card, p.status_error, 0.10),
                        );
                    }
                    mi::paint_centered(
                        painter,
                        close.center(),
                        "close",
                        16.0,
                        if crr.hovered() { p.status_error } else { p.text_dim },
                    );
                    if crr.clicked() {
                        want_close = true;
                    }
                    crr.on_hover_text(state.t("Close (Esc)"));

                    // ── Divider ──
                    let dy = h_rect.max.y;
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            pos2(h_rect.min.x + 16.0, dy),
                            vec2(ND_W - 32.0, 1.0),
                        ),
                        CornerRadius::same(0),
                        p.border_strong,
                    );

                    // ── Body (`.nd-body`: scrolls, 12/16/8 padding) ──
                    let body_max = (screen.height() - ND_HEADER - ND_FOOTER - 56.0).max(200.0);
                    nd_scroll_style(ui);
                    egui::ScrollArea::vertical()
                        .id_salt("nd-body")
                        // Height shrinks to the content (the old dialog grows
                        // and shrinks with its contents) but is capped at the
                        // viewport; beyond that the body scrolls.
                        .auto_shrink([false, true])
                        .max_height(body_max)
                        .show(ui, |ui| {
                            ui.set_width(ND_W);
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                            // ── Tab strip ──
                            // One full-strip interaction (the proven `seg_switch`
                            // pattern): the two halves are decided from the
                            // pointer's x, avoiding overlapping hit-test widgets.
                            ui.add_space(12.0);
                            let (tr, _) =
                                ui.allocate_exact_size(vec2(ND_W, 40.0), Sense::hover());
                            let tr = tr.shrink2(vec2(16.0, 0.0));
                            let tresp =
                                ui.interact(tr, egui::Id::new("nd-tab-strip"), Sense::click());
                            ui.painter().rect_filled(
                                tr,
                                CornerRadius::same(10),
                                p.hover_veil.gamma_multiply(0.5),
                            );
                            let half = vec2((tr.width() - 8.0) / 2.0, 32.0);
                            let t = ui.ctx().animate_value_with_time(
                                egui::Id::new("nd-tabs"),
                                if state.nd_torrent_tab { 1.0 } else { 0.0 },
                                0.12,
                            );
                            let ind = egui::Rect::from_min_size(
                                pos2(
                                    tr.min.x + 4.0 + t * (half.x + 4.0),
                                    tr.min.y + 4.0,
                                ),
                                half,
                            );
                            ui.painter()
                                .rect_filled(ind, CornerRadius::same(7), p.bg_card);
                            ui.painter().rect_stroke(
                                ind,
                                CornerRadius::same(7),
                                egui::Stroke::new(1.0, p.border),
                                egui::StrokeKind::Inside,
                            );
                            // Pointer x decides which half is hovered / clicked.
                            let px = tresp.interact_pointer_pos().unwrap_or(tr.min);
                            let right_half = px.x > tr.center().x;
                            if tresp.clicked() {
                                state.nd_torrent_tab = right_half;
                            }
                            for (sel, name, icon) in [
                                (0.0, state.t("URL"), "link"),
                                (1.0, state.t("Torrent"), "bolt"),
                            ] {
                                let b = egui::Rect::from_min_size(
                                    pos2(
                                        tr.min.x + 4.0 + sel * (half.x + 4.0),
                                        tr.min.y + 4.0,
                                    ),
                                    half,
                                );
                                let is_active = (sel == 1.0) == state.nd_torrent_tab;
                                let hovered = tresp.hovered() && (sel == 1.0) == right_half;
                                let col = if is_active {
                                    p.accent_strong
                                } else if hovered {
                                    p.text_muted
                                } else {
                                    p.text_dim
                                };
                                mi::paint_centered(
                                    ui.painter(),
                                    pos2(b.center().x - 24.0, b.center().y),
                                    icon,
                                    15.0,
                                    col,
                                );
                                ui.painter().text(
                                    pos2(b.center().x + 6.0, b.center().y),
                                    Align2::CENTER_CENTER,
                                    name,
                                    FontId::new(12.0, tokens::typography::semibold_family()),
                                    col,
                                );
                            }

                            // ── Source sections ──
                            ui.add_space(12.0);
                            match tab {
                                NdTab::Link => {
                                    nd_section_row(ui, &p, state.t("Source"), host.as_deref());
                                    let url_ph = state.t("https://... or any direct download link");
                                    nd_input(
                                        ui,
                                        &p,
                                        egui::Id::new("nd-url"),
                                        &mut state.add_url,
                                        url_ph,
                                        Some("close"),
                                        focus_req,
                                    );
                                    // `.nd-addon-hint` — `matchedAddon`: "Handled by
                                    // {name}" plus a button that hands the URL to the
                                    // addon page (the embedded addon host is deferred,
                                    // so only the handoff is recorded).
                                    if let Some((_, name)) = &addon_hint {
                                        let r = nd_row(ui, 34.0);
                                        ui.painter().rect_filled(
                                            r,
                                            CornerRadius::same(8),
                                            mk::blend(p.bg_card, p.accent, 0.08),
                                        );
                                        ui.painter().rect_stroke(
                                            r,
                                            CornerRadius::same(8),
                                            Stroke::new(1.0, mk::blend(p.border, p.accent, 0.25)),
                                            StrokeKind::Inside,
                                        );
                                        mi::paint_centered(
                                            ui.painter(),
                                            pos2(r.min.x + 16.0, r.center().y),
                                            "extension",
                                            15.0,
                                            p.accent_strong,
                                        );
                                        ui.painter().text(
                                            pos2(r.min.x + 30.0, r.center().y),
                                            Align2::LEFT_CENTER,
                                            format!("Handled by {name}"),
                                            FontId::new(11.5, egui::FontFamily::default()),
                                            p.text,
                                        );
                                        // `.nd-addon-open`
                                        let bw = 104.0;
                                        let btn = Rect::from_min_size(
                                            pos2(r.max.x - bw - 6.0, r.min.y + 5.0),
                                            vec2(bw, r.height() - 10.0),
                                        );
                                        if ui.allocate_rect(btn, Sense::click()).clicked() {
                                            want_open_addon = true;
                                        }
                                        ui.painter().rect_filled(
                                            btn,
                                            CornerRadius::same(7),
                                            p.accent,
                                        );
                                        ui.painter().text(
                                            btn.center(),
                                            Align2::CENTER_CENTER,
                                            state.t("Open in Addon"),
                                            FontId::new(
                                                11.0,
                                                tokens::typography::semibold_family(),
                                            ),
                                            p.bg_primary,
                                        );
                                    }
                                    // `.nd-clipboard-badge`
                                    if state.nd_pasted && !state.add_url.trim().is_empty() {
                                        let r = nd_row(ui, 22.0);
                                        let label = state.t("Pasted from clipboard");
                                        let tw = text_w(ui, label, 11.0);
                                        let pill = egui::Rect::from_min_size(
                                            pos2(r.min.x, r.min.y + 2.0),
                                            vec2(8.0 + 13.0 + 4.0 + tw + 8.0, 18.0),
                                        );
                                        ui.painter().rect_filled(
                                            pill,
                                            CornerRadius::same(4),
                                            p.hover_veil,
                                        );
                                        mi::paint_centered(
                                            ui.painter(),
                                            pos2(pill.min.x + 14.5, pill.center().y),
                                            "content_paste",
                                            13.0,
                                            p.text_dim,
                                        );
                                        ui.painter().text(
                                            pos2(pill.min.x + 25.0, pill.center().y),
                                            Align2::LEFT_CENTER,
                                            label,
                                            FontId::new(11.0, egui::FontFamily::default()),
                                            p.text_dim,
                                        );
                                    }
                                }
                                NdTab::Torrent => {
                                    nd_section_row(ui, &p, state.t("Torrent Source"), Some(state.t("BitTorrent")));
                                    nd_field_label(
                                        ui,
                                        &p,
                                        "description",
                                        state.t("Torrent Files"),
                                        Some(state.t("(optional if magnet)")),
                                    );

                                    // Dropzone
                                    let filled = !state.nd_torrents.is_empty();
                                    let dz = nd_row(ui, 100.0);
                                    let dz_rr = ui.interact(
                                        dz,
                                        egui::Id::new("nd-dropzone"),
                                        Sense::click(),
                                    );
                                    let (dz_border, dz_bg) = if dz_rr.hovered() {
                                        (
                                            mk::blend(p.border_strong, p.accent, 0.35),
                                            mk::blend(p.bg_card, p.accent, 0.03),
                                        )
                                    } else if filled {
                                        (
                                            mk::blend(p.border_strong, p.accent, 0.30),
                                            mk::blend(p.bg_card, p.accent, 0.04),
                                        )
                                    } else {
                                        (
                                            p.border_strong,
                                            mk::blend(p.bg_card, Color32::WHITE, 0.015),
                                        )
                                    };
                                    ui.painter()
                                        .rect_filled(dz, CornerRadius::same(10), dz_bg);
                                    nd_dashed_rect(ui.painter(), dz, dz_border, 1.5);
                                    let icon_col = if filled || dz_rr.hovered() {
                                        p.accent_strong
                                    } else {
                                        p.text_dim
                                    };
                                    if filled {
                                        mi::paint_centered(
                                            ui.painter(),
                                            pos2(dz.center().x, dz.min.y + 31.0),
                                            "insert_drive_file",
                                            26.0,
                                            icon_col,
                                        );
                                        ui.painter().text(
                                            pos2(dz.center().x, dz.min.y + 56.0),
                                            Align2::CENTER_CENTER,
                                            state.t_of(&format!(
                                                "{} {}",
                                                state.nd_torrents.len(),
                                                state.raw("torrent files selected")
                                            )),
                                            FontId::new(12.5, tokens::typography::semibold_family()),
                                            p.accent_strong,
                                        );
                                        ui.painter().text(
                                            pos2(dz.center().x, dz.min.y + 76.0),
                                            Align2::CENTER_CENTER,
                                            state.t("Click to add more or drop more"),
                                            FontId::new(11.0, egui::FontFamily::default()),
                                            p.text_dim,
                                        );
                                    } else {
                                        mi::paint_centered(
                                            ui.painter(),
                                            pos2(dz.center().x, dz.min.y + 31.0),
                                            "file_download",
                                            26.0,
                                            icon_col,
                                        );
                                        ui.painter().text(
                                            pos2(dz.center().x, dz.min.y + 56.0),
                                            Align2::CENTER_CENTER,
                                            state.t("Drop .torrent files here"),
                                            FontId::new(12.5, tokens::typography::semibold_family()),
                                            p.text_muted,
                                        );
                                        ui.painter().text(
                                            pos2(dz.center().x, dz.min.y + 76.0),
                                            Align2::CENTER_CENTER,
                                            state.t("or click to browse (multi-select)"),
                                            FontId::new(11.0, egui::FontFamily::default()),
                                            p.text_dim,
                                        );
                                    }
                                    if dz_rr.clicked() {
                                        want_pick_torrents = true;
                                    }

                                    // Torrent cards
                                    let select_all = state.t("SELECT ALL");
                                    for i in 0..state.nd_torrents.len() {
                                        let t = &mut state.nd_torrents[i];
                                        let head_h = 34.0;
                                        let body_h = if t.loading || !t.error.is_empty() {
                                            26.0
                                        } else if t.files.is_empty() {
                                            0.0
                                        } else {
                                            26.0 + (t.files.len() as f32 * 27.0).min(120.0)
                                        };
                                        let card = nd_row(ui, head_h + body_h);
                                        let card_bg = mk::blend(p.bg_card, Color32::WHITE, 0.015);
                                        ui.painter().rect_filled(
                                            card,
                                            CornerRadius::same(10),
                                            card_bg,
                                        );
                                        ui.painter().rect_stroke(
                                            card,
                                            CornerRadius::same(10),
                                            egui::Stroke::new(1.0, p.border),
                                            egui::StrokeKind::Inside,
                                        );
                                        // Head: bg strip + description icon +
                                        // name (clipped) + remove chip.
                                        let head = egui::Rect::from_min_max(
                                            card.min,
                                            pos2(card.max.x, card.min.y + head_h),
                                        );
                                        ui.painter().rect_filled(
                                            head,
                                            CornerRadius::same(9),
                                            p.hover_veil,
                                        );
                                        mi::paint_centered(
                                            ui.painter(),
                                            pos2(head.min.x + 12.0 + 8.5, head.center().y),
                                            "description",
                                            17.0,
                                            p.accent_strong,
                                        );
                                        let remove = egui::Rect::from_center_size(
                                            pos2(head.max.x - 12.0 - 9.0, head.center().y),
                                            vec2(18.0, 18.0),
                                        );
                                        let name_clip = egui::Rect::from_min_max(
                                            pos2(head.min.x + 12.0 + 17.0 + 8.0, head.min.y),
                                            pos2(remove.min.x - 6.0, head.max.y),
                                        );
                                        ui.painter().with_clip_rect(name_clip).text(
                                            pos2(name_clip.min.x, head.center().y),
                                            Align2::LEFT_CENTER,
                                            t.name.clone(),
                                            FontId::new(12.0, tokens::typography::semibold_family()),
                                            p.text,
                                        );
                                        let rm_rr = ui.interact(
                                            remove,
                                            egui::Id::new(("nd-torrent-rm", i)),
                                            Sense::click(),
                                        );
                                        if rm_rr.hovered() {
                                            ui.painter().rect_filled(
                                                remove,
                                                CornerRadius::same(4),
                                                p.hover_veil,
                                            );
                                        }
                                        mi::paint_centered(
                                            ui.painter(),
                                            remove.center(),
                                            "close",
                                            14.0,
                                            if rm_rr.hovered() { p.text } else { p.text_dim },
                                        );
                                        if rm_rr.clicked() {
                                            remove_torrent = Some(i);
                                        }
                                        // Body states
                                        if t.loading {
                                            let r = egui::Rect::from_min_max(
                                                pos2(card.min.x, card.min.y + head_h),
                                                card.max,
                                            );
                                            mi::paint_centered(
                                                ui.painter(),
                                                pos2(r.min.x + 12.0 + 7.0, r.center().y),
                                                "hourglass_top",
                                                14.0,
                                                p.text_dim,
                                            );
                                            ui.painter().text(
                                                pos2(r.min.x + 12.0 + 14.0 + 6.0, r.center().y),
                                                Align2::LEFT_CENTER,
                                                state.t("Reading contents..."),
                                                FontId::new(11.0, egui::FontFamily::default()),
                                                p.text_dim,
                                            );
                                        } else if !t.error.is_empty() {
                                            let r = egui::Rect::from_min_max(
                                                pos2(card.min.x, card.min.y + head_h),
                                                card.max,
                                            );
                                            mi::paint_centered(
                                                ui.painter(),
                                                pos2(r.min.x + 12.0 + 7.5, r.center().y),
                                                "error_outline",
                                                15.0,
                                                p.status_error,
                                            );
                                            ui.painter()
                                                .with_clip_rect(r)
                                                .text(
                                                    pos2(r.min.x + 12.0 + 15.0 + 6.0, r.center().y),
                                                    Align2::LEFT_CENTER,
                                                    t.error.clone(),
                                                    FontId::new(11.0, egui::FontFamily::default()),
                                                    p.status_error,
                                                );
                                        } else if !t.files.is_empty() {
                                            // Files header: select-all + counts.
                                            let hr = egui::Rect::from_min_max(
                                                pos2(card.min.x, card.min.y + head_h),
                                                pos2(card.max.x, card.min.y + head_h + 26.0),
                                            );
                                            let cb = egui::Rect::from_min_size(
                                                pos2(hr.min.x + 12.0, hr.center().y - 7.0),
                                                vec2(14.0, 14.0),
                                            );
                                            if nd_checkbox(
                                                ui,
                                                &p,
                                                cb,
                                                t.all_selected(),
                                                &format!("nd-tf-all-{i}"),
                                            ) {
                                                let all = t.all_selected();
                                                for s in t.selected.iter_mut() {
                                                    *s = !all;
                                                }
                                            }
                                            ui.painter().text(
                                                pos2(cb.max.x + 6.0, hr.center().y),
                                                Align2::LEFT_CENTER,
                                                select_all,
                                                FontId::new(
                                                    11.0,
                                                    tokens::typography::semibold_family(),
                                                ),
                                                p.text_muted,
                                            );
                                            let counts = format!(
                                                "{} / {} · {}",
                                                t.selected_count(),
                                                t.files.len(),
                                                moonhunt_core::util::format_bytes(
                                                    t.selected_bytes()
                                                )
                                            );
                                            ui.painter().text(
                                                pos2(hr.max.x - 12.0, hr.center().y),
                                                Align2::RIGHT_CENTER,
                                                counts,
                                                FontId::new(11.0, egui::FontFamily::default()),
                                                p.text_dim,
                                            );
                                            // Scrolling file rows (max 120px).
                                            let vp = egui::Rect::from_min_max(
                                                pos2(card.min.x, card.min.y + head_h + 26.0),
                                                card.max,
                                            );
                                            ui.scope_builder(
                                                egui::UiBuilder::new().max_rect(vp),
                                                |ui| {
                                                    nd_scroll_style(ui);
                                                    egui::ScrollArea::vertical()
                                                        .id_salt(("nd-tf", i))
                                                        .auto_shrink([false, false])
                                                        .max_height(vp.height())
                                                        .show(ui, |ui| {
                                                            ui.set_width(vp.width());
                                                            ui.spacing_mut().item_spacing =
                                                                vec2(0.0, 0.0);
                                                            for fi in 0..t.files.len() {
                                                                let (fname, fsize) =
                                                                    t.files[fi].clone();
                                                                let row = ui.allocate_exact_size(
                                                                    vec2(vp.width(), 27.0),
                                                                    Sense::hover(),
                                                                );
                                                                let r = egui::Rect::from_min_size(
                                                                    pos2(
                                                                        vp.min.x,
                                                                        row.0.min.y,
                                                                    ),
                                                                    row.0.size(),
                                                                );
                                                                let cb = egui::Rect::from_min_size(
                                                                    pos2(
                                                                        r.min.x + 12.0,
                                                                        r.center().y - 7.0,
                                                                    ),
                                                                    vec2(14.0, 14.0),
                                                                );
                                                                if nd_checkbox(
                                                                    ui,
                                                                    &p,
                                                                    cb,
                                                                    t.selected[fi],
                                                                    &format!(
                                                                        "nd-tf-{i}-{fi}"
                                                                    ),
                                                                ) {
                                                                    t.selected[fi] =
                                                                        !t.selected[fi];
                                                                }
                                                                mi::paint_centered(
                                                                    ui.painter(),
                                                                    pos2(
                                                                        cb.max.x + 8.0 + 7.5,
                                                                        r.center().y,
                                                                    ),
                                                                    "insert_drive_file",
                                                                    15.0,
                                                                    p.text_dim,
                                                                );
                                                                let size_txt =
                                                                    moonhunt_core::util::format_bytes(
                                                                        fsize,
                                                                    );
                                                                let size_w = text_w(
                                                                    ui,
                                                                    &size_txt,
                                                                    11.0,
                                                                );
                                                                let name_clip =
                                                                    egui::Rect::from_min_max(
                                                                        pos2(
                                                                            cb.max.x + 8.0
                                                                                + 15.0
                                                                                + 8.0,
                                                                            r.min.y,
                                                                        ),
                                                                        pos2(
                                                                            r.max.x - 12.0
                                                                                - size_w
                                                                                - 8.0,
                                                                            r.max.y,
                                                                        ),
                                                                    );
                                                                ui.painter()
                                                                    .with_clip_rect(name_clip)
                                                                    .text(
                                                                        pos2(
                                                                            name_clip.min.x,
                                                                            r.center().y,
                                                                        ),
                                                                        Align2::LEFT_CENTER,
                                                                        fname,
                                                                        FontId::new(
                                                                            12.0,
                                                                            egui::FontFamily::default(),
                                                                        ),
                                                                        p.text,
                                                                    );
                                                                ui.painter().text(
                                                                    pos2(
                                                                        r.max.x - 12.0,
                                                                        r.center().y,
                                                                    ),
                                                                    Align2::RIGHT_CENTER,
                                                                    size_txt,
                                                                    FontId::new(
                                                                        11.0,
                                                                        egui::FontFamily::default(),
                                                                    ),
                                                                    p.text_dim,
                                                                );
                                                                // Row divider.
                                                                if fi + 1 != t.files.len() {
                                                                    ui.painter().rect_filled(
                                                                        egui::Rect::from_min_max(
                                                                            pos2(
                                                                                r.min.x,
                                                                                r.max.y - 1.0,
                                                                            ),
                                                                            r.max,
                                                                        ),
                                                                        CornerRadius::same(0),
                                                                        p.border,
                                                                    );
                                                                }
                                                            }
                                                        });
                                                },
                                            );
                                        }
                                    }

                                    // Magnet field + hint
                                    nd_field_label(
                                        ui,
                                        &p,
                                        "link",
                                        state.t("Magnet Link"),
                                        Some(state.t("(optional if file)")),
                                    );
                                    nd_input(
                                        ui,
                                        &p,
                                        egui::Id::new("nd-magnet"),
                                        &mut state.nd_magnet,
                                        "magnet:?xt=urn:btih:...",
                                        Some("close"),
                                        false,
                                    );
                                    nd_hint(
                                        ui,
                                        &p,
                                        state.t("Provide a .torrent file or a magnet link — or both. aria2 handles the swarm natively."),
                                    );
                                }
                            }

                            // ── Duplicate URL warning ──
                            if dup_gid.is_some() {
                                ui.add_space(20.0); // body gap 12 + margin-top 8
                                let r = nd_row(ui, 70.0);
                                ui.painter().rect_filled(
                                    r,
                                    CornerRadius::same(10),
                                    Color32::from_rgba_unmultiplied(255, 152, 0, 20),
                                );
                                ui.painter().rect_stroke(
                                    r,
                                    CornerRadius::same(10),
                                    egui::Stroke::new(
                                        1.0,
                                        Color32::from_rgba_unmultiplied(255, 152, 0, 51),
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                let amber = Color32::from_rgb(0xff, 0xb7, 0x4d);
                                let icon_r = egui::Rect::from_min_size(
                                    pos2(r.min.x + 14.0, r.min.y + 13.0),
                                    vec2(36.0, 36.0),
                                );
                                ui.painter().rect_filled(
                                    icon_r,
                                    CornerRadius::same(8),
                                    Color32::from_rgba_unmultiplied(255, 152, 0, 38),
                                );
                                mi::paint_centered(
                                    ui.painter(),
                                    icon_r.center(),
                                    "warning",
                                    18.0,
                                    amber,
                                );
                                let tx = icon_r.max.x + 12.0;
                                ui.painter().text(
                                    pos2(tx, r.min.y + 21.0),
                                    Align2::LEFT_CENTER,
                                    state.t("Duplicate URL detected"),
                                    FontId::new(13.0, tokens::typography::semibold_family()),
                                    amber,
                                );
                                // `.nd-dupe-toggle` 34×20 amber switch.
                                let tr = egui::Rect::from_min_size(
                                    pos2(tx, r.min.y + 38.0),
                                    vec2(34.0, 20.0),
                                );
                                let on = state.nd_allow_duplicate;
                                let t = ui.ctx().animate_value_with_time(
                                    egui::Id::new("nd-dupe").with("t"),
                                    if on { 1.0 } else { 0.0 },
                                    0.12,
                                );
                                ui.painter().rect_filled(
                                    tr,
                                    CornerRadius::same(10),
                                    mk::blend(
                                        Color32::from_rgb(0x50, 0x36, 0x0f),
                                        Color32::from_rgb(0xc7, 0x7f, 0x1a),
                                        t,
                                    ),
                                );
                                ui.painter().rect_stroke(
                                    tr,
                                    CornerRadius::same(10),
                                    egui::Stroke::new(
                                        1.0,
                                        mk::blend(
                                            Color32::from_rgba_unmultiplied(255, 152, 0, 64),
                                            Color32::from_rgb(0xd9, 0x9a, 0x2b),
                                            t,
                                        ),
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                ui.painter().circle_filled(
                                    pos2(tr.min.x + 9.0 + 14.0 * t, tr.center().y),
                                    7.0,
                                    mk::blend(p.text_dim, amber, t),
                                );
                                let dupe_rr = ui.interact(
                                    tr,
                                    egui::Id::new("nd-dupe"),
                                    Sense::click(),
                                );
                                if dupe_rr.clicked() {
                                    state.nd_allow_duplicate = !on;
                                }
                                // `.nd-duplicate-action` Show button.
                                let bw = 74.0;
                                let br = egui::Rect::from_min_size(
                                    pos2(r.max.x - 14.0 - bw, r.min.y + 13.0),
                                    vec2(bw, 30.0),
                                );
                                let show_rr = ui.interact(
                                    br,
                                    egui::Id::new("nd-dupe-show"),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    br,
                                    CornerRadius::same(8),
                                    Color32::from_rgba_unmultiplied(255, 152, 0, 26),
                                );
                                ui.painter().rect_stroke(
                                    br,
                                    CornerRadius::same(8),
                                    egui::Stroke::new(
                                        1.0,
                                        Color32::from_rgba_unmultiplied(255, 152, 0, 51),
                                    ),
                                    egui::StrokeKind::Inside,
                                );
                                {
                                    let font = FontId::new(
                                        11.5,
                                        tokens::typography::semibold_family(),
                                    );
                                    let lw = ui
                                        .painter()
                                        .layout_no_wrap(
                                            state.t("Show").to_owned(),
                                            font.clone(),
                                            Color32::WHITE,
                                        )
                                        .rect
                                        .width();
                                    let gx = br.center().x - (18.0 + lw) * 0.5;
                                    mi::paint_centered(
                                        ui.painter(),
                                        pos2(gx + 7.0, br.center().y),
                                        "visibility",
                                        14.0,
                                        amber,
                                    );
                                    ui.painter().text(
                                        pos2(gx + 18.0, br.center().y),
                                        Align2::LEFT_CENTER,
                                        state.t("Show"),
                                        font,
                                        amber,
                                    );
                                }
                                if show_rr.clicked() {
                                    want_show_dup = true;
                                }
                            }

                            // ── Save As ──
                            ui.add_space(12.0);
                            nd_section_row(ui, &p, state.t("Save As"), Option::<&str>::None);
                            let auto_ph = state.t("Auto-detect from URL");
                            nd_input(
                                ui,
                                &p,
                                egui::Id::new("nd-filename"),
                                &mut state.nd_filename,
                                auto_ph,
                                Some("close"),
                                false,
                            );

                            // ── Save Location ──
                            ui.add_space(12.0);
                            nd_section_row(ui, &p, state.t("Save Location"), Option::<&str>::None);
                            let folder_ph = state.t("Save path placeholder");
                            nd_input(
                                ui,
                                &p,
                                egui::Id::new("nd-path"),
                                &mut state.nd_path,
                                folder_ph,
                                Some("folder_open"),
                                false,
                            );

                            // ── Connections (ConnectionSelector stepper) ──
                            ui.add_space(12.0);
                            nd_section_row(ui, &p, state.t("Connections"), Option::<&str>::None);
                            let (srow, _) = ui.allocate_exact_size(vec2(ND_W, 36.0), Sense::hover());
                            let srow = srow.shrink2(vec2(16.0, 0.0));
                            // `.cs-stepper`: one bordered 32px pill — two 30px
                            // `.cs-step` buttons around a 44px `.cs-input` well
                            // with left/right hairlines.
                            let cs = egui::Rect::from_min_size(
                                pos2(srow.min.x, srow.min.y + 2.0),
                                vec2(104.0, 32.0),
                            );
                            ui.painter().rect_filled(cs, CornerRadius::same(9), p.bg_surface);
                            ui.painter().rect_stroke(
                                cs,
                                CornerRadius::same(9),
                                egui::Stroke::new(1.0, p.border_strong),
                                egui::StrokeKind::Inside,
                            );
                            let well = egui::Rect::from_min_size(
                                pos2(cs.min.x + 30.0, cs.min.y),
                                vec2(44.0, 32.0),
                            );
                            for x in [well.min.x, well.max.x - 1.0] {
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(pos2(x, cs.min.y), vec2(1.0, 32.0)),
                                    CornerRadius::same(0),
                                    p.border_strong,
                                );
                            }
                            let mrect =
                                egui::Rect::from_min_max(cs.min, pos2(cs.min.x + 30.0, cs.max.y));
                            if nd_step_btn(
                                ui,
                                &p,
                                mrect,
                                "nd-conn-minus",
                                "remove",
                                state.nd_connections <= 1,
                            ) {
                                state.nd_connections = (state.nd_connections as i32 - 1).max(1) as u32;
                            }
                            ui.painter().text(
                                well.center(),
                                Align2::CENTER_CENTER,
                                format!("{}", state.nd_connections),
                                FontId::new(12.0, tokens::typography::semibold_family()),
                                p.text,
                            );
                            let prect =
                                egui::Rect::from_min_max(pos2(cs.max.x - 30.0, cs.min.y), cs.max);
                            if nd_step_btn(ui, &p, prect, "nd-conn-plus", "add", false) {
                                state.nd_connections += 1;
                            }

                            // ── Categories (URL tab only) ──
                            if matches!(tab, NdTab::Link) {
                                ui.add_space(12.0);
                                let cat_enabled = state.nd_cat_enabled;
                                let cat_body = if cat_enabled { 76.0 } else { 0.0 };
                                let card = nd_card(ui, &p, "category", state.t("Categories"), cat_enabled, cat_body);
                                let toggle = egui::Rect::from_min_size(
                                    pos2(card.max.x - 14.0 - 42.0, card.min.y + 10.0),
                                    vec2(42.0, 24.0),
                                );
                                if cat_enabled {
                                    let manage = egui::Rect::from_min_size(
                                        pos2(toggle.min.x - 8.0 - 78.0, card.min.y + 10.0),
                                        vec2(78.0, 24.0),
                                    );
                                    if nd_link_button(
                                        ui, &p, manage, "folder_open", state.t("Manage"), "nd-cat-manage",
                                    ) {
                                        want_open_catmgr = true;
                                    }
                                }
                                if nd_switch(ui, &p, toggle, cat_enabled, "nd-cat-toggle") {
                                    state.nd_cat_enabled = !cat_enabled;
                                    state.nd_cat_error.clear();
                                }
                                if cat_enabled {
                                    nd_card_divider(ui, &p, card);
                                    let y = card.min.y + 44.0 + 12.0;
                                    let inner_x = card.min.x + 14.0;
                                    let inner_w = card.width() - 28.0;
                                    // Mode dropdown (CustomDropdown).
                                    let modes = [
                                        ("0".to_owned(), state.t("Formatted").to_owned()),
                                        ("1".to_owned(), state.t("Packaged").to_owned()),
                                        ("2".to_owned(), state.t("Manual").to_owned()),
                                    ];
                                    let mode_rect = egui::Rect::from_min_size(
                                        pos2(inner_x, y),
                                        vec2(inner_w, 38.0),
                                    );
                                    let cur_mode = state.nd_cat_mode.to_string();
                                    if let Some(v) = nd_dropdown(
                                        ui,
                                        &p,
                                        &mut state.nd_dd_open,
                                        "nd-cat-mode",
                                        mode_rect,
                                        &modes,
                                        &cur_mode,
                                    ) {
                                        state.nd_cat_mode = v.parse().unwrap_or(0);
                                        state.nd_cat_error.clear();
                                    }
                                    let y2 = y + 38.0 + 8.0;
                                    if state.nd_cat_mode == 2 {
                                        // Manual: category dropdown + hint/preview.
                                        let mut opts =
                                            vec![(String::new(), state.t("Choose a category...").to_owned())];
                                        for c in &state.nd_categories {
                                            opts.push((c.clone(), c.clone()));
                                        }
                                        let sel_rect = egui::Rect::from_min_size(
                                            pos2(inner_x, y2),
                                            vec2(inner_w, 38.0),
                                        );
                                        let cur_sel = state.nd_cat_selected.clone();
                                        if let Some(v) = nd_dropdown(
                                            ui,
                                            &p,
                                            &mut state.nd_dd_open,
                                            "nd-cat-sel",
                                            sel_rect,
                                            &opts,
                                            &cur_sel,
                                        ) {
                                            state.nd_cat_selected = v;
                                            state.nd_cat_error.clear();
                                        }
                                        let y3 = y2 + 38.0 + 8.0;
                                        let row = egui::Rect::from_min_size(
                                            pos2(inner_x, y3),
                                            vec2(inner_w, 18.0),
                                        );
                                        if state.nd_cat_selected.is_empty() {
                                            nd_hint_at(
                                                ui,
                                                &p,
                                                row,
                                                state.t("Select a saved category or open Manage to create one."),
                                            );
                                        } else {
                                            nd_preview_row(
                                                ui,
                                                &p,
                                                row,
                                                "folder",
                                                &state.nd_cat_selected,
                                            );
                                        }
                                    } else {
                                        // Auto folder/tree preview.
                                        let filename_src = if !state.nd_filename.trim().is_empty()
                                        {
                                            state.nd_filename.trim().to_owned()
                                        } else if !detected_filename.is_empty() {
                                            detected_filename.clone()
                                        } else {
                                            state.add_url.clone()
                                        };
                                        let preview = if state.nd_cat_mode == 1 {
                                            format!(
                                                "Auto tree: {}",
                                                packaged_category_folder(&filename_src)
                                            )
                                        } else {
                                            format!(
                                                "Auto folder: {}",
                                                formatted_category_folder(&filename_src)
                                            )
                                        };
                                        let row = egui::Rect::from_min_size(
                                            pos2(inner_x, y2),
                                            vec2(inner_w, 18.0),
                                        );
                                        nd_preview_row(ui, &p, row, "folder", &preview);
                                    }
                                }
                                // Inline `.nd-category-error` row.
                                if !state.nd_cat_error.is_empty() {
                                    let r = nd_row(ui, 18.0);
                                    mi::paint_centered(
                                        ui.painter(),
                                        pos2(r.min.x + 7.0, r.center().y),
                                        "info",
                                        14.0,
                                        p.accent_strong,
                                    );
                                    ui.painter().with_clip_rect(r).text(
                                        pos2(r.min.x + 18.0, r.center().y),
                                        Align2::LEFT_CENTER,
                                        state.nd_cat_error.clone(),
                                        FontId::new(11.0, egui::FontFamily::default()),
                                        p.text_dim,
                                    );
                                }
                            }

                            // ── Schedule (SchedulePicker) ──
                            ui.add_space(12.0);
                            let sched_enabled = state.nd_sched_enabled;
                            let sched_body = if sched_enabled { 30.0 } else { 0.0 };
                            let card =
                                nd_card(ui, &p, "schedule", state.t("Schedule"), sched_enabled, sched_body);
                            let toggle = egui::Rect::from_min_size(
                                pos2(card.max.x - 14.0 - 42.0, card.min.y + 10.0),
                                vec2(42.0, 24.0),
                            );
                            if sched_enabled {
                                let configure = egui::Rect::from_min_size(
                                    pos2(toggle.min.x - 8.0 - 90.0, card.min.y + 10.0),
                                    vec2(90.0, 24.0),
                                );
                                if nd_link_button(
                                    ui, &p, configure, "event", state.t("Configure"), "nd-sched-config",
                                ) {
                                    want_open_sched = true;
                                }
                            }
                            if nd_switch(ui, &p, toggle, sched_enabled, "nd-sched-toggle") {
                                state.nd_sched_enabled = !sched_enabled;
                                if !sched_enabled {
                                    // Toggling off clears the pending schedule
                                    // (`emit('update:schedule', null)`).
                                    state.nd_sched_set = false;
                                }
                            }
                            if sched_enabled {
                                nd_card_divider(ui, &p, card);
                                let row = egui::Rect::from_min_size(
                                    pos2(card.min.x + 14.0, card.min.y + 44.0 + 12.0),
                                    vec2(card.width() - 28.0, 18.0),
                                );
                                if state.nd_sched_set {
                                    let preview = state.nd_schedule_preview();
                                    nd_preview_row(ui, &p, row, "event_available", &preview);
                                } else {
                                    let r = egui::Rect::from_min_max(
                                        row.min,
                                        pos2(row.max.x, row.max.y - 2.0),
                                    );
                                    ui.painter().text(
                                        pos2(r.min.x, r.center().y),
                                        Align2::LEFT_CENTER,
                                        state.t("Configure a date and time..."),
                                        FontId::new(11.0, egui::FontFamily::default()),
                                        p.text_dim,
                                    );
                                }
                            }

                            // ── Proxy (ProxyField) ──
                            ui.add_space(12.0);
                            let px_enabled = state.nd_proxy_enabled;
                            let detect_msg = if state.nd_proxy_msg.is_empty() {
                                String::new()
                            } else {
                                state.nd_proxy_msg.clone()
                            };
                            let px_body = if px_enabled {
                                124.0 + if detect_msg.is_empty() { 0.0 } else { 24.0 }
                            } else {
                                0.0
                            };
                            let card = nd_card(ui, &p, "web", state.t("Enable Proxy"), px_enabled, px_body);
                            let toggle = egui::Rect::from_min_size(
                                pos2(card.max.x - 14.0 - 42.0, card.min.y + 10.0),
                                vec2(42.0, 24.0),
                            );
                            // `.pf-detect-btn` — auto-detect system proxy.
                            let detect = egui::Rect::from_min_size(
                                pos2(toggle.min.x - 6.0 - 30.0, card.min.y + 7.0),
                                vec2(30.0, 30.0),
                            );
                            let det_rr =
                                ui.interact(detect, egui::Id::new("nd-px-detect"), Sense::click());
                            let detecting = state.nd_proxy_detecting;
                            if det_rr.hovered() && !detecting {
                                ui.painter().rect_filled(
                                    detect,
                                    CornerRadius::same(8),
                                    p.hover_veil,
                                );
                                ui.painter().rect_stroke(
                                    detect,
                                    CornerRadius::same(8),
                                    egui::Stroke::new(1.0, p.border),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            if detecting {
                                ui.painter().rect_filled(
                                    detect,
                                    CornerRadius::same(8),
                                    p.hover_veil.gamma_multiply(0.5),
                                );
                            }
                            mi::paint_centered(
                                ui.painter(),
                                detect.center(),
                                if detecting { "hourglass_empty" } else { "wifi_find" },
                                17.0,
                                if det_rr.hovered() && !detecting {
                                    p.accent_strong
                                } else {
                                    p.text_dim
                                },
                            );
                            if det_rr.clicked() && !detecting {
                                state.nd_proxy_detecting = true;
                                state.nd_proxy_msg.clear();
                                cctx.run(move |app| async move {
                                    match moonhunt_core::aria2::detect_system_proxy().await {
                                        Ok(v) => app.emit("nd-proxy-detect", v),
                                        Err(e) => app.emit(
                                            "nd-proxy-detect",
                                            serde_json::json!({
                                                "enabled": false,
                                                "host": "",
                                                "port": 0,
                                                "message": e,
                                            }),
                                        ),
                                    }
                                    Ok::<String, String>(String::new())
                                });
                            }
                            if nd_switch(ui, &p, toggle, px_enabled, "nd-px-toggle") {
                                state.nd_proxy_enabled = !px_enabled;
                            }
                            if px_enabled {
                                nd_card_divider(ui, &p, card);
                                let inner_x = card.min.x + 14.0;
                                let col_w = (card.width() - 28.0 - 8.0) / 2.0;
                                let y0 = card.min.y + 44.0 + 12.0;
                                let cell = |i: usize, j: usize| -> egui::Rect {
                                    egui::Rect::from_min_size(
                                        pos2(inner_x + i as f32 * (col_w + 8.0), y0 + j as f32 * 60.0),
                                        vec2(col_w, 32.0),
                                    )
                                };
                                let label_at = |ui: &mut Ui, rect: egui::Rect, text: &str, opt: Option<&str>| {
                                    let ly = rect.min.y - 7.0;
                                    ui.painter().text(
                                        pos2(rect.min.x, ly),
                                        Align2::LEFT_CENTER,
                                        text,
                                        FontId::new(11.0, egui::FontFamily::default()),
                                        p.text_dim,
                                    );
                                    if let Some(extra) = opt {
                                        let x = rect.min.x
                                            + text_w(ui, text, 11.0)
                                            + 4.0;
                                        ui.painter().text(
                                            pos2(x, ly),
                                            Align2::LEFT_CENTER,
                                            extra,
                                            FontId::new(10.0, egui::FontFamily::default()),
                                            p.text_dim,
                                        );
                                    }
                                };
                                // Placeholders are hoisted: `nd_input_at` takes
                                // `&mut state.nd_proxy_*` alongside them.
                                let ph_host = state.t("proxy.example.com");
                                let ph_none = state.t("Leave blank if none");
                                let r = cell(0, 0);
                                label_at(ui, r, state.t("Host"), None);
                                nd_input_at(
                                    ui, &p, egui::Id::new("pf-host"),
                                    egui::Rect::from_min_size(r.min, r.size()),
                                    &mut state.nd_proxy_host,
                                    ph_host, None, false, false,
                                );
                                let r = cell(1, 0);
                                label_at(ui, r, state.t("Port"), None);
                                nd_input_at(
                                    ui, &p, egui::Id::new("pf-port"),
                                    egui::Rect::from_min_size(r.min, r.size()),
                                    &mut state.nd_proxy_port,
                                    "1080", None, false, false,
                                );
                                let r = cell(0, 1);
                                label_at(ui, r, state.t("Username"), Some(state.t("(optional)")));
                                nd_input_at(
                                    ui, &p, egui::Id::new("pf-user"),
                                    egui::Rect::from_min_size(r.min, r.size()),
                                    &mut state.nd_proxy_user,
                                    ph_none, None, false, false,
                                );
                                let r = cell(1, 1);
                                label_at(ui, r, state.t("Password"), Some(state.t("(optional)")));
                                nd_input_at(
                                    ui, &p, egui::Id::new("pf-pass"),
                                    egui::Rect::from_min_size(r.min, r.size()),
                                    &mut state.nd_proxy_pass,
                                    ph_none, None, true, false,
                                );
                                if !detect_msg.is_empty() {
                                    let ok = state.nd_proxy_msg_ok;
                                    let row = egui::Rect::from_min_size(
                                        pos2(inner_x, y0 + 112.0 + 8.0),
                                        vec2(card.width() - 28.0, 16.0),
                                    );
                                    ui.painter().with_clip_rect(row).text(
                                        pos2(row.min.x, row.center().y),
                                        Align2::LEFT_CENTER,
                                        detect_msg,
                                        FontId::new(11.5, egui::FontFamily::default()),
                                        if ok { p.status_complete } else { p.text_dim },
                                    );
                                }
                            }

                            ui.add_space(8.0); // `.nd-body` padding-bottom
                        });

                    // ── Footer ──
                    ui.add_space(12.0);
                    let (f_rect, _) =
                        ui.allocate_exact_size(vec2(ND_W, ND_FOOTER), Sense::hover());
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            pos2(f_rect.min.x, f_rect.min.y),
                            vec2(ND_W, 1.0),
                        ),
                        CornerRadius::same(0),
                        p.border,
                    );
                    let bw = (ND_W - 32.0 - 8.0) / 2.0;
                    let by = f_rect.min.y + 7.0;
                    if nd_button(
                        ui,
                        &p,
                        "nd-cancel",
                        egui::Rect::from_min_size(
                            pos2(f_rect.min.x + 16.0, by),
                            vec2(bw, 38.0),
                        ),
                        state.t("Cancel"),
                        Some("close"),
                        false,
                        true,
                    ) {
                        want_close = true;
                    }
                    // Primary: spinner + "Starting..." while in flight, else the
                    // tab's own label (`Start Download` / `Add Torrent`).
                    let (add_label, add_icon) = if adding {
                        (state.t("Starting..."), "hourglass_top")
                    } else if matches!(tab, NdTab::Link) {
                        (state.t("Start Download"), "cloud_download")
                    } else {
                        (state.t("Add Torrent"), "bolt")
                    };
                    if nd_button(
                        ui,
                        &p,
                        "nd-add",
                        egui::Rect::from_min_size(
                            pos2(f_rect.min.x + 16.0 + bw + 8.0, by),
                            vec2(bw, 38.0),
                        ),
                        add_label,
                        Some(add_icon),
                        true,
                        can_submit,
                    ) {
                        want_submit = true;
                    }
                });
        });

    // `watch(newUrl)` autofill — only when the URL actually changed this frame.
    if state.add_url != url_before {
        state.nd_autofill_filename();
    }
    if focus_req {
        state.nd_focus_frames = state.nd_focus_frames.saturating_sub(1);
        if state.nd_focus_frames == 0 {
            state.nd_focus_next = false;
        }
    }

    // Global keys: Esc closes; Enter submits — swallowed while a sub-dialog
    // (ScheduleModal / CategoryManagerModal) owns the keyboard.
    if !sub_open {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            want_close = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            want_submit = true;
        }
    }

    if want_pick_torrents {
        nd_pick_torrents(state, cctx);
    }
    if let Some(idx) = remove_torrent {
        if idx < state.nd_torrents.len() {
            state.nd_torrents.remove(idx);
        }
    }
    if want_open_sched {
        // `ScheduleModal.open()` resets its form.
        state.nd_sched_open = true;
        state.nd_sched_action = 0;
        state.nd_sched_date.clear();
        state.nd_sched_time.clear();
        state.nd_sched_repeat = 0;
        state.nd_sched_error.clear();
    }
    if want_open_catmgr {
        // `CategoryManagerModal.open()` resets + reloads.
        state.nd_catmgr_open = true;
        state.nd_catmgr_new.clear();
        state.nd_catmgr_edit = None;
        state.nd_catmgr_edit_value.clear();
        state.nd_categories = nd_load_categories();
    }
    if want_open_addon {
        // `open-addon` — leave the dialog and land on the addon page with the
        // URL queued for it (the embedded addon host itself is still deferred).
        if let Some((id, _)) = addon_hint {
            state.screen = crate::state::Screen::Addon;
            state.addon.active = id;
            state.addon.pending_url = Some(state.add_url.trim().to_owned());
            state.show_new_download = false;
            state.nd_clear_source();
        }
    }
    if want_show_dup {
        // `bringToExistingDownload`: close, land on the list, announce it.
        let gid = dup_gid.unwrap_or_default();
        let name = state
            .downloads
            .iter()
            .find(|d| d.gid == gid)
            .map(|d| d.filename.clone())
            .unwrap_or_default();
        state.show_new_download = false;
        state.nd_dd_open = None;
        state.screen = crate::state::Screen::Downloads;
        state.toast(
            true,
            format!(
                "Download already exists: {}",
                if name.is_empty() { gid } else { name }
            ),
        );
    }
    if want_close && !want_submit {
        state.show_new_download = false;
        state.nd_clear_source();
        state.nd_dd_open = None;
    }
    if want_submit && can_submit {
        nd_submit(state, cctx, tab);
    }
}

// ─── New-download support: proxy badge, tick, dropper, schedulers ───────────

/// `ProxyUrlCheckBadge` view-model, snapshotted before the modal Area borrows
/// `state` mutably.
struct NdProxyBadge {
    busy: bool,
    direct: bool,
    ok: bool,
    text: Option<String>,
}

/// The badge pill in `.nd-header-actions`, right-aligned before the close
/// button: dot + label, tinted per state (accent / direct / ok / fail).
/// `Proxy · 200 · 43ms` and `Proxy · Proxy test failed` are built by the check
/// task, which has no `UiState`. Swap the two English words for their table
/// values here and order the line once, so the chip reads «پروکسی · 200 · 43ms».
fn translate_badge(lang: &str, text: &str) -> String {
    let swapped = text
        .replace("Proxy test failed", crate::i18n::raw(lang, "Proxy test failed"))
        .replace("Proxy", crate::i18n::raw(lang, "Proxy"));
    crate::i18n::show(lang, &swapped)
}

fn nd_proxy_badge(
    ui: &Ui,
    p: &Palette,
    h_rect: Rect,
    right: f32,
    b: &NdProxyBadge,
    lang: &str,
) {
    let (label, color, bg, border, dot) = if b.busy {
        (
            crate::i18n::t(lang, "Proxy…"),
            p.accent_strong,
            mk::blend(p.bg_card, p.accent, 0.07),
            mk::blend(p.border, p.accent, 0.16),
            p.accent,
        )
    } else if b.direct {
        (
            crate::i18n::t(lang, "Direct"),
            p.text_dim,
            p.hover_veil,
            p.border,
            p.text_dim,
        )
    } else if let Some(t) = &b.text {
        if b.ok {
            (
                t.as_str(),
                p.status_complete,
                Color32::from_rgba_unmultiplied(129, 199, 132, 20),
                Color32::from_rgba_unmultiplied(129, 199, 132, 56),
                p.status_complete,
            )
        } else {
            (
                t.as_str(),
                p.status_error,
                Color32::from_rgba_unmultiplied(239, 83, 80, 20),
                Color32::from_rgba_unmultiplied(239, 83, 80, 56),
                p.status_error,
            )
        }
    } else {
        (
            "Proxy",
            p.accent_strong,
            mk::blend(p.bg_card, p.accent, 0.07),
            mk::blend(p.border, p.accent, 0.16),
            p.accent,
        )
    };
    let label_owned = translate_badge(lang, label);
    let label = label_owned.as_str();
    let font = FontId::new(10.0, tokens::typography::semibold_family());
    let tw = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width();
    let w = 8.0 + 5.0 + 5.0 + tw + 8.0;
    let r = Rect::from_min_size(
        pos2(right - w, h_rect.min.y + 20.0),
        vec2(w, 20.0),
    );
    ui.painter().rect_filled(r, CornerRadius::same(10), bg);
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(10),
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    ui.painter().circle_filled(
        pos2(r.min.x + 8.0 + 2.5, r.center().y),
        2.5,
        dot,
    );
    ui.painter().text(
        pos2(r.min.x + 8.0 + 5.0 + 5.0, r.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        color,
    );
    // `title` tooltip (parity with the Vue badge's title attribute).
    let rr = ui.interact(r, egui::Id::new("nd-proxy-badge"), Sense::hover());
    let title = if b.direct {
        "No proxy on aria2 — direct connection".to_owned()
    } else if b.busy {
        "Proxy: checking…".to_owned()
    } else if let Some(t) = &b.text {
        t.clone()
    } else {
        "Proxy".to_owned()
    };
    rr.on_hover_text(title);
}

/// Debounced proxy re-check (the old badge's `scheduleCheck`): 350ms after the
/// URL settles, one task fetches the proxy info and runs `test_proxy_url`,
/// then pushes the result back through the `nd-proxy-check` event. A URL change
/// mid-flight invalidates the response via the event's key guard.
fn nd_proxy_check_tick(state: &mut UiState, cctx: &Ctx, url: &str) {
    let target = url.trim().to_owned();
    if state.nd_pcheck_key != target {
        state.nd_pcheck_key = target.clone();
        state.nd_pcheck_at = Instant::now();
        state.nd_pcheck_busy = false;
    }
    if state.nd_pcheck_busy || state.nd_pcheck_done == state.nd_pcheck_key {
        return;
    }
    if state.nd_pcheck_at.elapsed() < Duration::from_millis(350) {
        return;
    }
    state.nd_pcheck_done = state.nd_pcheck_key.clone();
    state.nd_pcheck_busy = true;
    let key = state.nd_pcheck_key.clone();
    let is_http = {
        let l = key.to_ascii_lowercase();
        l.starts_with("http://") || l.starts_with("https://")
    };
    cctx.run(move |app| async move {
        let info = moonhunt_core::aria2::check_aria2_proxy(app.clone())
            .await
            .unwrap_or_else(|_| serde_json::json!({ "configured": false }));
        let configured = info
            .get("configured")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let (text, ok, direct) = if !configured {
            // No proxy on aria2 — the badge just says "Direct".
            ("Direct".to_owned(), false, true)
        } else if key.is_empty() {
            // Proxy configured, no URL yet — plain "Proxy".
            ("Proxy".to_owned(), false, false)
        } else if !is_http {
            ("Proxy · Proxy test failed".to_owned(), false, false)
        } else {
            match moonhunt_core::aria2::test_proxy_url(app.clone(), key.clone(), None).await {
                Ok(r) => {
                    let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    let status = r.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
                    let ms = r.get("latencyMs").and_then(|v| v.as_u64()).unwrap_or(0);
                    let text = if ok {
                        format!("Proxy · {} · {}ms", status, ms)
                    } else if status > 0 {
                        format!("Proxy · {}", status)
                    } else {
                        "Proxy · Proxy test failed".to_owned()
                    };
                    (text, ok, false)
                }
                Err(_) => ("Proxy · Proxy test failed".to_owned(), false, false),
            }
        };
        app.emit(
            "nd-proxy-check",
            serde_json::json!({ "key": key, "text": text, "ok": ok, "direct": direct }),
        );
        Ok::<String, String>(String::new())
    });
}

// ─── Small painting widgets ─────────────────────────────────────────────────

/// 14×14 checkbox (`.nd-torrent-select-all` / `.nd-torrent-file-row` input).
fn nd_checkbox(ui: &mut Ui, p: &Palette, rect: Rect, on: bool, id: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    let fill = if on {
        p.accent
    } else {
        mk::blend(p.bg_card, Color32::WHITE, 0.03)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(3), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(3),
        egui::Stroke::new(1.0, if on { p.accent } else { p.border_strong }),
        egui::StrokeKind::Inside,
    );
    if on {
        mi::paint_centered(
            ui.painter(),
            rect.center(),
            "check",
            11.0,
            Color32::from_rgb(0x0a, 0x0e, 0x1a),
        );
    }
    rr.clicked()
}

/// Dashed rectangle (`.nd-dropzone` border — egui strokes are solid, so the
/// dashes are drawn as short segments).
fn nd_dashed_rect(painter: &egui::Painter, r: Rect, color: Color32, thickness: f32) {
    let (dash, gap) = (5.0_f32, 4.0_f32);
    let mut x = r.min.x;
    while x < r.max.x {
        let w = (x + dash).min(r.max.x) - x;
        painter.rect_filled(
            Rect::from_min_size(pos2(x, r.min.y), vec2(w, thickness)),
            CornerRadius::same(0),
            color,
        );
        painter.rect_filled(
            Rect::from_min_size(pos2(x, r.max.y - thickness), vec2(w, thickness)),
            CornerRadius::same(0),
            color,
        );
        x += dash + gap;
    }
    let mut y = r.min.y;
    while y < r.max.y {
        let h = (y + dash).min(r.max.y) - y;
        painter.rect_filled(
            Rect::from_min_size(pos2(r.min.x, y), vec2(thickness, h)),
            CornerRadius::same(0),
            color,
        );
        painter.rect_filled(
            Rect::from_min_size(pos2(r.max.x - thickness, y), vec2(thickness, h)),
            CornerRadius::same(0),
            color,
        );
        y += dash + gap;
    }
}

/// 30px connections stepper cell (`.cs-step`): slightly raised fill, hover
/// veil, `disabled` at the lower clamp (the old `opacity: 0.3`).
fn nd_step_btn(
    ui: &mut Ui,
    p: &Palette,
    r: Rect,
    id: &str,
    icon: &str,
    disabled: bool,
) -> bool {
    let rr = ui.interact(r, egui::Id::new(id), Sense::click());
    if rr.hovered() && !disabled {
        ui.painter().rect_filled(r, CornerRadius::same(0), p.hover_veil);
    }
    mi::paint_centered(
        ui.painter(),
        r.center(),
        icon,
        16.0,
        if disabled {
            p.text_dim.gamma_multiply(0.3)
        } else if rr.hovered() {
            p.text
        } else {
            p.text_muted
        },
    );
    rr.clicked() && !disabled
}

/// 1px `.cp-controls` / `.sp-controls` / `.pf-controls` top divider.
fn nd_card_divider(ui: &Ui, p: &Palette, card: Rect) {
    ui.painter().rect_filled(
        Rect::from_min_max(
            pos2(card.min.x + 14.0, card.min.y + 44.0),
            pos2(card.max.x - 14.0, card.min.y + 45.0),
        ),
        CornerRadius::same(0),
        p.border,
    );
}

/// Accent folder preview row (`.cp-preview` / `.sp-preview`).
fn nd_preview_row(ui: &Ui, p: &Palette, rect: Rect, icon: &str, text: &str) {
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 7.5, rect.center().y),
        icon,
        15.0,
        p.accent_strong,
    );
    ui.painter().with_clip_rect(rect).text(
        pos2(rect.min.x + 20.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(12.0, egui::FontFamily::default()),
        p.text_muted,
    );
}

/// Info-icon hint inside a card body (`.cp-hint` / `.sp-hint`).
fn nd_hint_at(ui: &Ui, p: &Palette, rect: Rect, text: &str) {
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 6.5, rect.center().y),
        "info",
        13.0,
        p.accent_strong,
    );
    ui.painter().with_clip_rect(rect).text(
        pos2(rect.min.x + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(11.0, egui::FontFamily::default()),
        p.text_dim,
    );
}

/// `CustomDropdown` port: a trigger box plus a foreground popup list. Returns
/// the picked value; an outside click closes the list (`handleClickOutside`).
fn nd_dropdown(
    ui: &mut Ui,
    p: &Palette,
    dd_open: &mut Option<String>,
    id: &str,
    rect: Rect,
    options: &[(String, String)],
    current: &str,
) -> Option<String> {
    let is_open = dd_open.as_deref() == Some(id);
    // Trigger.
    let rr = ui.interact(rect, egui::Id::new(id).with("trg"), Sense::click());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(10),
        p.hover_veil.gamma_multiply(0.5),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        egui::Stroke::new(
            1.0,
            if is_open { p.accent_strong } else { p.border_strong },
        ),
        egui::StrokeKind::Inside,
    );
    let label = options
        .iter()
        .find(|(v, _)| v == current)
        .map(|(_, l)| l.as_str())
        .unwrap_or("");
    let label_clip = Rect::from_min_max(
        pos2(rect.min.x + 14.0, rect.min.y),
        pos2(rect.max.x - 26.0, rect.max.y),
    );
    ui.painter().with_clip_rect(label_clip).text(
        pos2(label_clip.min.x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(13.0, egui::FontFamily::default()),
        p.text,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.max.x - 14.0, rect.center().y),
        if is_open { "expand_less" } else { "expand_more" },
        18.0,
        p.text_dim,
    );
    if rr.clicked() {
        *dd_open = if is_open { None } else { Some(id.to_owned()) };
    }
    // Popup.
    if is_open {
        let mut picked: Option<String> = None;
        let mut pop_rect = Rect::NOTHING;
        egui::Area::new(egui::Id::new(id).with("pop"))
            .order(egui::Order::Foreground)
            .fixed_pos([rect.min.x, rect.max.y + 4.0])
            .show(ui.ctx(), |ui| {
                ui.set_width(rect.width());
                egui::Frame::new()
                    .fill(p.bg_card)
                    .stroke(egui::Stroke::new(1.0, p.border_strong))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        nd_scroll_style(ui);
                        egui::ScrollArea::vertical()
                            .id_salt(("dd", id))
                            .auto_shrink([false, true])
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for (i, (value, text)) in options.iter().enumerate() {
                                    let (r, _) = ui.allocate_exact_size(
                                        vec2(ui.available_width(), 30.0),
                                        Sense::hover(),
                                    );
                                    let item_rr = ui.interact(
                                        r,
                                        egui::Id::new(id).with(("opt", i)),
                                        Sense::click(),
                                    );
                                    let active = value == current;
                                    if item_rr.hovered() || active {
                                        ui.painter().rect_filled(
                                            r,
                                            CornerRadius::same(7),
                                            if active {
                                                mk::blend(p.bg_card, p.accent, 0.12)
                                            } else {
                                                p.hover_veil
                                            },
                                        );
                                    }
                                    ui.painter().with_clip_rect(r).text(
                                        pos2(r.min.x + 8.0, r.center().y),
                                        Align2::LEFT_CENTER,
                                        text.clone(),
                                        FontId::new(13.0, egui::FontFamily::default()),
                                        if active { p.accent_strong } else { p.text },
                                    );
                                    if item_rr.clicked() {
                                        picked = Some(value.clone());
                                    }
                                }
                            });
                    });
                pop_rect = ui.min_rect();
            });
        if picked.is_some() {
            *dd_open = None;
            return picked;
        }
        // Outside press closes (skip the frame where the trigger toggled).
        let trigger_toggled = rr.clicked();
        if !trigger_toggled {
            let pressed = ui.ctx().input(|i| i.pointer.primary_pressed());
            let pos = ui.ctx().input(|i| i.pointer.interact_pos());
            if pressed {
                if let Some(pos) = pos {
                    if !pop_rect.contains(pos) {
                        *dd_open = None;
                    }
                }
            }
        }
    }
    None
}

// ─── Torrent intake ─────────────────────────────────────────────────────────

/// `pickTorrentFiles` — multi-select dialog, then queue each path.
fn nd_pick_torrents(state: &mut UiState, cctx: &Ctx) {
    if let Some(files) = rfd::FileDialog::new()
        .add_filter("Torrent", &["torrent"])
        .pick_files()
    {
        for f in files {
            let path = f.to_string_lossy().into_owned();
            nd_push_torrent(state, cctx, path);
        }
    }
}

/// `addTorrentFileEntry` — dedupe, push the loading card, parse off-thread.
fn nd_push_torrent(state: &mut UiState, cctx: &Ctx, path: String) {
    if state.nd_torrents.iter().any(|t| t.path == path) {
        return;
    }
    let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
    state.nd_torrents.push(NdTorrentFile {
        path: path.clone(),
        name,
        loading: true,
        ..Default::default()
    });
    cctx.run(move |app| async move {
        match moonhunt_core::aria2::parse_torrent_files(path.clone()) {
            Ok(files) => app.emit(
                "nd-torrent-files",
                serde_json::json!({ "path": path, "files": files }),
            ),
            Err(e) => app.emit(
                "nd-torrent-error",
                serde_json::json!({
                    "path": path,
                    "error": format!("Failed to read torrent contents: {e}"),
                }),
            ),
        }
        Ok::<String, String>(String::new())
    });
}

/// Global `.torrent` drop (`tauri://drag-drop`): queue the files and open the
/// dialog on the Torrent tab; non-torrent drops are skipped with a toast.
fn nd_drop_tick(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let files = ctx.input(|i| i.raw.dropped_files.clone());
    if files.is_empty() {
        return;
    }
    let mut skipped = 0usize;
    let mut queued = false;
    for f in files {
        let path = f.path();
        if path.as_os_str().is_empty() {
            continue;
        }
        let s = path.to_string_lossy();
        if s.to_lowercase().ends_with(".torrent") {
            nd_push_torrent(state, cctx, s.into_owned());
            queued = true;
        } else {
            skipped += 1;
        }
    }
    if queued {
        state.nd_torrent_tab = true;
        state.show_new_download = true;
    }
    if skipped > 0 {
        state.toast(
            false,
            state.t_of(&format!(
                "{} {} {}",
                state.raw("Skipped"),
                skipped,
                state.raw(if skipped == 1 {
                    "non-torrent file"
                } else {
                    "non-torrent files"
                })
            )),
        );
    }
}

// ─── Due-schedule poller ────────────────────────────────────────────────────

/// `MainLayout.checkDueSchedules` — every 10s, execute due rows.
fn nd_due_tick(state: &mut UiState, cctx: &Ctx) {
    if state.nd_due_poll.elapsed() < Duration::from_secs(10) {
        return;
    }
    state.nd_due_poll = Instant::now();
    cctx.run(move |_app| async move {
        if let Ok(due) = moonhunt_core::db::get_due_schedules() {
            for s in due {
                if let Some(id) = s.id {
                    let _ = moonhunt_core::db::execute_schedule(id).await;
                }
            }
        }
        Ok::<String, String>(String::new())
    });
}

// ─── Submit paths (addDownload / addTorrent) ────────────────────────────────

fn nd_submit(state: &mut UiState, cctx: &Ctx, tab: NdTab) {
    match tab {
        NdTab::Link => nd_submit_url(state, cctx),
        NdTab::Torrent => nd_submit_torrent(state, cctx),
    }
}

/// `addDownload` — validation, duplicate override, category dir resolution,
/// remembered connections, then the backend call; the `nd-add-done` event
/// closes the dialog on success.
fn nd_submit_url(state: &mut UiState, cctx: &Ctx) {
    if state.add_url.trim().is_empty() {
        state.toast(false, "Please enter a download URL");
        return;
    }
    if state.nd_cat_enabled && state.nd_cat_mode == 2 && state.nd_cat_selected.trim().is_empty() {
        state.toast(false, "Choose a manual category or turn Categories off.");
        return;
    }
    let use_dup = state.nd_duplicate_gid().is_some() && state.nd_allow_duplicate;
    if use_dup {
        state.toast(true, "Adding as duplicate");
    }
    state.nd_is_adding = true;

    // `baseDir = newPath || loadDefaultDir()`
    let base = state.nd_path.trim().to_owned();
    let base = if base.is_empty() {
        moonhunt_core::state::get_default_download_dir()
    } else {
        base
    };
    if state.nd_cat_enabled && base.is_empty() {
        state.toast(false, "Could not resolve the download folder.");
        state.nd_is_adding = false;
        return;
    }

    let detected = state.nd_detected_filename();
    let resolved = if !state.nd_filename.trim().is_empty() {
        state.nd_filename.trim().to_owned()
    } else if !detected.is_empty() {
        detected
    } else {
        state.add_url.clone()
    };
    let final_name = if use_dup {
        with_duplicate_suffix(&resolved, 1)
    } else {
        resolved
    };
    let category_dir = resolve_category_dir(
        state.nd_cat_enabled,
        state.nd_cat_mode,
        &state.nd_cat_selected,
        &final_name,
        &base,
    );
    let connections = state.nd_connections.max(1);
    // rememberRequestedConnections(url, connections)
    let url_norm = state.add_url.trim().to_lowercase();
    state.nd_conn_memory.retain(|(u, _)| *u != url_norm);
    state.nd_conn_memory.push((url_norm, connections));
    let filename_arg = if use_dup {
        Some(final_name)
    } else {
        let f = state.nd_filename.trim().to_owned();
        (!f.is_empty()).then_some(f)
    };
    let url = state.add_url.clone();
    let schedule = state.nd_schedule_value();
    let proxy = state.nd_proxy_value();
    // Extension-captured headers (cookies/UA); `None` for a hand-typed URL.
    let headers = state.nd_headers.take();
    cctx.run(move |app| async move {
        match moonhunt_core::aria2::add_download(
            app.clone(),
            url,
            filename_arg,
            Some(connections),
            category_dir,
            schedule,
            proxy,
            headers,
            None,
        )
        .await
        {
            Ok(resp) => {
                let duplicate = serde_json::from_str::<serde_json::Value>(&resp)
                    .ok()
                    .and_then(|v| v["duplicate"].as_bool())
                    .unwrap_or(false);
                app.emit(
                    "nd-add-done",
                    serde_json::json!({ "ok": true, "tab": "url" }),
                );
                Ok(if duplicate {
                    "Download added (file already existed at destination)".to_owned()
                } else {
                    "Download added!".to_owned()
                })
            }
            Err(e) => {
                app.emit(
                    "nd-add-done",
                    serde_json::json!({ "ok": false, "tab": "url" }),
                );
                Err(format!("Failed to add download: {e}"))
            }
        }
    });
}

/// `addTorrent` — one call per queued .torrent (with 1-based selected-file
/// indices) plus the optional magnet; the dialog only closes on success.
fn nd_submit_torrent(state: &mut UiState, cctx: &Ctx) {
    if state.nd_is_adding {
        return;
    }
    if state.nd_torrents.is_empty() && state.nd_magnet.trim().is_empty() {
        state.toast(false, "Provide at least one .torrent file or a magnet link");
        return;
    }
    state.nd_is_adding = true;
    let entries: Vec<(String, Option<Vec<u32>>)> = state
        .nd_torrents
        .iter()
        .map(|t| {
            let all = t.files.is_empty() || t.all_selected();
            let sel = if all {
                None
            } else {
                Some(
                    t.selected
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| **s)
                        .map(|(i, _)| (i + 1) as u32)
                        .collect(),
                )
            };
            (t.path.clone(), sel)
        })
        .collect();
    let magnet = state.nd_magnet.trim().to_owned();
    let dir_raw = state.nd_path.trim().to_owned();
    let dir = (!dir_raw.is_empty()).then_some(dir_raw);
    cctx.run(move |app| async move {
        let mut added = 0usize;
        for (path, sel) in &entries {
            match moonhunt_core::aria2::add_torrent(None, Some(path.clone()), dir.clone(), sel.clone())
                .await
            {
                Ok(r) => {
                    let ok = serde_json::from_str::<serde_json::Value>(&r)
                        .ok()
                        .and_then(|v| v.get("gid").cloned())
                        .map(|g| g.is_string())
                        .unwrap_or(true);
                    if ok {
                        added += 1;
                    }
                }
                Err(e) => {
                    app.emit(
                        "nd-add-done",
                        serde_json::json!({ "ok": false, "tab": "torrent" }),
                    );
                    return Err(format!("Failed to add torrent: {e}"));
                }
            }
        }
        if !magnet.is_empty() {
            match moonhunt_core::aria2::add_torrent(Some(magnet), None, dir.clone(), None).await {
                Ok(_) => added += 1,
                Err(e) => {
                    app.emit(
                        "nd-add-done",
                        serde_json::json!({ "ok": false, "tab": "torrent" }),
                    );
                    return Err(format!("Failed to add torrent: {e}"));
                }
            }
        }
        app.emit(
            "nd-add-done",
            serde_json::json!({ "ok": true, "tab": "torrent" }),
        );
        Ok(if added > 0 {
            "Torrents added!"
        } else {
            "Torrent added!"
        }
        .to_owned())
    });
}

// ─── ScheduleModal (`.sm-*`) ────────────────────────────────────────────────

fn nd_schedule_modal(state: &mut UiState, ctx: &egui::Context) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let mut cancel = false;
    let mut confirm = false;

    // Overlay: rgba(0,0,0,0.55), click outside closes.
    egui::Area::new(egui::Id::new("sm-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(140));
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                cancel = true;
            }
        });

    // Live validation: both halves must parse, and the time must be future.
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let epoch = state.nd_schedule_epoch_ms();
    let has_any = !state.nd_sched_date.trim().is_empty() || !state.nd_sched_time.trim().is_empty();
    state.nd_sched_error = if has_any {
        match epoch {
            None => state.t("Use YYYY-MM-DD and HH:MM").to_owned(),
            Some(ms) if ms <= now_ms => state.t("Pick a date & time in the future").to_owned(),
            _ => String::new(),
        }
    } else {
        String::new()
    };
    let valid = matches!(epoch, Some(ms) if ms > now_ms);

    egui::Area::new(egui::Id::new("sm-panel"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(SM_W);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    // Head: event icon + title + close.
                    let (h, _) = ui.allocate_exact_size(vec2(SM_W, 44.0), Sense::hover());
                    mi::paint_centered(
                        ui.painter(),
                        pos2(h.min.x + 14.0 + 8.0, h.center().y),
                        "event",
                        16.0,
                        p.accent_strong,
                    );
                    ui.painter().text(
                        pos2(h.min.x + 14.0 + 16.0 + 7.0, h.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Schedule"),
                        FontId::new(13.0, tokens::typography::semibold_family()),
                        p.text,
                    );
                    let close = egui::Rect::from_center_size(
                        pos2(h.max.x - 14.0 - 11.0, h.center().y),
                        vec2(22.0, 22.0),
                    );
                    let crr = ui.interact(close, egui::Id::new("sm-close"), Sense::click());
                    if crr.hovered() {
                        ui.painter()
                            .rect_filled(close, CornerRadius::same(5), p.hover_veil);
                    }
                    mi::paint_centered(
                        ui.painter(),
                        close.center(),
                        "close",
                        15.0,
                        if crr.hovered() { p.text } else { p.text_dim },
                    );
                    if crr.clicked() {
                        cancel = true;
                    }
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(h.min.x, h.max.y), vec2(SM_W, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );

                    // Body.
                    ui.add_space(12.0);
                    // Action
                    sm_label(ui, &p, state.t("Action"));
                    let r = row_pad(ui, 32.0, SM_W, SM_PAD);
                    if let Some(i) = nd_seg(
                        ui,
                        &p,
                        r,
                        "sm-action",
                        &[
                            state.t("Start"),
                            state.t("Pause"),
                            state.t("Resume"),
                        ],
                        state.nd_sched_action.min(2) as usize,
                    ) {
                        state.nd_sched_action = i as u8;
                    }
                    ui.add_space(10.0);
                    // Date & Time
                    sm_label(ui, &p, state.t("Date & Time"));
                    let r = row_pad(ui, 32.0, SM_W, SM_PAD);
                    let half = (r.width() - 8.0) / 2.0;
                    nd_input_at(
                        ui,
                        &p,
                        egui::Id::new("sm-date"),
                        Rect::from_min_size(r.min, vec2(half, 32.0)),
                        &mut state.nd_sched_date,
                        "YYYY-MM-DD",
                        None,
                        false,
                        false,
                    );
                    nd_input_at(
                        ui,
                        &p,
                        egui::Id::new("sm-time"),
                        Rect::from_min_size(pos2(r.min.x + half + 8.0, r.min.y), vec2(half, 32.0)),
                        &mut state.nd_sched_time,
                        "HH:MM",
                        None,
                        false,
                        false,
                    );
                    if !state.nd_sched_error.is_empty() {
                        let e = row_pad(ui, 16.0, SM_W, SM_PAD);
                        ui.painter().text(
                            pos2(e.min.x, e.center().y),
                            Align2::LEFT_CENTER,
                            state.nd_sched_error.clone(),
                            FontId::new(11.0, egui::FontFamily::default()),
                            p.status_error,
                        );
                    }
                    ui.add_space(10.0);
                    // Repeat
                    sm_label(ui, &p, state.t("Repeat"));
                    let r = row_pad(ui, 32.0, SM_W, SM_PAD);
                    if let Some(i) = nd_seg(
                        ui,
                        &p,
                        r,
                        "sm-repeat",
                        &[
                            state.t("Once"),
                            state.t("Hourly"),
                            state.t("Daily"),
                            state.t("Weekly"),
                        ],
                        state.nd_sched_repeat.min(3) as usize,
                    ) {
                        state.nd_sched_repeat = i as u8;
                    }
                    ui.add_space(12.0);

                    // Footer: Cancel ghost + Set Schedule accent.
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            pos2(ui.cursor().min.x, ui.cursor().min.y),
                            vec2(SM_W, 1.0),
                        ),
                        CornerRadius::same(0),
                        p.border,
                    );
                    let (f, _) = ui.allocate_exact_size(vec2(SM_W, 52.0), Sense::hover());
                    let set_w = 118.0;
                    let set_r = Rect::from_min_size(
                        pos2(f.max.x - 14.0 - set_w, f.min.y + 11.0),
                        vec2(set_w, 30.0),
                    );
                    if nd_button(
                        ui,
                        &p,
                        "sm-set",
                        set_r,
                        state.t("Set Schedule"),
                        Some("check"),
                        true,
                        valid,
                    ) {
                        confirm = true;
                    }
                    let cancel_r = Rect::from_min_size(
                        pos2(set_r.min.x - 6.0 - 70.0, f.min.y + 11.0),
                        vec2(70.0, 30.0),
                    );
                    if nd_button(
                        ui,
                        &p,
                        "sm-cancel",
                        cancel_r,
                        state.t("Cancel"),
                        None,
                        false,
                        true,
                    ) {
                        cancel = true;
                    }
                });
        });

    if !state.nd_catmgr_open {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            cancel = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) && valid {
            confirm = true;
        }
    }
    if confirm && valid {
        state.nd_sched_set = true;
        state.nd_sched_open = false;
    }
    if cancel {
        state.nd_sched_open = false;
    }
}

/// 10px uppercase `.sm-label` row.
fn sm_label(ui: &mut Ui, p: &Palette, text: &str) {
    let r = row_pad(ui, 14.0, SM_W, SM_PAD);
    ui.painter().text(
        pos2(r.min.x, r.center().y),
        Align2::LEFT_CENTER,
        text.to_uppercase(),
        FontId::new(10.0, tokens::typography::semibold_family()),
        p.text_dim,
    );
}

// ─── CategoryManagerModal (`.cm-*`) ─────────────────────────────────────────

/// One full reload through `loadDownloadCategories`' normalization rule.
fn nd_load_categories() -> Vec<String> {
    let raw = moonhunt_core::db::get_download_categories()
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut norm: Vec<String> = Vec::new();
    for c in raw {
        let f = sanitize_category_folder(&c);
        if !f.is_empty() && !norm.contains(&f) {
            norm.push(f);
        }
    }
    norm
}

/// `createCategory` — sanitize, append when new, persist, return the folder
/// (the old composable returns it even for duplicates, so it gets selected).
fn nd_cat_create(state: &mut UiState, cctx: &Ctx, raw: &str) -> Option<String> {
    let folder = sanitize_category_folder(raw);
    if folder.is_empty() {
        return None;
    }
    if !state.nd_categories.contains(&folder) {
        state.nd_categories.push(folder.clone());
        nd_cat_persist(cctx, state);
    }
    Some(folder)
}

/// `renameCategory` — returns the new folder (selects it) or None on failure.
fn nd_cat_rename(state: &mut UiState, cctx: &Ctx, old: &str, new: &str) -> Option<String> {
    let folder = sanitize_category_folder(new);
    let idx = state.nd_categories.iter().position(|c| c == old)?;
    if folder.is_empty() {
        return None;
    }
    if state.nd_categories.contains(&folder) {
        return if folder == old { Some(folder) } else { None };
    }
    state.nd_categories[idx] = folder.clone();
    nd_cat_persist(cctx, state);
    Some(folder)
}

/// `removeCategory` — drops the entry and persists; true when found.
fn nd_cat_remove(state: &mut UiState, cctx: &Ctx, name: &str) -> bool {
    let Some(idx) = state.nd_categories.iter().position(|c| c == name) else {
        return false;
    };
    state.nd_categories.remove(idx);
    nd_cat_persist(cctx, state);
    true
}

/// `persistToBackend` → `save_download_categories` (silent).
fn nd_cat_persist(cctx: &Ctx, state: &UiState) {
    let cats = state.nd_categories.clone();
    cctx.run(move |_| async move {
        let _ = moonhunt_core::db::save_download_categories(cats).await;
        Ok::<String, String>(String::new())
    });
}

fn nd_category_manager(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    const CM_W: f32 = 320.0;
    let mut close = false;
    let mut want_add = false;
    let mut want_select: Option<String> = None;
    let mut want_rename_save = false;
    let mut want_rename_cancel = false;
    let mut want_start_rename: Option<usize> = None;
    let mut want_delete: Option<usize> = None;

    // Overlay: rgba(0,0,0,0.55), click outside closes.
    egui::Area::new(egui::Id::new("cm-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(140));
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                close = true;
            }
        });

    egui::Area::new(egui::Id::new("cm-panel"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .corner_radius(CornerRadius::same(14))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(CM_W);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    // Head.
                    let (h, _) = ui.allocate_exact_size(vec2(CM_W, 44.0), Sense::hover());
                    mi::paint_centered(
                        ui.painter(),
                        pos2(h.min.x + 14.0 + 8.5, h.center().y),
                        "folder_special",
                        17.0,
                        p.accent_strong,
                    );
                    ui.painter().text(
                        pos2(h.min.x + 14.0 + 17.0 + 7.0, h.center().y),
                        Align2::LEFT_CENTER,
                        state.t("Manual Categories"),
                        FontId::new(13.0, tokens::typography::semibold_family()),
                        p.text,
                    );
                    let x = egui::Rect::from_center_size(
                        pos2(h.max.x - 14.0 - 12.0, h.center().y),
                        vec2(24.0, 24.0),
                    );
                    let xrr = ui.interact(x, egui::Id::new("cm-close"), Sense::click());
                    if xrr.hovered() {
                        ui.painter()
                            .rect_filled(x, CornerRadius::same(5), p.hover_veil);
                    }
                    mi::paint_centered(
                        ui.painter(),
                        x.center(),
                        "close",
                        15.0,
                        if xrr.hovered() { p.text } else { p.text_dim },
                    );
                    if xrr.clicked() {
                        close = true;
                    }
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(h.min.x, h.max.y), vec2(CM_W, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );

                    // Body (padding 14/12, gap 10).
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        // Add row: input + Add button.
                        let (row, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                        let btn_w = 58.0;
                        let in_r = Rect::from_min_size(
                            row.min,
                            vec2(row.width() - btn_w - 6.0, 30.0),
                        );
                        let btn_r = Rect::from_min_size(
                            pos2(row.max.x - btn_w, row.min.y),
                            vec2(btn_w, 30.0),
                        );
                        let cm_new_ph = state.t("New category folder...");
                        nd_input_at(
                            ui,
                            &p,
                            egui::Id::new("cm-new"),
                            in_r,
                            &mut state.nd_catmgr_new,
                            cm_new_ph,
                            None,
                            false,
                            state.nd_catmgr_edit.is_none(),
                        );
                        let add_ok = !state.nd_catmgr_new.trim().is_empty();
                        let add_rr = ui.interact(btn_r, egui::Id::new("cm-add"), Sense::click());
                        let add_fill = if add_ok {
                            p.accent
                        } else {
                            mk::blend(p.accent, p.bg_card, 0.65)
                        };
                        ui.painter()
                            .rect_filled(btn_r, CornerRadius::same(7), add_fill);
                        {
                            let font =
                                FontId::new(11.0, tokens::typography::semibold_family());
                            let add_label = state.t("Add");
                            let lw = ui
                                .painter()
                                .layout_no_wrap(add_label.to_owned(), font.clone(), Color32::WHITE)
                                .rect
                                .width();
                            let gx = btn_r.center().x - (17.0 + lw) * 0.5;
                            mi::paint_centered(
                                ui.painter(),
                                pos2(gx + 7.0, btn_r.center().y),
                                "add",
                                14.0,
                                Color32::from_rgb(0x0a, 0x0e, 0x1a),
                            );
                            ui.painter().text(
                                pos2(gx + 17.0, btn_r.center().y),
                                Align2::LEFT_CENTER,
                                add_label,
                                font,
                                Color32::from_rgb(0x0a, 0x0e, 0x1a),
                            );
                        }
                        if add_rr.clicked() && add_ok {
                            want_add = true;
                        }
                        ui.add_space(10.0);

                        if state.nd_categories.is_empty() {
                            // `.cm-empty`
                            let (r, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 74.0),
                                Sense::hover(),
                            );
                            mi::paint_centered(
                                ui.painter(),
                                pos2(r.center().x, r.min.y + 22.0),
                                "create_new_folder",
                                26.0,
                                p.text_dim.gamma_multiply(0.6),
                            );
                            ui.painter().text(
                                pos2(r.center().x, r.min.y + 52.0),
                                Align2::CENTER_CENTER,
                                state.t("Create your first category folder above."),
                                FontId::new(12.0, egui::FontFamily::default()),
                                p.text_dim,
                            );
                        } else {
                            nd_scroll_style(ui);
                            egui::ScrollArea::vertical()
                                .id_salt("cm-list")
                                .auto_shrink([false, true])
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = vec2(0.0, 6.0);
                                    for i in 0..state.nd_categories.len() {
                                        let category = state.nd_categories[i].clone();
                                        let (r, _) = ui.allocate_exact_size(
                                            vec2(ui.available_width(), 34.0),
                                            Sense::hover(),
                                        );
                                        let active = category == state.nd_cat_selected;
                                        ui.painter().rect_filled(
                                            r,
                                            CornerRadius::same(8),
                                            if active {
                                                mk::blend(p.bg_card, p.accent, 0.06)
                                            } else {
                                                p.hover_veil.gamma_multiply(0.5)
                                            },
                                        );
                                        if active {
                                            ui.painter().rect_stroke(
                                                r,
                                                CornerRadius::same(8),
                                                egui::Stroke::new(
                                                    1.0,
                                                    mk::blend(p.border, p.accent, 0.35),
                                                ),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        if state.nd_catmgr_edit == Some(i) {
                                            // Rename row: input + save/cancel.
                                            let save_r = Rect::from_min_size(
                                                pos2(r.max.x - 6.0 - 24.0, r.min.y + 2.0),
                                                vec2(24.0, 30.0),
                                            );
                                            let cancel_r = Rect::from_min_size(
                                                pos2(save_r.min.x - 3.0 - 24.0, r.min.y + 2.0),
                                                vec2(24.0, 30.0),
                                            );
                                            let in_r = Rect::from_min_size(
                                                pos2(r.min.x + 4.0, r.min.y + 2.0),
                                                vec2(
                                                    cancel_r.min.x - 6.0 - (r.min.x + 4.0),
                                                    30.0,
                                                ),
                                            );
                                            nd_input_at(
                                                ui,
                                                &p,
                                                egui::Id::new("cm-rename"),
                                                in_r,
                                                &mut state.nd_catmgr_edit_value,
                                                "",
                                                None,
                                                false,
                                                false,
                                            );
                                            let sv = ui.interact(
                                                save_r,
                                                egui::Id::new("cm-rename-save"),
                                                Sense::click(),
                                            );
                                            if sv.hovered() {
                                                ui.painter().rect_filled(
                                                    save_r,
                                                    CornerRadius::same(6),
                                                    p.hover_veil,
                                                );
                                            }
                                            mi::paint_centered(
                                                ui.painter(),
                                                save_r.center(),
                                                "check",
                                                14.0,
                                                p.accent_strong,
                                            );
                                            if sv.clicked() {
                                                want_rename_save = true;
                                            }
                                            sv.on_hover_text(state.t("Save rename"));
                                            let cv = ui.interact(
                                                cancel_r,
                                                egui::Id::new("cm-rename-cancel"),
                                                Sense::click(),
                                            );
                                            if cv.hovered() {
                                                ui.painter().rect_filled(
                                                    cancel_r,
                                                    CornerRadius::same(6),
                                                    p.hover_veil,
                                                );
                                            }
                                            mi::paint_centered(
                                                ui.painter(),
                                                cancel_r.center(),
                                                "close",
                                                14.0,
                                                p.text_dim,
                                            );
                                            if cv.clicked() {
                                                want_rename_cancel = true;
                                            }
                                            cv.on_hover_text(state.t("Cancel rename"));
                                        } else {
                                            // Row: folder + name + actions.
                                            mi::paint_centered(
                                                ui.painter(),
                                                pos2(r.min.x + 9.0 + 7.5, r.center().y),
                                                "folder",
                                                15.0,
                                                p.accent_strong,
                                            );
                                            let del_r = Rect::from_min_size(
                                                pos2(r.max.x - 6.0 - 24.0, r.min.y + 5.0),
                                                vec2(24.0, 24.0),
                                            );
                                            let edit_r = Rect::from_min_size(
                                                pos2(del_r.min.x - 3.0 - 24.0, r.min.y + 5.0),
                                                vec2(24.0, 24.0),
                                            );
                                            let sel_r = Rect::from_min_size(
                                                pos2(edit_r.min.x - 3.0 - 46.0, r.min.y + 5.0),
                                                vec2(46.0, 24.0),
                                            );
                                            let name_clip = Rect::from_min_max(
                                                pos2(r.min.x + 9.0 + 15.0 + 6.0, r.min.y),
                                                pos2(sel_r.min.x - 6.0, r.max.y),
                                            );
                                            ui.painter().with_clip_rect(name_clip).text(
                                                pos2(name_clip.min.x, r.center().y),
                                                Align2::LEFT_CENTER,
                                                category.clone(),
                                                FontId::new(
                                                    12.0,
                                                    tokens::typography::semibold_family(),
                                                ),
                                                p.text,
                                            );
                                            // Select.
                                            let srr = ui.interact(
                                                sel_r,
                                                egui::Id::new(("cm-select", i)),
                                                Sense::click(),
                                            );
                                            ui.painter().rect_filled(
                                                sel_r,
                                                CornerRadius::same(6),
                                                mk::blend(p.bg_card, p.accent, 0.12),
                                            );
                                            ui.painter().text(
                                                sel_r.center(),
                                                Align2::CENTER_CENTER,
                                                state.t("Select"),
                                                FontId::new(
                                                    10.5,
                                                    tokens::typography::semibold_family(),
                                                ),
                                                p.accent_strong,
                                            );
                                            if srr.clicked() {
                                                want_select = Some(category.clone());
                                            }
                                            // Rename.
                                            let err = ui.interact(
                                                edit_r,
                                                egui::Id::new(("cm-rename", i)),
                                                Sense::click(),
                                            );
                                            if err.hovered() {
                                                ui.painter().rect_filled(
                                                    edit_r,
                                                    CornerRadius::same(6),
                                                    p.border_strong,
                                                );
                                            }
                                            mi::paint_centered(
                                                ui.painter(),
                                                edit_r.center(),
                                                "edit",
                                                14.0,
                                                if err.hovered() { p.text } else { p.text_dim },
                                            );
                                            if err.clicked() {
                                                want_start_rename = Some(i);
                                            }
                                            // Delete.
                                            let drr = ui.interact(
                                                del_r,
                                                egui::Id::new(("cm-delete", i)),
                                                Sense::click(),
                                            );
                                            if drr.hovered() {
                                                ui.painter().rect_filled(
                                                    del_r,
                                                    CornerRadius::same(6),
                                                    Color32::from_rgba_unmultiplied(
                                                        239, 83, 80, 31,
                                                    ),
                                                );
                                            }
                                            mi::paint_centered(
                                                ui.painter(),
                                                del_r.center(),
                                                "delete",
                                                14.0,
                                                if drr.hovered() {
                                                    p.status_error
                                                } else {
                                                    p.text_dim
                                                },
                                            );
                                            if drr.clicked() {
                                                want_delete = Some(i);
                                            }
                                        }
                                    }
                                });
                        }
                    });
                });
        });

    // Keyboard: Enter adds (or saves a rename); Esc cancels a rename, else
    // closes the manager.
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        if state.nd_catmgr_edit.is_some() {
            want_rename_save = true;
        } else if !state.nd_catmgr_new.trim().is_empty() {
            want_add = true;
        }
    }
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        if state.nd_catmgr_edit.is_some() {
            want_rename_cancel = true;
        } else {
            close = true;
        }
    }

    if let Some(i) = want_start_rename {
        state.nd_catmgr_edit = Some(i);
        state.nd_catmgr_edit_value = state
            .nd_categories
            .get(i)
            .cloned()
            .unwrap_or_default();
    }
    if want_rename_cancel {
        state.nd_catmgr_edit = None;
        state.nd_catmgr_edit_value.clear();
    }
    if want_rename_save {
        if let Some(i) = state.nd_catmgr_edit {
            let old = state.nd_categories.get(i).cloned().unwrap_or_default();
            let new = state.nd_catmgr_edit_value.clone();
            if let Some(folder) = nd_cat_rename(state, cctx, &old, &new) {
                if state.nd_cat_selected == old {
                    state.nd_cat_selected = folder;
                }
            }
            state.nd_catmgr_edit = None;
            state.nd_catmgr_edit_value.clear();
        }
    }
    if let Some(name) = want_select {
        state.nd_cat_selected = name;
        state.nd_cat_error.clear();
        close = true;
    }
    if let Some(i) = want_delete {
        let name = state.nd_categories.get(i).cloned().unwrap_or_default();
        if nd_cat_remove(state, cctx, &name) && state.nd_cat_selected == name {
            state.nd_cat_selected.clear();
        }
        if state.nd_catmgr_edit == Some(i) {
            state.nd_catmgr_edit = None;
            state.nd_catmgr_edit_value.clear();
        }
    }
    if want_add {
        if let Some(folder) = nd_cat_create(state, cctx, state.nd_catmgr_new.trim().to_owned().as_str())
        {
            state.nd_cat_selected = folder;
            state.nd_cat_error.clear();
            state.nd_catmgr_new.clear();
        }
    }
    if close {
        state.nd_catmgr_open = false;
        state.nd_catmgr_edit = None;
        state.nd_catmgr_edit_value.clear();
    }
}

// ─── Download details modal (`.details-modal-new` / `.dm-*`) ────────────────

/// Fixed shell heights. The old CSS gives every band an intrinsic height:
/// header `13px 20px` padding around a 44px avatar + 1px rule, tabs
/// `11px 14px` around a 17.5px line + 2.5px underline + 1px rule, footer
/// `10px 20px` around a 17px line + 1px rule.
const DM_HEADER_H: f32 = 71.0;
const DM_TABS_H: f32 = 43.0;
const DM_FOOTER_H: f32 = 59.5;
/// `.dm-table th { width: 130px }`.
const DM_TH_W: f32 = 130.0;
/// `.dm-table tr` heights: plain text row, chip/pill row (12px × 1.4 line +
/// 3px × 2 padding), row holding a 34px `.dm-icon-btn`.
const DM_ROW_H: f32 = 37.5;
const DM_ROW_H_CHIP: f32 = 43.0;
const DM_ROW_H_BTN: f32 = 55.0;
/// `.dm-file-table tr` (10px cell padding around the 28px `.sm` path button)
/// and its `thead` (9px padding around an 11px line).
const DM_FILE_ROW_H: f32 = 49.0;
const DM_FILE_HEAD_H: f32 = 32.0;
/// `.dm-body { padding: 16px 20px 12px }` — horizontal inset of every row.
const DM_PAD_X: f32 = 20.0;
/// `PATH_PREVIEW_MAX` — path previews never exceed 18 characters.
const DM_PATH_MAX: usize = 18;
/// `.pf-field` internals: `.pf-sub` label line, `.pf-input` height, gap 6.
const DM_FIELD_LABEL_H: f32 = 15.0;
const DM_INPUT_H: f32 = 32.0;
const DM_FIELD_H: f32 = DM_FIELD_LABEL_H + 6.0 + DM_INPUT_H;

/// Galley metrics of a single line. egui has no `Ui::fonts`, so measuring
/// goes through the painter's layout cache (`layout_no_wrap`).
fn dm_text_size(ui: &Ui, text: &str, font: &FontId) -> (f32, f32) {
    let g = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
    (g.rect.width(), g.rect.height())
}

/// Width of one un-wrapped line (used to center icon+label groups).
fn dm_text_w(ui: &Ui, text: &str, font: &FontId) -> f32 {
    dm_text_size(ui, text, font).0
}

/// CSS `text-overflow: ellipsis`: longest prefix of `s` that still fits
/// `max_w` next to the "…" tail. Binary-searched on character boundaries —
/// while `lo < hi` the midpoint is always ≥ 1, so `mid - 1` never underflows.
fn dm_ellipsize(ui: &Ui, s: &str, font: &FontId, max_w: f32) -> String {
    if max_w <= 0.0 {
        return String::new();
    }
    if dm_text_w(ui, s, font) <= max_w {
        return s.to_owned();
    }
    let dots = dm_text_w(ui, "...", font);
    if dots >= max_w {
        return String::new();
    }
    crate::rtl::truncate_with(s, "...", |s| dm_text_w(ui, s, font) + dots <= max_w)
}

/// `.dm-icon-btn` — 34×34 (28×28 `.sm`) transparent button: `text-dim`
/// glyph that flips to `text` over a `hover-bg` veil.
fn dm_icon_btn(ui: &mut Ui, p: &Palette, rect: Rect, icon: &str, id: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    if rr.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    }
    // `.dm-icon-btn i { font-size: 18px }`, `.sm i { 15px }` — inferred from
    // the button width so one helper serves both sizes.
    let size = if rect.width() <= 28.5 { 15.0 } else { 18.0 };
    mi::paint_centered(
        ui.painter(),
        rect.center(),
        icon,
        size,
        if rr.hovered() { p.text } else { p.text_dim },
    );
    rr.clicked()
}

/// `getFileIcon()` — extension bucket → Material icon.
fn dm_file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "mp3" | "flac" | "wav" | "m4a" | "ogg" => "movie",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "archive",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" => "image",
        "pdf" | "doc" | "docx" | "txt" | "md" | "rtf" | "odt" => "description",
        "js" | "ts" | "py" | "rs" | "go" | "cpp" | "c" | "h" | "java" | "cs" | "php" | "rb" => {
            "code"
        }
        _ => "insert_drive_file",
    }
}

/// `getStatusIcon()` — the old map had no entry for aria2's `removed` and
/// fell back to `fiber_manual_record`; we use `cancel` instead so the sixth
/// state keeps a distinct glyph (documented deviation).
fn dm_status_icon(s: Status) -> &'static str {
    match s {
        Status::Active => "play_circle",
        Status::Complete => "check_circle",
        Status::Paused => "pause_circle",
        Status::Error => "error",
        Status::Waiting => "schedule",
        Status::Removed => "cancel",
    }
}

/// `capitalizeStatus()` — the translated labels (note `complete → Completed`
/// and `error → Failed`; `Status::label()` keeps aria2's vocabulary instead).
fn dm_status_label(s: Status) -> &'static str {
    match s {
        Status::Active => "Active",
        Status::Complete => "Completed",
        Status::Paused => "Paused",
        Status::Error => "Failed",
        Status::Waiting => "Waiting",
        Status::Removed => "Removed",
    }
}

/// Full-width flow row of `h` (leaf bands: header/tabs/footer).
fn dm_row(ui: &mut Ui, h: f32) -> Rect {
    let w = ui.available_width();
    ui.allocate_exact_size(vec2(w, h), Sense::hover()).0
}

/// `.dm-body` row: full modal width, inset by the 20px side padding.
fn dm_body_row(ui: &mut Ui, h: f32) -> Rect {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    Rect::from_min_size(
        pos2(r.min.x + DM_PAD_X, r.min.y),
        vec2(w - 2.0 * DM_PAD_X, h),
    )
}

/// `.dm-section-title` — 11px bold uppercase, `text-dim` at 75% opacity.
/// egui has no letter-spacing, so the `.06em` tracking is dropped here and
/// in `.dm-file-table th` (`.04em`).
fn dm_section_title(ui: &mut Ui, p: &Palette, title: &str, mb: f32) {
    dm_section_title_icon(ui, p, None, title, mb);
}

/// Same, with the optional leading glyph (`.dm-error-section i`, 15px, gap 6).
fn dm_section_title_icon(
    ui: &mut Ui,
    p: &Palette,
    icon: Option<(&str, Color32)>,
    title: &str,
    mb: f32,
) {
    let r = dm_body_row(ui, 15.0);
    let mut x = r.min.x;
    if let Some((ic, col)) = icon {
        mi::paint_centered(ui.painter(), pos2(r.min.x + 7.5, r.center().y), ic, 15.0, col);
        x = r.min.x + 21.0;
    }
    ui.painter().text(
        pos2(x, r.center().y),
        Align2::LEFT_CENTER,
        title.to_uppercase(),
        FontId::new(11.0, tokens::typography::bold_family()),
        p.text_dim.gamma_multiply(0.75),
    );
    if mb > 0.0 {
        // `margin: 0 0 10px` (4px on `.dm-debug-title`).
        dm_body_row(ui, mb);
    }
}

/// `.dm-section { margin-bottom: 20px }` — omitted after the last section.
fn dm_section_gap(ui: &mut Ui) {
    dm_body_row(ui, 20.0);
}

/// `.dm-table tr` — 130px `th` on a `hover-bg` wash, transparent `td`, and a
/// `rgba(accent,0.06)` rule under every row but the last. Returns both text
/// boxes after the CSS `10px 14px` cell padding.
fn dm_tr(
    ui: &mut Ui,
    p: &Palette,
    cw: f32,
    label: &str,
    h: f32,
    first: bool,
    last: bool,
) -> (Rect, Rect) {
    let _ = (first, cw); // the wash is a plain rect; `first` only documents CSS
    let r = dm_body_row(ui, h);
    let painter = ui.painter();
    // `.dm-table th { background: var(--hover-bg) }` over the fixed column.
    let th = Rect::from_min_max(r.min, pos2(r.min.x + DM_TH_W, r.max.y));
    painter.rect_filled(th, CornerRadius::same(0), p.hover_veil);
    // `.dm-table tr:last-child` drops the rule.
    if !last {
        painter.rect_filled(
            Rect::from_min_max(pos2(r.min.x, r.max.y - 1.0), r.max),
            CornerRadius::same(0),
            mk::blend(p.bg_card, p.accent, 0.06),
        );
    }
    // `padding: 10px 14px` on both cells; the 1px rule eats the bottom pad.
    let bottom_pad = if last { 10.0 } else { 11.0 };
    let th_text = Rect::from_min_max(
        pos2(th.min.x + 14.0, th.min.y + 10.0),
        pos2(th.max.x - 14.0, th.max.y - bottom_pad),
    );
    // The `td` box gets its own `0 14px` padding after the `th` column.
    let td = Rect::from_min_max(
        pos2(th.max.x + 14.0, th.min.y + 10.0),
        pos2(r.max.x - 14.0, r.max.y - bottom_pad),
    );
    // `.dm-table th` label: 13px semibold `text-dim`, vertically centered in
    // the cell box (the browser centers it in the line box instead).
    painter.text(
        pos2(th_text.min.x, th_text.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(13.0, tokens::typography::semibold_family()),
        p.text_dim,
    );
    (th_text, td)
}

/// `.dm-chip` / `.dm-priority` / `.dm-source-tag` / `.dm-status-pill` — a
/// fully rounded inline label anchored by its left-center point (table cells
/// center their content vertically, start it at the left like `inline-flex`).
struct DmPill {
    pad_x: f32,
    pad_y: f32,
    gap: f32,
    /// `(icon, size)` — the CSS metrics differ per pill (14–16px).
    icon: Option<(&'static str, f32)>,
    font: FontId,
    fg: Color32,
    /// Solid fill; `None` keeps the card background showing through.
    bg: Option<Color32>,
}

/// Width a `DmPill` would occupy (for right-aligning a group of them).
fn dm_pill_w(ui: &Ui, label: &str, pill: &DmPill) -> f32 {
    let tw = dm_text_w(ui, label, &pill.font);
    let iw = pill.icon.map(|(_, s)| s + pill.gap).unwrap_or(0.0);
    pill.pad_x * 2.0 + iw + tw
}

/// Paint a `DmPill` with its left edge / vertical center at `at`.
fn dm_pill(ui: &Ui, at: Pos2, label: &str, pill: &DmPill) -> Rect {
    let painter = ui.painter();
    let (tw, th) = dm_text_size(ui, label, &pill.font);
    let (iw, ih) = match pill.icon {
        Some((_, s)) => (s + pill.gap, s),
        None => (0.0, 0.0),
    };
    let w = pill.pad_x * 2.0 + iw + tw;
    let h = pill.pad_y * 2.0 + th.max(ih);
    let rect = Rect::from_min_size(pos2(at.x, at.y - h * 0.5), vec2(w, h));
    if let Some(bg) = pill.bg {
        painter.rect_filled(rect, CornerRadius::same((h * 0.5) as u8), bg);
    }
    let mut x = rect.min.x + pill.pad_x;
    let cy = rect.center().y;
    if let Some((ic, s)) = pill.icon {
        mi::paint_centered(painter, pos2(x + s * 0.5, cy), ic, s, pill.fg);
        x += s + pill.gap;
    }
    painter.text(
        pos2(x, cy),
        Align2::LEFT_CENTER,
        label,
        pill.font.clone(),
        pill.fg,
    );
    rect
}

/// `.dm-progress-cell` — track (max 160×6, r3) + `min-width: 38px` pct.
fn dm_progress(ui: &Ui, p: &Palette, rect: Rect, pct: f32) {
    let pct = pct.clamp(0.0, 100.0);
    let font = FontId::new(13.0, tokens::typography::semibold_family());
    let label = format!("{pct:.0}%");
    let lw = dm_text_w(ui, &label, &font);
    let gap = 10.0;
    let track_w = (rect.width() - gap - lw.max(38.0)).clamp(0.0, 160.0);
    let cy = rect.center().y;
    let track = Rect::from_min_size(pos2(rect.min.x, cy - 3.0), vec2(track_w, 6.0));
    ui.painter()
        .rect_filled(track, CornerRadius::same(3), mk::blend(p.bg_card, p.accent, 0.10));
    let fill = track_w * pct / 100.0;
    if fill > 0.0 {
        // `.dm-progress-fill` is `linear-gradient(accent → accent .7)`; egui
        // shapes carry a flat fill, so the leading tone is used.
        ui.painter().rect_filled(
            Rect::from_min_max(track.min, pos2(track.min.x + fill, track.max.y)),
            CornerRadius::same(3),
            p.accent,
        );
    }
    ui.painter()
        .text(pos2(track.max.x + gap, cy), Align2::LEFT_CENTER, label, font, p.text);
}

/// `.dm-mini-progress` — 60×4 track + 11.5px dim pct (file table).
fn dm_mini_progress(ui: &Ui, p: &Palette, rect: Rect, ratio: f32) {
    let font = FontId::new(11.5, egui::FontFamily::default());
    let label = format!("{:.1}%", ratio.clamp(0.0, 1.0) * 100.0);
    let lw = dm_text_w(ui, &label, &font);
    let gap = 8.0;
    let cy = rect.center().y;
    let track = Rect::from_min_size(pos2(rect.min.x, cy - 2.0), vec2(60.0, 4.0));
    ui.painter()
        .rect_filled(track, CornerRadius::same(2), mk::blend(p.bg_card, p.accent, 0.10));
    let fill = track.width() * ratio.clamp(0.0, 1.0);
    if fill > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_max(track.min, pos2(track.min.x + fill, track.max.y)),
            CornerRadius::same(2),
            p.accent,
        );
    }
    ui.painter().text(
        pos2(track.max.x + gap, cy),
        Align2::LEFT_CENTER,
        label,
        font,
        p.text_dim,
    );
    let _ = lw;
}

/// `.dm-empty-state` — dashed box, 28px dim glyph, message, optional hint.
fn dm_empty_state(
    ui: &mut Ui,
    p: &Palette,
    icon: &str,
    msg: &str,
    hint: Option<&str>,
) -> Rect {
    let w = ui.available_width();
    let h = 20.0 + 28.0 + 8.0 + 17.5 + if hint.is_some() { 4.0 + 15.5 } else { 0.0 } + 20.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(10), p.hover_veil);
    nd_dashed_rect(ui.painter(), r, p.border, 1.5);
    mi::paint_centered(
        ui.painter(),
        pos2(r.center().x, r.min.y + 20.0 + 14.0),
        icon,
        28.0,
        p.text_dim.gamma_multiply(0.5),
    );
    let mut y = r.min.y + 20.0 + 28.0 + 8.0;
    ui.painter().text(
        pos2(r.center().x, y + 8.75),
        Align2::CENTER_CENTER,
        msg,
        FontId::new(14.0, tokens::typography::semibold_family()),
        p.text_dim,
    );
    y += 17.5;
    if let Some(hint) = hint {
        ui.painter().text(
            pos2(r.center().x, y + 4.0 + 7.75),
            Align2::CENTER_CENTER,
            hint,
            FontId::new(12.0, egui::FontFamily::default()),
            p.text_dim.gamma_multiply(0.6),
        );
    }
    r
}

/// `.dm-empty` — borderless variant (36px glyph at 40% opacity, no hint).
fn dm_empty(ui: &mut Ui, p: &Palette, icon: &str, msg: &str) -> Rect {
    let w = ui.available_width();
    let h = 40.0 + 36.0 + 8.0 + 16.5 + 40.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    mi::paint_centered(
        ui.painter(),
        pos2(r.center().x, r.min.y + 40.0 + 18.0),
        icon,
        36.0,
        p.text_dim.gamma_multiply(0.4),
    );
    ui.painter().text(
        pos2(r.center().x, r.min.y + 40.0 + 36.0 + 8.0 + 8.25),
        Align2::CENTER_CENTER,
        msg,
        FontId::new(13.0, egui::FontFamily::default()),
        p.text_dim,
    );
    r
}

/// `.dm-debug-table tr` — 90px uppercase transparent `th`, 11.5px `td`,
/// `3px 8px 3px 0` padding, no rules.
fn dm_debug_row(ui: &mut Ui, p: &Palette, label: &str, value: &str) {
    let r = dm_body_row(ui, 22.0);
    ui.painter().text(
        pos2(r.min.x, r.center().y),
        Align2::LEFT_CENTER,
        label.to_uppercase(),
        FontId::new(10.5, tokens::typography::semibold_family()),
        p.text_dim,
    );
    ui.painter().text(
        pos2(r.min.x + 90.0, r.center().y),
        Align2::LEFT_CENTER,
        value,
        FontId::new(11.5, egui::FontFamily::default()),
        p.text,
    );
}

/// `.dm-log-health` — the 24-unit SVG ring (r 10, stroke 3, `dasharray
/// 62.83`, rotated −90°) plus its pct/label column. Returns the 22px ring
/// width so the caller can place following text.
fn dm_health_ring(ui: &Ui, p: &Palette, at: Pos2, health: f32, errors: i64) -> f32 {
    const S: f32 = 22.0;
    let r = S * 10.0 / 24.0;
    let stroke = S * 3.0 / 24.0;
    let c = pos2(at.x + S * 0.5, at.y + S * 0.5);
    ui.painter()
        .circle_stroke(c, r, egui::Stroke::new(stroke, p.border));
    let health = health.clamp(0.0, 1.0);
    if health > 0.0 {
        // `.dm-log-circle.good/.warn/.bad` thresholds (0.8 / 0.5).
        let color = if health > 0.8 {
            Color32::from_rgb(0x22, 0xc5, 0x5e)
        } else if health > 0.5 {
            Color32::from_rgb(0xea, 0xb3, 0x08)
        } else {
            Color32::from_rgb(0xef, 0x44, 0x44)
        };
        // `stroke-dashoffset: 62.83 × (1 − health)` from −90°: the visible arc
        // spans `TAU × health`, tessellated as a polyline (round caps ≈ the
        // SVG's `stroke-linecap: round`).
        let steps = ((health * 64.0).ceil() as usize).max(1);
        let span = std::f32::consts::TAU * health;
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + span * (i as f32) / steps as f32;
                pos2(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect();
        ui.painter()
            .add(egui::Shape::line(pts, egui::Stroke::new(stroke, color)));
    }
    let x0 = at.x + S + 6.0;
    let cy = at.y + S * 0.5;
    ui.painter().text(
        pos2(x0, cy - 6.0),
        Align2::LEFT_CENTER,
        format!("{}%", (health * 100.0).round() as i64),
        FontId::new(10.0, tokens::typography::bold_family()),
        p.text,
    );
    // `logPacketLoss.errors > 0 ? 'Timeouts' : 'Packets'`.
    ui.painter().text(
        pos2(x0, cy + 7.0),
        Align2::LEFT_CENTER,
        if errors > 0 { "Timeouts" } else { "Packets" },
        FontId::new(11.0, egui::FontFamily::default()),
        p.text_dim,
    );
    S
}

/// `.dm-log-badge` — `(foreground, background)` for one `eventType`. The
/// background is the CSS color washed to 15% over the card.
fn dm_log_badge(p: &Palette, ty: &str) -> (Color32, Color32) {
    let (fg, base) = match ty {
        "state_change" => (
            Color32::from_rgb(0x60, 0xa5, 0xfa),
            Color32::from_rgb(0x3b, 0x82, 0xf6),
        ),
        "error" => (
            Color32::from_rgb(0xf8, 0x71, 0x71),
            Color32::from_rgb(0xef, 0x44, 0x44),
        ),
        "timeout" => (
            Color32::from_rgb(0xfb, 0x92, 0x3c),
            Color32::from_rgb(0xfb, 0x92, 0x3c),
        ),
        "speed_sample" => (
            Color32::from_rgb(0x4a, 0xde, 0x80),
            Color32::from_rgb(0x22, 0xc5, 0x5e),
        ),
        "retry" => (
            Color32::from_rgb(0xfa, 0xcc, 0x15),
            Color32::from_rgb(0xea, 0xb3, 0x08),
        ),
        _ => (
            Color32::from_rgb(0x94, 0xa3, 0xb8),
            Color32::from_rgb(0x94, 0xa3, 0xb8),
        ),
    };
    (fg, mk::blend(p.bg_card, base, 0.15))
}

/// Numeric aria2/DB field that arrives either as a decimal string or a
/// number (same rule as `state::num_u64`).
fn dm_num(v: &serde_json::Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| {
        x.as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| x.as_u64())
    })
}

/// String field with an empty-string default.
fn dm_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_owned()
}

/// `formatRemainingTime()` — `Ns` / `Nm Ns` / `Nh Nm` / `Nd Nh`, zero tails
/// dropped, `null`/`<= 0` → `"-"`.
fn dm_format_remaining(secs: Option<u64>) -> String {
    let Some(raw) = secs.filter(|s| *s > 0) else {
        return "-".to_owned();
    };
    let total = raw as f64;
    let total = total.ceil() as u64;
    if total < 60 {
        return format!("{total}s");
    }
    let minutes = total / 60;
    if minutes < 60 {
        let s = total % 60;
        return if s > 0 {
            format!("{minutes}m {s}s")
        } else {
            format!("{minutes}m")
        };
    }
    let hours = minutes / 60;
    if hours < 24 {
        let m = minutes % 60;
        return if m > 0 {
            format!("{hours}h {m}m")
        } else {
            format!("{hours}h")
        };
    }
    let days = hours / 24;
    let h = hours % 24;
    if h > 0 {
        format!("{days}d {h}h")
    } else {
        format!("{days}d")
    }
}

/// Inverse of `state::days_from_civil` (Howard Hinnant's `civil_from_days`).
fn dm_civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Local wall-clock breakdown of an epoch-ms timestamp — the browser's
/// `new Date(ms)` → `toLocaleString()` was local time too.
fn dm_civil(ms: i64) -> (i64, u32, u32, u32, u32, u32) {
    let local = ms + crate::state::local_utc_offset_ms();
    let secs = local.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = dm_civil_from_days(days);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    )
}

/// `Date.toLocaleString()` → `M/D/YYYY, h:mm:ss AM/PM` (en-US, unpadded
/// month/day — exactly what `toLocaleString()` emits).
fn dm_format_dt(ms: i64) -> String {
    let (y, m, d, h24, mi, s) = dm_civil(ms);
    let (h, ampm) = match h24 {
        0 => (12, "AM"),
        1..=11 => (h24, "AM"),
        12 => (12, "PM"),
        _ => (h24 - 12, "PM"),
    };
    format!("{m}/{d}/{y}, {h}:{mi:02}:{s:02} {ampm}")
}

/// `Date.toLocaleTimeString()` → `h:mm:ss AM/PM` (log entry timestamps).
fn dm_format_log_time(ms: i64) -> String {
    let (_, _, _, h24, mi, s) = dm_civil(ms);
    let (h, ampm) = match h24 {
        0 => (12, "AM"),
        1..=11 => (h24, "AM"),
        12 => (12, "PM"),
        _ => (h24 - 12, "PM"),
    };
    format!("{h}:{mi:02}:{s:02} {ampm}")
}

/// ISO-8601 → epoch ms. `Z`/`±HH:MM` suffixes are honoured; a naive string
/// (aria2's `startTime`) is read as local time, which is how the browser's
/// `new Date(iso)` treats it.
fn dm_iso_to_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: i64 = s[5..7].parse().ok()?;
    let day: i64 = s[8..10].parse().ok()?;
    let hour: i64 = s[11..13].parse().ok()?;
    let min: i64 = s[14..16].parse().ok()?;
    let sec: i64 = s[17..19].parse().ok()?;
    let mut rest = &s[19..];
    let mut frac = 0i64;
    if let Some(tail) = rest.strip_prefix('.') {
        let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        frac = digits
            .get(..3)
            .and_then(|x| x.parse::<i64>().ok())
            .unwrap_or(0);
        rest = &tail[digits.len()..];
    }
    // Offset added to UTC to reach the printed wall clock.
    let offset = if rest.is_empty() {
        crate::state::local_utc_offset_ms()
    } else if rest.starts_with('Z') || rest.starts_with('z') {
        0
    } else {
        let sign = if rest.starts_with('-') { -1 } else { 1 };
        let r = &rest[1..];
        let hh: i64 = r.get(0..2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let mm: i64 = if r.len() >= 5 && r.as_bytes()[2] == b':' {
            r.get(3..5).and_then(|x| x.parse().ok()).unwrap_or(0)
        } else if r.len() >= 4 {
            r.get(2..4).and_then(|x| x.parse().ok()).unwrap_or(0)
        } else {
            0
        };
        sign * (hh * 3600 + mm * 60) * 1000
    };
    let wall = crate::state::days_from_civil(year, month, day) * 86_400_000
        + hour * 3_600_000
        + min * 60_000
        + sec * 1000
        + frac;
    Some(wall - offset)
}

/// `formatDateTime()` — numbers are epoch-ms, numeric strings too, anything
/// else goes through the ISO parser before falling back to the raw value.
fn dm_format_dt_value(v: &serde_json::Value) -> String {
    if v.is_null() {
        return "-".to_owned();
    }
    if let Some(n) = v.as_i64() {
        return dm_format_dt(n);
    }
    if let Some(n) = v.as_u64() {
        return dm_format_dt(n as i64);
    }
    if let Some(s) = v.as_str() {
        if s.is_empty() {
            return "-".to_owned();
        }
        if let Ok(n) = s.parse::<f64>() {
            if n.is_finite() {
                return dm_format_dt(n as i64);
            }
        }
        if let Some(ms) = dm_iso_to_ms(s) {
            return dm_format_dt(ms);
        }
        return s.to_owned(); // `isNaN(d.getTime()) → String(value)`
    }
    "-".to_owned()
}

/// `shortenPath()` — first `PATH_PREVIEW_MAX` characters plus `…`.
fn dm_shorten_path(value: &str) -> String {
    if value.is_empty() {
        return "-".to_owned();
    }
    if value.chars().count() <= DM_PATH_MAX {
        return value.to_owned();
    }
    let head: String = value.chars().take(DM_PATH_MAX).collect();
    format!("{head}...")
}

/// `getDownloadFolder()` — parent of the first path we have (`i > 0`,
/// otherwise `"-"`), keeping the original separators.
fn dm_folder(v: &DmView, details: Option<&serde_json::Value>) -> String {
    let parent = |p: &str| -> Option<String> {
        let i = p.rfind(['/', '\\'])?;
        if i > 0 {
            Some(p[..i].to_owned())
        } else {
            None
        }
    };
    if !v.vm.path.is_empty() {
        if let Some(dir) = parent(&v.vm.path) {
            return dir;
        }
    }
    if let Some(f) = details
        .and_then(|d| d.get("files"))
        .and_then(|x| x.as_array())
        .and_then(|a| a.first())
    {
        if let Some(p) = f.get("path").and_then(|x| x.as_str()) {
            if let Some(dir) = parent(p) {
                return dir;
            }
        }
    }
    "-".to_owned()
}

/// `selectedDownloadDetails.files` (aria2 reports `path` / `length` /
/// `completedLength` as decimal strings).
fn dm_files(details: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    details
        .and_then(|d| d.get("files"))
        .and_then(|f| f.as_array())
        .cloned()
        .unwrap_or_default()
}

/// `selectedDownloadDetails.errorMessage`, falling back to the error code
/// the way `DownloadVm::from_value` builds it.
fn dm_error_message(details: Option<&serde_json::Value>) -> String {
    let Some(d) = details else {
        return String::new();
    };
    let msg = dm_str(d, "errorMessage");
    if !msg.trim().is_empty() {
        return msg.trim().to_owned();
    }
    let code = dm_str(d, "errorCode");
    let code = code.trim();
    if code.is_empty() || code == "0" {
        return String::new();
    }
    format!("Error code: {code}")
}

/// `.pf-input` — 32px inset field with the hover/focus border ladder.
fn dm_input(
    ui: &mut Ui,
    p: &Palette,
    id: egui::Id,
    rect: Rect,
    value: &mut String,
    hint: &str,
    secret: bool,
) {
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
    // `.pf-input` `#1b1b2f` → the repo's inset token; hover `border-strong`,
    // focus `accent-strong` (the focus `#191b2d` fill is imperceptible here).
    let stroke = if focused {
        p.accent_strong
    } else if hovered {
        p.border_strong
    } else {
        p.border
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), p.bg_card_inset);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    // egui 0.37 ignores `.margin()` when `.frame()` is set, so the CSS
    // `padding: 0 10px` lives on the frame itself (same as `nd_input_at`).
    let font = FontId::new(12.0, egui::FontFamily::default());
    let line_h = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
    let text_rect = Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect).id(id), |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(text_rect.width())
                .font(font)
                .text_color(p.text)
                .hint_text(RichText::new(hint).size(12.0).color(p.text_dim.gamma_multiply(0.5)))
                .password(secret)
                .frame(egui::Frame::NONE.inner_margin(Margin {
                    left: 10,
                    right: 10,
                    top: 0,
                    bottom: 0,
                })),
        );
    });
}

/// `.pf-field` — 11px label (with the optional `(optional)` tail) at
/// `at.y + 7`, `.pf-input` at `at.y + 20`.
fn dm_field(
    ui: &mut Ui,
    p: &Palette,
    id: egui::Id,
    at: Pos2,
    w: f32,
    label: &str,
    opt: Option<&str>,
    value: &mut String,
    hint: &str,
    secret: bool,
) {
    let font = FontId::new(11.0, tokens::typography::semibold_family());
    ui.painter()
        .text(pos2(at.x, at.y + 7.0), Align2::LEFT_CENTER, label, font, p.text_dim);
    if let Some(opt) = opt {
        let x = at.x + dm_text_w(ui, label, &FontId::new(11.0, tokens::typography::semibold_family())) + 4.0;
        ui.painter().text(
            pos2(x, at.y + 7.0),
            Align2::LEFT_CENTER,
            opt,
            FontId::new(10.0, egui::FontFamily::default()),
            p.text_dim,
        );
    }
    dm_input(
        ui,
        p,
        id,
        Rect::from_min_size(pos2(at.x, at.y + 20.0), vec2(w, DM_INPUT_H)),
        value,
        hint,
        secret,
    );
}

/// `parseProxyString()` — a non-empty value always yields an enabled http
/// proxy (port defaults to 1080); empty yields the `emptyProxyForm()`.
/// Returns `(enabled, type, host, port, user, pass)`.
pub(crate) fn dm_parse_proxy(value: &str) -> (bool, String, String, String, String, String) {
    let empty = || {
        (
            false,
            "http".to_owned(),
            String::new(),
            "1080".to_owned(),
            String::new(),
            String::new(),
        )
    };
    if value.is_empty() {
        return empty();
    }
    // `/^([a-zA-Z][a-zA-Z0-9+.-]*):\/\//`
    let mut rest = value;
    if let Some(idx) = value.find("://") {
        let scheme = &value[..idx];
        if scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        {
            rest = &value[idx + 3..];
        }
    }
    // Credentials: everything before the *last* `@`.
    let mut creds = "";
    if let Some(at) = rest.rfind('@') {
        creds = &rest[..at];
        rest = &rest[at + 1..];
    }
    // `creds.includes(':') ? creds.split(':') : [creds, '']` — first colon
    // splits, a second one truncates the password.
    let (user, pass) = match creds.find(':') {
        Some(c) => (
            &creds[..c],
            creds[c + 1..].split(':').next().unwrap_or(""),
        ),
        None => (creds, ""),
    };
    // Host/port: last colon followed only by digits.
    let mut host = rest;
    let mut port: u32 = 0;
    if let Some(colon) = rest.rfind(':') {
        if colon > 0 {
            let tail = &rest[colon + 1..];
            if !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()) {
                host = &rest[..colon];
                port = tail.parse().unwrap_or(0);
            }
        }
    }
    if port == 0 {
        port = 1080;
    }
    (
        true,
        "http".to_owned(),
        host.to_owned(),
        port.to_string(),
        user.to_owned(),
        pass.to_owned(),
    )
}

/// `serializeProxy()` — `http://[user:pass@]host:port`, or empty when the
/// proxy is off / hostless (meaning "inherit global").
fn dm_serialize_proxy(state: &UiState) -> String {
    if !state.dm_proxy_enabled {
        return String::new();
    }
    let host = state.dm_proxy_host.trim();
    if host.is_empty() {
        return String::new();
    }
    let port = state
        .dm_proxy_port
        .parse::<u32>()
        .ok()
        .filter(|p| *p > 0)
        .unwrap_or(1080);
    let creds = if !state.dm_proxy_user.is_empty() || !state.dm_proxy_pass.is_empty() {
        format!("{}:{}@", state.dm_proxy_user, state.dm_proxy_pass)
    } else {
        String::new()
    };
    format!("http://{creds}{host}:{port}")
}

// ─── Download details modal — view model, open-edge fetches, shell ──────────

/// Immutable per-frame snapshot of the download under inspection: the live
/// list row (or a DB shell when aria2 has already forgotten the gid), the
/// yt-dlp marker, the `added_at` stamp and the persisted proxy string.
struct DmView {
    vm: DownloadVm,
    is_yt: bool,
    added_at: Option<i64>,
    proxy: String,
}

/// Resolve `state.dm_gid` against the live list first (the old app re-read
/// `selectedDownload` on every `downloads` refresh) and fall back to the
/// persisted row, then to the raw details payload, then to an empty shell so
/// the header still renders while the fetches are in flight.
fn dm_view(state: &UiState) -> Option<DmView> {
    let gid = state.dm_gid.clone()?;
    let db = state.dm_db_row.as_ref().filter(|r| !r.is_null());
    let db_source = db
        .and_then(|r| r.get("source"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let db_token = db
        .and_then(|r| r.get("ytdlp_token"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let added_at = db
        .and_then(|r| r.get("added_at"))
        .and_then(|v| v.as_i64())
        .or_else(|| state.first_seen.get(&gid).copied());
    let proxy = db
        .and_then(|r| r.get("proxy"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let direct_yt = gid.starts_with("yt-dlp-direct-");
    let is_yt = |vm_is_yt: bool| {
        vm_is_yt
            || !db_token.is_empty()
            || db_source == "yt-dlp"
            || direct_yt
    };

    if let Some(vm) = state.downloads.iter().find(|d| d.gid == gid) {
        return Some(DmView {
            vm: vm.clone(),
            is_yt: is_yt(vm.is_yt),
            added_at,
            proxy,
        });
    }
    // The persisted row (completed/failed downloads aria2 no longer tracks).
    if let Some(row) = db {
        let vm = DownloadVm {
            gid: gid.clone(),
            filename: dm_str(row, "filename"),
            path: row
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            url: dm_str(row, "url"),
            status: Status::parse(&dm_str(row, "status")),
            total: 0,
            completed: 0,
            size_text: dm_str(row, "size"),
            added_at: row.get("added_at").and_then(|v| v.as_i64()).unwrap_or(0),
            speed: 0,
            connections: row.get("connections").and_then(|v| v.as_i64()).map(|n| n as u32),
            remaining_seconds: None,
            error_message: dm_str(row, "errorMessage"),
            error_code: String::new(),
            is_yt: false,
            progress: row.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) as f32,
            file_state: row
                .get("fileState")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
        };
        return Some(DmView {
            vm,
            is_yt: is_yt(false),
            added_at,
            proxy,
        });
    }
    // Last resort: the aria2 details payload, else an empty shell.
    let details = state.dm_data.as_ref();
    let path = details
        .and_then(|d| d.get("files"))
        .and_then(|f| f.as_array())
        .and_then(|a| a.first())
        .and_then(|f| f.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let filename = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_owned();
    let status = details
        .map(|d| Status::parse(&dm_str(d, "status")))
        .unwrap_or(Status::Waiting);
    let total = details.and_then(|d| dm_num(d, "totalLength")).unwrap_or(0);
    let completed = details
        .and_then(|d| dm_num(d, "completedLength"))
        .unwrap_or(0);
    let vm = DownloadVm {
        gid,
        filename,
        path,
        url: String::new(),
        status,
        total,
        completed,
        size_text: String::new(),
        added_at: 0,
        speed: 0,
        connections: details
            .and_then(|d| dm_num(d, "connections"))
            .map(|n| n as u32),
        remaining_seconds: None,
        error_message: dm_error_message(details),
        error_code: String::new(),
        is_yt: false,
        progress: if total > 0 {
            completed as f32 / total as f32 * 100.0
        } else {
            0.0
        },
        file_state: String::new(),
    };
    Some(DmView {
        vm,
        is_yt: is_yt(false),
        added_at,
        proxy,
    })
}

/// `showDownloadDetails` / the `watch(showDetailsModal)` open work — runs
/// exactly once per open: tab and every fetch result reset, the proxy form
/// seeded (`parseProxyString('')`), then the three detail fetches fire.
fn dm_on_open(state: &mut UiState, cctx: &Ctx) {
    state.dm_tab = 0;
    state.dm_data = None;
    state.dm_loading = true;
    state.dm_events.clear();
    state.dm_events_loading = false;
    state.dm_yt_health = None;
    state.dm_db_row = None;
    state.dm_proxy_msg = None;
    state.dm_proxy_silent = false;
    state.dm_proxy_applying = false;
    state.dm_detecting = false;
    state.dm_detect_msg = None;
    // `detailProxy.value = parseProxyString(download.proxy)` — the `dm-db`
    // handler reseeds the form from the persisted row when it lands.
    let (enabled, typ, host, port, user, pass) = dm_parse_proxy("");
    state.dm_proxy_enabled = enabled;
    state.dm_proxy_type = typ;
    state.dm_proxy_host = host;
    state.dm_proxy_port = port;
    state.dm_proxy_user = user;
    state.dm_proxy_pass = pass;

    let Some(gid) = state.dm_gid.clone() else {
        state.dm_loading = false;
        return;
    };
    cctx.emit("dm-details", {
        let g = gid.clone();
        move |_app| async move { moonhunt_core::aria2::get_download_details(g).await }
    });
    cctx.emit("dm-db", {
        let g = gid.clone();
        move |app| async move {
            moonhunt_core::db::get_download_by_gid(app, g)
                .map(|r| r.unwrap_or(serde_json::Value::Null))
        }
    });
    // `loadYtHealth()` only runs for `yt-dlp-direct-*` gids.
    if gid.starts_with("yt-dlp-direct-") {
        cctx.emit("dm-yt-health", move |app| async move {
            moonhunt_core::ytdlp::get_yt_download_health(app, gid)
        });
    }
}

/// `applyDetailProxy(silent)` — guards re-entry, records whether this result
/// must stay quiet, optimistically writes the serialized proxy onto the DB
/// row we hold (the old app mutated `selectedDownload.proxy` the same way)
/// and hands the form to `set_download_proxy`.
fn dm_apply_proxy(state: &mut UiState, cctx: &Ctx, silent: bool) {
    let Some(gid) = state.dm_gid.clone() else {
        return;
    };
    if state.dm_proxy_applying {
        return;
    }
    state.dm_proxy_applying = true;
    state.dm_proxy_silent = silent;
    state.dm_proxy_msg = None;
    let serialized = dm_serialize_proxy(state);
    if let Some(row) = state.dm_db_row.as_mut() {
        if row.is_object() {
            row["proxy"] = serde_json::Value::String(serialized);
        }
    }
    let pdp = moonhunt_core::state::PerDownloadProxy {
        enabled: state.dm_proxy_enabled,
        proxy_type: state.dm_proxy_type.clone(),
        proxy_host: state.dm_proxy_host.clone(),
        proxy_port: state
            .dm_proxy_port
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|p| *p > 0)
            .unwrap_or(1080),
        proxy_user: state.dm_proxy_user.clone(),
        proxy_pass: state.dm_proxy_pass.clone(),
    };
    cctx.emit("dm-proxy-apply", move |app| async move {
        moonhunt_core::aria2::set_download_proxy(app, gid, Some(pdp))
            .await
            .map(serde_json::Value::String)
    });
}

/// `.dm-btn` — the footer's accent button: 13px bold `#0a0e1a` label, 16px
/// glyph, 6px gap, 10r and the `0 2px 8px rgba(accent,.2)` drop shadow
/// (egui shapes have no blur, so it is a solid rect 2px lower).
fn dm_btn(ui: &mut Ui, p: &Palette, rect: Rect, label: &str, icon: &str, id: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    let painter = ui.painter();
    // `.dm-btn:hover { transform: translateY(-1px) }`.
    let lift = if rr.hovered() { -1.0 } else { 0.0 };
    let btn = rect.translate(vec2(0.0, lift));
    let shadow = Rect::from_min_max(
        pos2(btn.min.x, btn.min.y + 2.0),
        pos2(btn.max.x, btn.max.y + 2.0),
    );
    painter.rect_filled(shadow, CornerRadius::same(10), p.accent.gamma_multiply(0.2));
    painter.rect_filled(btn, CornerRadius::same(10), p.accent);
    let font = FontId::new(13.0, tokens::typography::bold_family());
    let label_w = dm_text_w(ui, label, &font);
    let total = 16.0 + 6.0 + label_w;
    let x = btn.center().x - total * 0.5;
    let dark = Color32::from_rgb(0x0a, 0x0e, 0x1a);
    mi::paint_centered(painter, pos2(x + 8.0, btn.center().y), icon, 16.0, dark);
    painter.text(
        pos2(x + 22.0, btn.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        dark,
    );
    rr.clicked()
}

/// The modal itself: dim overlay + Esc, then the taffy shell (header / tabs
/// / scrollable body / footer) inside a rounded, bordered card.
fn details_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    use egui_taffy::taffy::style_helpers::{auto, length};

    let p = mk::pal();
    let screen = ctx.content_rect();
    // `.details-modal-new { max-width: 640px; width: 94%; max-height: 85vh }`.
    let w = 640.0_f32.min(screen.width() * 0.94);
    let max_h = screen.height() * 0.85;

    let Some(v) = dm_view(state) else {
        state.dm_open = false;
        return;
    };

    // `.details-overlay` — dim + click-outside close (`useOverlayClose`).
    {
        let open = &mut state.dm_open;
        egui::Area::new(egui::Id::new("dm-overlay"))
            .order(egui::Order::Middle)
            .anchor(Align2::LEFT_TOP, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(140));
                if ui.allocate_rect(screen, Sense::click()).clicked() {
                    *open = false;
                }
            });
    }

    // Esc closes — swallowed while a sub-dialog of the new-download modal is
    // stacked above (they consume their own key presses first).
    if !state.nd_sched_open
        && !state.nd_catmgr_open
        && ctx.input(|i| i.key_pressed(Key::Escape))
    {
        state.dm_open = false;
    }
    if !state.dm_open {
        return;
    }

    egui::Area::new(egui::Id::new("dm-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(16))
                // `.details-modal-new { border: 1px solid var(--border) }`.
                .stroke(egui::Stroke::new(1.0, p.border))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    // Root: column flex, definite width, content height capped
                    // at `85vh` (the CSS `max-height`).
                    let root = taffy::Style {
                        flex_direction: taffy::FlexDirection::Column,
                        size: taffy::Size {
                            width: length(w),
                            height: auto(),
                        },
                        max_size: taffy::Size {
                            width: length(w),
                            height: length(max_h),
                        },
                        ..Default::default()
                    };
                    // Header/tabs/footer keep their intrinsic height; only the
                    // body flexes (taffy defaults: shrink 1, grow 0).
                    let band = |h: f32| taffy::Style {
                        size: taffy::Size {
                            width: length(w),
                            height: length(h),
                        },
                        flex_shrink: 0.0,
                        ..Default::default()
                    };
                    // `.dm-body { flex: 1; overflow-y: auto; min-height: 100px }`.
                    let body = taffy::Style {
                        size: taffy::Size {
                            width: length(w),
                            height: auto(),
                        },
                        flex_grow: 1.0,
                        flex_shrink: 1.0,
                        flex_basis: auto(),
                        min_size: taffy::Size {
                            width: auto(),
                            height: length(100.0),
                        },
                        overflow: taffy::Point {
                            x: taffy::Overflow::Visible,
                            y: taffy::Overflow::Scroll,
                        },
                        ..Default::default()
                    };

                    egui_taffy::tui(ui, egui::Id::new("dm-shell"))
                        .with_available_space(taffy::Size {
                            width: taffy::AvailableSpace::Definite(w),
                            height: taffy::AvailableSpace::Definite(max_h),
                        })
                        .style(root)
                        .show(|tui| {
                            tui.style(band(DM_HEADER_H))
                                .ui(|ui| dm_header(ui, state, &v, w));
                            tui.style(band(DM_TABS_H))
                                .ui(|ui| dm_tabs(ui, state, cctx, w));
                            // Body is a container so the ScrollArea wraps a
                            // leaf that carries the panel's content height.
                            tui.style(body).add(|tui| {
                                tui.ui(|ui| dm_panel(ui, &p, &v, state, cctx, w));
                            });
                            tui.style(band(DM_FOOTER_H))
                                .ui(|ui| dm_footer(ui, state, &p, w));
                        });
                });
        });
}

/// `.dm-header` — avatar + filename + meta on the left, status pill and the
/// copy/close icon buttons on the right, 1px rule under it all.
fn dm_header(ui: &mut Ui, state: &mut UiState, v: &DmView, w: f32) {
    let p = mk::pal();
    ui.set_width(w);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let r = dm_row(ui, DM_HEADER_H);
    let painter = ui.painter();

    // `padding: 13px 20px` around a 44px avatar (+1px rule at the bottom).
    let inner = Rect::from_min_max(
        pos2(r.min.x + DM_PAD_X, r.min.y + 13.0),
        pos2(r.max.x - DM_PAD_X, r.max.y - 14.0),
    );

    // ── Left: `.dm-file-avatar` + `.dm-file-info` (gap 14) ──
    let avatar = Rect::from_min_size(inner.min, vec2(44.0, 44.0));
    painter.rect_filled(
        avatar,
        CornerRadius::same(12),
        mk::blend(p.bg_card, p.accent, 0.08),
    );
    mi::paint_centered(
        painter,
        avatar.center(),
        dm_file_icon(&v.vm.filename),
        22.0,
        p.accent_strong,
    );

    let info_x = avatar.max.x + 14.0;
    // The actions group is measured first so the filename knows its budget.
    let pill_label = dm_status_label(v.vm.status);
    let pill = DmPill {
        pad_x: 12.0,
        pad_y: 5.0,
        gap: 5.0,
        icon: Some((dm_status_icon(v.vm.status), 15.0)),
        font: FontId::new(11.0, tokens::typography::semibold_family()),
        fg: v.vm.status.color(),
        bg: Some(mk::blend(p.bg_card, v.vm.status.color(), 0.14)),
    };
    let pill_w = dm_pill_w(ui, pill_label, &pill);
    let btn = 34.0;
    let actions_w = pill_w + 8.0 + btn + 8.0 + btn;
    let actions_x = r.max.x - DM_PAD_X - actions_w;

    // `.dm-filename` 15/600, line-height 1.4, ellipsized.
    let name_font = FontId::new(15.0, tokens::typography::semibold_family());
    let (name_w, name_h) = dm_text_size(ui, "Ag", &name_font);
    let _ = name_w;
    // `.dm-meta-row { margin-top: 3px }` — gid 11 mono .7, `•` 10 .4, badge.
    let meta_font = FontId::new(11.0, egui::FontFamily::Monospace);
    let badge_label = if v.is_yt {
        "YouTube / yt-dlp"
    } else {
        "HTTP / FTP"
    };
    let badge = DmPill {
        pad_x: 8.0,
        pad_y: 2.0,
        gap: 4.0,
        icon: Some((
            if v.is_yt { "smart_display" } else { "cloud_download" },
            13.0,
        )),
        font: FontId::new(10.5, tokens::typography::semibold_family()),
        fg: if v.is_yt { p.status_error } else { p.accent_strong },
        bg: Some(mk::blend(
            p.bg_card,
            if v.is_yt { p.status_error } else { p.accent },
            0.08,
        )),
    };
    let _badge_w = dm_pill_w(ui, badge_label, &badge);
    let (gid_w, gid_h) = dm_text_size(ui, "000000000000", &meta_font);
    let meta_h = gid_h.max(18.0);
    let block_h = name_h + 3.0 + meta_h;
    let block_y = inner.min.y + (inner.height() - block_h).max(0.0) * 0.5;

    let name_max = (actions_x - 16.0 - info_x).max(40.0);
    let name = if v.vm.filename.is_empty() {
        "Details".to_owned()
    } else {
        v.vm.filename.clone()
    };
    painter.text(
        pos2(info_x, block_y + name_h * 0.5),
        Align2::LEFT_CENTER,
        dm_ellipsize(ui, &name, &name_font, name_max),
        name_font,
        p.text,
    );

    let meta_y = block_y + name_h + 3.0 + meta_h * 0.5;
    let gid_text = v.vm.gid.chars().take(12).collect::<String>();
    painter.text(
        pos2(info_x, meta_y),
        Align2::LEFT_CENTER,
        gid_text,
        meta_font.clone(),
        p.text_dim.gamma_multiply(0.7),
    );
    let mut mx = info_x + gid_w + 6.0;
    // `.dm-divider` — the `•` separator at 40% opacity.
    painter.text(
        pos2(mx, meta_y),
        Align2::LEFT_CENTER,
        "•",
        FontId::new(10.0, egui::FontFamily::default()),
        p.text_dim.gamma_multiply(0.4),
    );
    mx += dm_text_w(ui, "•", &FontId::new(10.0, egui::FontFamily::default())) + 6.0;
    dm_pill(ui, pos2(mx, meta_y), badge_label, &badge);

    // ── Right: `.dm-header-actions` (gap 8) ──
    let cy = inner.center().y;
    let copy = Rect::from_min_size(
        pos2(actions_x + pill_w + 8.0, cy - btn * 0.5),
        vec2(btn, btn),
    );
    let close = Rect::from_min_size(
        pos2(copy.max.x + 8.0, cy - btn * 0.5),
        vec2(btn, btn),
    );
    dm_pill(ui, pos2(actions_x, cy), pill_label, &pill);
    if dm_icon_btn(ui, &p, copy, "content_copy", "dm-copy-gid") {
        ui.ctx().copy_text(v.vm.gid.clone());
        state.toast(true, "Copied to clipboard");
    }
    if dm_icon_btn(ui, &p, close, "close", "dm-close") {
        state.dm_open = false;
    }

    // `border-bottom: 1px solid var(--border)` — painted, not modelled.
    ui.painter().rect_filled(
        Rect::from_min_max(pos2(r.min.x, r.max.y - 1.0), r.max),
        CornerRadius::same(0),
        p.border,
    );
}

/// `.dm-tabs` — three icon+label tabs, active one accented with a 2.5px
/// underline; clicking Log re-fetches the event list (as the old handler did).
fn dm_tabs(ui: &mut Ui, state: &mut UiState, cctx: &Ctx, w: f32) {
    let p = mk::pal();
    ui.set_width(w);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let r = dm_row(ui, DM_TABS_H);
    let painter = ui.painter();
    // `padding: 0 16px; gap: 2px` — the rule sits under everything.
    let content_h = r.height() - 1.0;
    let font = FontId::new(12.5, tokens::typography::semibold_family());
    let icon_font_size = 17.0;
    let mut x = r.min.x + 16.0;
    let mut clicked: Option<u8> = None;

    for (i, (icon, label)) in [
        ("dashboard", "General"),
        ("dns", "Network"),
        ("terminal", "Log"),
    ]
    .into_iter()
    .enumerate()
    {
        let i = i as u8;
        let label_w = dm_text_w(ui, label, &font);
        // `padding: 11px 14px; gap: 7px; border-bottom: 2.5px transparent`.
        let tw = 14.0 + icon_font_size + 7.0 + label_w + 14.0;
        let rect = Rect::from_min_size(
            pos2(x, r.min.y),
            vec2(tw, content_h),
        );
        let active = state.dm_tab == i;
        let rr = ui.interact(rect, egui::Id::new("dm-tab").with(i), Sense::click());
        if rr.hovered() && !active {
            // `.dm-tab:hover { background: rgba(accent,0.04) }`.
            painter.rect_filled(rect, CornerRadius::same(0), mk::blend(p.bg_card, p.accent, 0.04));
        }
        let fg = if active {
            p.accent_strong
        } else if rr.hovered() {
            p.text
        } else {
            p.text_dim
        };
        // Tabs center their icon+label in the line box above the underline.
        let cy = rect.min.y + (content_h - 2.5) * 0.5;
        mi::paint_centered(
            painter,
            pos2(x + 14.0 + icon_font_size * 0.5, cy),
            icon,
            icon_font_size,
            fg,
        );
        painter.text(
            pos2(x + 14.0 + icon_font_size + 7.0, cy),
            Align2::LEFT_CENTER,
            label,
            font.clone(),
            fg,
        );
        if active {
            // `.dm-tab.active { border-bottom-color: accent-strong }`.
            painter.rect_filled(
                Rect::from_min_max(
                    pos2(x, r.max.y - 1.0 - 2.5),
                    pos2(x + tw, r.max.y - 1.0),
                ),
                CornerRadius::same(0),
                p.accent_strong,
            );
        }
        if rr.clicked() {
            clicked = Some(i);
        }
        x += tw + 2.0; // `gap: 2px`
    }

    if let Some(i) = clicked {
        state.dm_tab = i;
        if i == 2 {
            // `@click="loadDownloadEvents(); activeDetailsTab = 'log'"` —
            // re-fetches on every Log click, not just the first.
            state.dm_events_loading = true;
            if let Some(gid) = state.dm_gid.clone() {
                cctx.emit("dm-events", move |_app| async move {
                    moonhunt_core::aria2::get_download_events(gid)
                        .await
                        .map(serde_json::Value::Array)
                });
            }
        }
    }

    painter.rect_filled(
        Rect::from_min_max(pos2(r.min.x, r.max.y - 1.0), r.max),
        CornerRadius::same(0),
        p.border,
    );
}

/// `.dm-footer` — right-aligned `.dm-btn` that dismisses the dialog.
fn dm_footer(ui: &mut Ui, state: &mut UiState, p: &Palette, w: f32) {
    ui.set_width(w);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let r = dm_row(ui, DM_FOOTER_H);
    // `border-top: 1px solid var(--border)`.
    ui.painter().rect_filled(
        Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    // `padding: 10px 20px; justify-content: flex-end`.
    let font = FontId::new(13.0, tokens::typography::bold_family());
    let label_w = dm_text_w(ui, state.t("Close"), &font);
    let bw = 22.0 + 16.0 + 6.0 + label_w + 22.0;
    let bh = 20.0 + dm_text_size(ui, state.t("Close"), &font).1;
    let rect = Rect::from_min_size(
        pos2(r.max.x - DM_PAD_X - bw, r.min.y + 1.0 + 10.0),
        vec2(bw, bh),
    );
    if dm_btn(ui, p, rect, state.t("Close"), "check", "dm-footer-close") {
        state.dm_open = false;
    }
}

// ─── Download details modal — tab panels ────────────────────────────────────

/// `.dm-body` content for the active `.dm-panel`: `padding: 16px 20px 12px`
/// (top/bottom rows here; the 20px sides are baked into `dm_body_row`) and the
/// tab dispatch. The ScrollArea itself comes from the taffy body node's
/// `overflow-y: scroll`, so this only stacks sections.
fn dm_panel(ui: &mut Ui, p: &Palette, v: &DmView, state: &mut UiState, cctx: &Ctx, w: f32) {
    ui.set_width(w);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    // Solid 4px bars for the nested lists (file table, log list).
    nd_scroll_style(ui);
    dm_body_row(ui, 16.0);
    let cw = w - 2.0 * DM_PAD_X;
    match state.dm_tab {
        0 => dm_general(ui, p, v, state, cw),
        1 => dm_network(ui, p, v, state, cctx, cw),
        _ => dm_log(ui, p, v, state, cw),
    }
    dm_body_row(ui, 12.0);
}

/// General tab: Transfer / Configuration `.dm-table`s, then the bordered
/// `.dm-file-table-wrapper` (or its loading / empty states).
fn dm_general(ui: &mut Ui, p: &Palette, v: &DmView, state: &mut UiState, cw: f32) {
    let data = state.dm_data.clone();
    let details = data.as_ref();
    let vm = &v.vm;
    let active = vm.status == Status::Active;

    // ── Transfer ──
    dm_section_title(ui, p, state.t("Transfer"), 10.0);
    let speed_row = vm.speed > 0 && active;
    // Status / Progress / Size, plus Speed and ETA while the row is active.
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Status"), DM_ROW_H, true, false);
        // `.dm-status-badge` — status-colored 16px glyph + 12.5/600 label
        // (`gap: 5px`), text keeps the td's `--text`.
        let cy = td.center().y;
        mi::paint_centered(
            ui.painter(),
            pos2(td.min.x + 8.0, cy),
            dm_status_icon(vm.status),
            16.0,
            vm.status.color(),
        );
        ui.painter().text(
            pos2(td.min.x + 8.0 + 16.0 + 5.0, cy),
            Align2::LEFT_CENTER,
            dm_status_label(vm.status),
            FontId::new(12.5, tokens::typography::semibold_family()),
            p.text,
        );
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Progress"), DM_ROW_H, false, false);
        // `.dm-progress-cell` — whole-percent fill + `{pct}%` label.
        dm_progress(ui, p, td, vm.progress.round());
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Size"), DM_ROW_H, false, !(speed_row || active));
        // `Number(totalLength) ? formatFileSize(…) : (totalLength || '—')`.
        let total_len = details.and_then(|d| dm_num(d, "totalLength")).unwrap_or(0);
        let txt = if total_len > 0 {
            moonhunt_core::util::format_bytes(total_len)
        } else {
            let raw = details.map(|d| dm_str(d, "totalLength")).unwrap_or_default();
            if raw.is_empty() {
                "—".to_owned()
            } else {
                raw
            }
        };
        ui.painter().text(
            pos2(td.min.x, td.center().y),
            Align2::LEFT_CENTER,
            txt,
            FontId::new(13.0, egui::FontFamily::default()),
            p.text,
        );
    }
    if speed_row {
        let (_, td) = dm_tr(ui, p, cw, state.t("Speed"), DM_ROW_H, false, !active);
        // `.dm-speed-cell` — accent-strong / 600.
        ui.painter().text(
            pos2(td.min.x, td.center().y),
            Align2::LEFT_CENTER,
            vm.speed_display(),
            FontId::new(13.0, tokens::typography::semibold_family()),
            p.accent_strong,
        );
    }
    if active {
        let (_, td) = dm_tr(ui, p, cw, state.t("ETA"), DM_ROW_H, false, true);
        ui.painter().text(
            pos2(td.min.x, td.center().y),
            Align2::LEFT_CENTER,
            dm_format_remaining(vm.remaining_seconds),
            FontId::new(13.0, egui::FontFamily::default()),
            p.text,
        );
    }
    dm_section_gap(ui);

    // ── Configuration ──
    dm_section_title(ui, p, state.t("Configuration"), 10.0);
    // `.dm-chip` / `.dm-priority` / `.dm-source-tag` share one pill metric
    // (`3px 10px`, 12/600, r100); only fill/foreground differ.
    let chip = |icon: Option<&'static str>, fg: Color32, bg: Color32| DmPill {
        pad_x: 10.0,
        pad_y: 3.0,
        gap: 4.0,
        icon: icon.map(|i| (i, 14.0)),
        font: FontId::new(12.0, tokens::typography::semibold_family()),
        fg,
        bg: Some(bg),
    };
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Connections"), DM_ROW_H_CHIP, true, false);
        let pill = chip(
            Some("layers"),
            p.accent_strong,
            mk::blend(p.bg_card, p.accent, 0.08),
        );
        let n = vm.connections.unwrap_or(1);
        dm_pill(ui, pos2(td.min.x, td.center().y), &n.to_string(), &pill);
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Active Connections"), DM_ROW_H_CHIP, false, false);
        let pill = chip(
            Some("device_hub"),
            p.accent_strong,
            mk::blend(p.bg_card, p.accent, 0.08),
        );
        dm_pill(ui, pos2(td.min.x, td.center().y), "1", &pill);
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Priority"), DM_ROW_H_CHIP, false, false);
        // `.dm-priority.normal` — neutral `#69708a` wash.
        let pill = chip(
            None,
            p.text_dim,
            mk::blend(p.bg_card, Color32::from_rgb(0x69, 0x70, 0x8a), 0.08),
        );
        dm_pill(ui, pos2(td.min.x, td.center().y), state.t("Normal"), &pill);
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Source"), DM_ROW_H_CHIP, false, false);
        let pill = if v.is_yt {
            chip(None, p.status_error, mk::blend(p.bg_card, p.status_error, 0.08))
        } else {
            chip(None, p.accent_strong, mk::blend(p.bg_card, p.accent, 0.08))
        };
        let label = if v.is_yt { state.t("YouTube (yt-dlp)") } else { state.t("HTTP / FTP") };
        dm_pill(ui, pos2(td.min.x, td.center().y), label, &pill);
    }
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Proxy"), DM_ROW_H_CHIP, false, true);
        // `.dm-source-tag.direct` for an empty per-download proxy string.
        let pill = if v.proxy.is_empty() {
            chip(None, p.text_dim, p.hover_veil)
        } else {
            let mut c = chip(None, p.text, p.text);
            c.bg = None;
            c
        };
        let label = if v.proxy.is_empty() { "Direct" } else { v.proxy.as_str() };
        dm_pill(ui, pos2(td.min.x, td.center().y), label, &pill);
    }
    dm_section_gap(ui);

    // ── Files ── (last section — `.dm-section:last-child` drops the gap)
    dm_section_title(ui, p, state.t("Files"), 10.0);
    if state.dm_loading {
        dm_empty_state(ui, p, "hourglass_empty", state.t("Loading..."), None);
        return;
    }
    let files = dm_files(details);
    if files.is_empty() {
        dm_empty_state(
            ui,
            p,
            "folder_open",
            state.t("No file information available"),
            Some(state.t("Files appear after download starts")),
        );
        return;
    }

    // `.dm-file-table-wrapper` — `r10 border1 overflow hidden max-height 240`.
    let wrap_h = (DM_FILE_HEAD_H + DM_FILE_ROW_H * files.len() as f32).min(240.0);
    let r = dm_body_row(ui, wrap_h);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(r).id_salt("dm-file-wrap"),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dm-file-table")
                .max_height(r.height())
                .show(ui, |ui| {
                    let width = ui.available_width();
                    // `colgroup` 30 / 13 / 19 / 38 %.
                    let cols = [
                        width * 0.30,
                        width * 0.13,
                        width * 0.19,
                        width * 0.38,
                    ];
                    // `thead` — hover-bg, 11/700 uppercase, 1px border rule.
                    let (hr, _) = ui.allocate_exact_size(vec2(width, DM_FILE_HEAD_H), Sense::hover());
                    ui.painter().rect_filled(hr, CornerRadius::same(0), p.hover_veil);
                    ui.painter().rect_filled(
                        Rect::from_min_max(pos2(hr.min.x, hr.max.y - 1.0), hr.max),
                        CornerRadius::same(0),
                        p.border,
                    );
                    let mut x = hr.min.x;
                    let heads = ["FILE", "SIZE", "PROGRESS", "PATH"].map(|k| state.t(k));
                    for (label, w) in heads.into_iter().zip(cols) {
                        ui.painter().text(
                            pos2(x + 12.0, hr.center().y),
                            Align2::LEFT_CENTER,
                            label,
                            FontId::new(11.0, tokens::typography::bold_family()),
                            p.text_dim,
                        );
                        x += w;
                    }
                    let n = files.len();
                    for (i, file) in files.iter().enumerate() {
                        let (rr, _) = ui.allocate_exact_size(vec2(width, DM_FILE_ROW_H), Sense::hover());
                        // `td` rule `rgba(accent,0.05)` — none on the last row.
                        if i + 1 < n {
                            ui.painter().rect_filled(
                                Rect::from_min_max(pos2(rr.min.x, rr.max.y - 1.0), rr.max),
                                CornerRadius::same(0),
                                mk::blend(p.bg_card, p.accent, 0.05),
                            );
                        }
                        let path = file.get("path").and_then(|x| x.as_str()).unwrap_or("");
                        let name = path
                            .rsplit(['/', '\\'])
                            .next()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(path);

                        // Name cell: 18px accent glyph, gap 8, name at 600.
                        let (cx0, cw0) = (rr.min.x, cols[0]);
                        mi::paint_centered(
                            ui.painter(),
                            pos2(cx0 + 12.0 + 9.0, rr.center().y),
                            dm_file_icon(name),
                            18.0,
                            p.accent_strong,
                        );
                        let name_font = FontId::new(12.5, tokens::typography::semibold_family());
                        let name_label = dm_ellipsize(
                            ui,
                            name,
                            &name_font,
                            cw0 - 24.0 - 18.0 - 8.0,
                        );
                        ui.painter().text(
                            pos2(cx0 + 12.0 + 18.0 + 8.0, rr.center().y),
                            Align2::LEFT_CENTER,
                            name_label,
                            name_font,
                            p.text,
                        );

                        // Size cell: `formatFileSize(Number(length) || 0)`.
                        let len = dm_num(file, "length").unwrap_or(0);
                        let cx1 = cx0 + cw0;
                        ui.painter().text(
                            pos2(cx1 + 12.0, rr.center().y),
                            Align2::LEFT_CENTER,
                            moonhunt_core::util::format_bytes(len),
                            FontId::new(12.5, egui::FontFamily::default()),
                            p.text,
                        );

                        // Progress cell: mini bar when both lengths exist.
                        let done = dm_num(file, "completedLength").unwrap_or(0);
                        let cx2 = cx1 + cols[1];
                        if len > 0 && done > 0 {
                            dm_mini_progress(
                                ui,
                                p,
                                Rect::from_min_size(pos2(cx2 + 12.0, rr.min.y), vec2(cols[2] - 24.0, rr.height())),
                                done as f32 / len as f32,
                            );
                        } else {
                            ui.painter().text(
                                pos2(cx2 + 12.0, rr.center().y),
                                Align2::LEFT_CENTER,
                                "—",
                                FontId::new(12.5, egui::FontFamily::default()),
                                p.text,
                            );
                        }

                        // Path cell: `shortenPath` + 28px `.sm` copy button.
                        let cx3 = cx2 + cols[2];
                        let path_font = FontId::new(12.5, egui::FontFamily::default());
                        let path_label =
                            dm_ellipsize(ui, &dm_shorten_path(path), &path_font, cols[3] - 24.0 - 28.0 - 8.0);
                        ui.painter().text(
                            pos2(cx3 + 12.0, rr.center().y),
                            Align2::LEFT_CENTER,
                            path_label,
                            path_font,
                            p.text_dim,
                        );
                        let btn = Rect::from_min_size(
                            pos2(cx3 + cols[3] - 12.0 - 28.0, rr.center().y - 14.0),
                            vec2(28.0, 28.0),
                        );
                        if dm_icon_btn(ui, p, btn, "content_copy", &format!("dm-copy-path-{i}")) {
                            ui.ctx().copy_text(path.to_owned());
                            // Old `copyToClipboard` (skips the "—" placeholder).
                            if !path.is_empty() && path != "—" {
                                state.toast(true, "Copied to clipboard");
                            }
                        }
                    }
                });
        },
    );
    // `border: 1px solid var(--border)` over the clipped content.
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(10),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );
}

/// Network tab: Connection table, optional Source URLs list, Proxy card.
fn dm_network(ui: &mut Ui, p: &Palette, v: &DmView, state: &mut UiState, cctx: &Ctx, cw: f32) {
    let data = state.dm_data.clone();
    let details = data.as_ref();

    // ── Connection ──
    dm_section_title(ui, p, state.t("Connection"), 10.0);
    let url = v.vm.url.clone();
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("URL"), DM_ROW_H_BTN, true, false);
        // `.dm-url-text` (`max-width: 260px`, 12px dim) + 34px copy button.
        let btn = Rect::from_min_size(pos2(td.max.x - 34.0, td.center().y - 17.0), vec2(34.0, 34.0));
        if url.is_empty() {
            ui.painter().text(
                pos2(td.min.x, td.center().y),
                Align2::LEFT_CENTER,
                "—",
                FontId::new(12.0, egui::FontFamily::default()),
                p.text_dim,
            );
        } else {
            let font = FontId::new(12.0, egui::FontFamily::default());
            let label = dm_ellipsize(ui, &url, &font, (260.0_f32).min(td.width() - 42.0));
            ui.painter()
                .text(pos2(td.min.x, td.center().y), Align2::LEFT_CENTER, label, font, p.text_dim);
            if dm_icon_btn(ui, p, btn, "content_copy", "dm-copy-url") {
                ui.ctx().copy_text(url.clone());
                state.toast(true, "Copied to clipboard");
            }
        }
    }
    let folder = dm_folder(v, details);
    let has_format = v.is_yt;
    {
        let (_, td) = dm_tr(ui, p, cw, state.t("Folder"), DM_ROW_H_BTN, false, !has_format);
        // `.dm-path-cell` — same shape as the URL row, copies the full path.
        let short = dm_shorten_path(&folder);
        let font = FontId::new(12.0, egui::FontFamily::default());
        let label = dm_ellipsize(ui, &short, &font, (220.0_f32).min(td.width() - 42.0));
        ui.painter()
            .text(pos2(td.min.x, td.center().y), Align2::LEFT_CENTER, label, font, p.text_dim);
        let btn = Rect::from_min_size(pos2(td.max.x - 34.0, td.center().y - 17.0), vec2(34.0, 34.0));
        if dm_icon_btn(ui, p, btn, "content_copy", "dm-copy-folder") {
            ui.ctx().copy_text(folder.clone());
            if !folder.is_empty() && folder != "—" {
                state.toast(true, "Copied to clipboard");
            }
        }
    }
    if has_format {
        // `v-if="selectedDownload?.source === 'yt-dlp'"`.
        let (_, td) = dm_tr(ui, p, cw, state.t("Format"), DM_ROW_H_CHIP, false, true);
        let pill = DmPill {
            pad_x: 10.0,
            pad_y: 3.0,
            gap: 4.0,
            icon: Some(("smart_display", 14.0)),
            font: FontId::new(12.0, tokens::typography::semibold_family()),
            fg: p.accent_strong,
            bg: Some(mk::blend(p.bg_card, p.accent, 0.08)),
        };
        dm_pill(ui, pos2(td.min.x, td.center().y), "yt-dlp", &pill);
    }
    dm_section_gap(ui);

    // ── Source URLs (`v-if="…details.source?.length"` — aria2 never fills
    // `source` today, so the section stays behind its guard) ──
    let sources: Vec<String> = details
        .and_then(|d| d.get("source"))
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if !sources.is_empty() {
        dm_section_title(ui, p, state.t("Source URLs"), 10.0);
        for (i, src) in sources.iter().enumerate() {
            // `.dm-source-list li` — `8px 12px`, hover-bg, r8, mb4.
            let font = FontId::new(12.0, egui::FontFamily::default());
            let g = ui
                .painter()
                .layout(src.clone(), font, p.text_dim, cw - 24.0 - 28.0 - 8.0);
            let h = 8.0 + g.rect.height() + 8.0;
            let r = dm_body_row(ui, h);
            ui.painter().rect_filled(r, CornerRadius::same(8), p.hover_veil);
            ui.painter()
                .galley(pos2(r.min.x + 12.0, r.min.y + 8.0), g, p.text_dim);
            let btn = Rect::from_min_size(
                pos2(r.max.x - 12.0 - 28.0, r.center().y - 14.0),
                vec2(28.0, 28.0),
            );
            if dm_icon_btn(ui, p, btn, "content_copy", &format!("dm-copy-src-{i}")) {
                ui.ctx().copy_text(src.clone());
                state.toast(true, "Copied to clipboard");
            }
            dm_body_row(ui, 4.0); // `margin-bottom: 4px`
        }
        dm_section_gap(ui);
    }

    // ── Proxy (ProxyField) — last section, no trailing gap ──
    dm_section_title(ui, p, state.t("Proxy"), 10.0);
    dm_proxy_field(ui, p, state, cctx);
}

/// Log tab: timeline line, optional diagnostics + error box, then the health
/// ring and event list (or the borderless empty state).
fn dm_log(ui: &mut Ui, p: &Palette, v: &DmView, state: &mut UiState, cw: f32) {
    let data = state.dm_data.clone();
    let details = data.as_ref();
    let vm = &v.vm;

    // ── `.dm-timeline-line` — Added · Started · Completed ──
    let mut line = format!(
        "Added {}",
        match v.added_at {
            Some(ms) => dm_format_dt(ms),
            None => "-".to_owned(),
        }
    );
    let push = |line: &mut String, prefix: &str, iso: Option<&str>| {
        if let Some(s) = iso.filter(|s| !s.is_empty()) {
            let v = serde_json::Value::String(s.to_owned());
            line.push_str(&format!(" · {prefix} {}", dm_format_dt_value(&v)));
        }
    };
    push(
        &mut line,
        state.t("Started"),
        details.and_then(|d| d.get("startTime")).and_then(|x| x.as_str()),
    );
    push(
        &mut line,
        state.t("Completed"),
        details
            .and_then(|d| d.get("completionTime"))
            .and_then(|x| x.as_str()),
    );
    {
        let font = FontId::new(11.5, egui::FontFamily::default());
        let g = ui.painter().layout(line, font, p.text_dim, cw - 20.0);
        let lh = g.rect.height().max(14.0);
        let r = dm_body_row(ui, lh + 12.0); // `margin-bottom: 12px`
        // `i` 14px at .7 opacity, column gap 6.
        mi::paint_centered(
            ui.painter(),
            pos2(r.min.x + 7.0, r.min.y + lh * 0.5),
            "history",
            14.0,
            p.text_dim.gamma_multiply(0.7),
        );
        ui.painter().galley(pos2(r.min.x + 20.0, r.min.y), g, p.text_dim);
    }

    // ── Diagnostics (`General > Show Download Diagnostics`) ──
    if state.settings.show_download_diagnostics {
        dm_section_title(ui, p, state.t("Diagnostics (debug)"), 4.0);
        dm_debug_row(ui, p, state.t("Engine"), "legacy");
        let done = if vm.completed > 0 {
            moonhunt_core::util::format_bytes(vm.completed)
        } else {
            "-".to_owned()
        };
        let tot = if vm.total > 0 {
            moonhunt_core::util::format_bytes(vm.total)
        } else {
            "-".to_owned()
        };
        dm_debug_row(ui, p, state.t("Raw bytes"), &format!("{done} / {tot}"));
        dm_debug_row(
            ui,
            p,
            state.t("Computed"),
            &format!("{}%", (vm.progress.round() as i64).clamp(0, 100)),
        );
        dm_debug_row(ui, p, state.t("Last event"), "-");
        if v.vm.gid.starts_with("yt-dlp-direct-") {
            let backend = match state.dm_yt_health {
                None => "…",
                Some(true) => state.t("process alive"),
                Some(false) => state.t("process exited"),
            };
            dm_debug_row(ui, p, state.t("Backend"), backend);
        }
        dm_body_row(ui, 10.0); // `.dm-debug { margin-bottom: 10px }`
    }

    // ── Error box ──
    let err = dm_error_message(details);
    if !err.is_empty() {
        dm_section_title_icon(ui, p, Some(("error_outline", p.status_error)), state.t("Error"), 10.0);
        let text_font = FontId::new(12.0, egui::FontFamily::default());
        // `pre { line-height: 1.5 }` — egui's row leading is close enough.
        let g = ui
            .painter()
            .layout(err.clone(), text_font, p.status_error, cw - 28.0);
        // `.dm-btn-secondary` — `6px 14px`, icon 15, gap 6, 12/600.
        let btn_font = FontId::new(12.0, tokens::typography::semibold_family());
        let btn_w = 14.0 + 15.0 + 6.0 + dm_text_w(ui, state.t("Copy error"), &btn_font) + 14.0 + 2.0;
        let btn_h = 6.0 + 15.0 + 6.0 + 2.0;
        let text_h = g.rect.height();
        let r = dm_body_row(ui, 12.0 + text_h + 10.0 + btn_h + 12.0);
        ui.painter()
            .rect_filled(r, CornerRadius::same(10), mk::blend(p.bg_card, p.status_error, 0.05));
        ui.painter().rect_stroke(
            r,
            CornerRadius::same(10),
            egui::Stroke::new(1.0, mk::blend(p.bg_card, p.status_error, 0.12)),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(pos2(r.min.x + 14.0, r.min.y + 12.0), g, p.status_error);
        let btn = Rect::from_min_size(
            pos2(r.min.x + 14.0, r.min.y + 12.0 + text_h + 10.0),
            vec2(btn_w, btn_h),
        );
        let br = ui.interact(btn, egui::Id::new("dm-copy-error"), Sense::click());
        let fill = if br.hovered() {
            mk::blend(p.bg_card, p.status_error, 0.18)
        } else {
            mk::blend(p.bg_card, p.status_error, 0.10)
        };
        ui.painter().rect_filled(btn, CornerRadius::same(6), fill);
        ui.painter().rect_stroke(
            btn,
            CornerRadius::same(6),
            egui::Stroke::new(1.0, mk::blend(p.bg_card, p.status_error, 0.10)),
            egui::StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(btn.min.x + 14.0 + 7.5, btn.center().y),
            "content_copy",
            15.0,
            p.status_error,
        );
        ui.painter().text(
            pos2(btn.min.x + 14.0 + 15.0 + 6.0, btn.center().y),
            Align2::LEFT_CENTER,
            state.t("Copy error"),
            btn_font,
            p.status_error,
        );
        if br.clicked() {
            ui.ctx().copy_text(err.clone());
            // Old `copyToClipboard` (skips the empty placeholder).
            if !err.is_empty() {
                state.toast(true, "Copied to clipboard");
            }
        }
        dm_section_gap(ui);
    }

    // ── Empty state, else health ring + `.dm-log-list` ──
    if state.dm_events.is_empty() && err.is_empty() {
        dm_empty(ui, p, "terminal", state.t("No events recorded yet"));
        return;
    }

    // `logPacketLoss()` — 0 timeouts → 100 %, else `1 − timeouts × 0.1`.
    let timeouts = state
        .dm_events
        .iter()
        .filter(|e| e.get("eventType").and_then(|x| x.as_str()) == Some("timeout"))
        .count() as i64;
    let health = if timeouts == 0 {
        1.0
    } else {
        (1.0 - timeouts as f32 * 0.1).max(0.0)
    };
    {
        // `.dm-log-health` — `padding: 2px 0 8px; gap: 6px` around the ring.
        let r = dm_body_row(ui, 2.0 + 22.0 + 8.0);
        dm_health_ring(ui, p, pos2(r.min.x, r.min.y + 2.0), health, timeouts);
    }

    let n = state.dm_events.len();
    let list_h = (n as f32 * 29.0).min(350.0); // `.dm-log-list { max-height }`
    let r = dm_body_row(ui, list_h);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(r).id_salt("dm-log-wrap"),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dm-log-list")
                .max_height(r.height())
                .show(ui, |ui| {
                    for ev in state.dm_events.iter() {
                        let ty = ev.get("eventType").and_then(|x| x.as_str()).unwrap_or("info");
                        let ts = ev.get("timestamp").and_then(|x| x.as_i64()).unwrap_or(0);
                        let detail = ev.get("detail").and_then(|x| x.as_str()).unwrap_or("");
                        let (er, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 29.0), Sense::hover());
                        // `.dm-log-time` — 12px mono, `text-dim`.
                        let mono = FontId::new(12.0, egui::FontFamily::Monospace);
                        let time = dm_format_log_time(ts);
                        let tw = dm_text_w(ui, &time, &mono);
                        ui.painter().text(
                            pos2(er.min.x, er.center().y),
                            Align2::LEFT_CENTER,
                            time,
                            mono.clone(),
                            p.text_dim,
                        );
                        // `.dm-log-badge` — `1px 6px`, 10/600 uppercase.
                        let (fg, bg) = dm_log_badge(p, ty);
                        let bfont = FontId::new(10.0, tokens::typography::semibold_family());
                        let blabel = ty.replace('_', " ").to_uppercase();
                        let btw = dm_text_w(ui, &blabel, &bfont);
                        let bth = dm_text_size(ui, &blabel, &bfont).1;
                        let brect = Rect::from_min_size(
                            pos2(er.min.x + tw + 10.0, er.center().y - (bth + 2.0) * 0.5),
                            vec2(btw + 12.0, bth + 2.0),
                        );
                        ui.painter().rect_filled(brect, CornerRadius::same(4), bg);
                        ui.painter().text(
                            pos2(brect.min.x + 6.0, brect.center().y),
                            Align2::LEFT_CENTER,
                            blabel,
                            bfont,
                            fg,
                        );
                        // `.dm-log-detail` — 12px mono (`break-all` in CSS; we
                        // ellipsize so the fixed-height row stays one line).
                        let dx = brect.max.x + 10.0;
                        let dlabel = dm_ellipsize(ui, detail, &mono, er.max.x - dx);
                        ui.painter().text(
                            pos2(dx, er.center().y),
                            Align2::LEFT_CENTER,
                            dlabel,
                            mono,
                            p.text_muted,
                        );
                        // `.dm-log-entry` — `border-bottom: 1px` on every row.
                        ui.painter().rect_filled(
                            Rect::from_min_max(pos2(er.min.x, er.max.y - 1.0), er.max),
                            CornerRadius::same(0),
                            p.border,
                        );
                    }
                });
        },
    );
}

/// ProxyField card (`.pf-card`): toggle row, `.pf-controls` grid while
/// enabled, the `.pf-detect-msg` line and the always-present `.pf-actions`
/// divider block (Apply row + `.dm-proxy-message`).
fn dm_proxy_field(ui: &mut Ui, p: &Palette, state: &mut UiState, cctx: &Ctx) {
    let enabled = state.dm_proxy_enabled;
    let detecting = state.dm_detecting;
    let applying = state.dm_proxy_applying;
    let detect_msg = state.dm_detect_msg.clone();
    let proxy_msg = state.dm_proxy_msg.clone();

    // Measure the `.pf-actions` content up front — it feeds the card height.
    let save_font = FontId::new(12.0, tokens::typography::semibold_family());
    let save_label = if applying {
        state.t("Applying...")
    } else {
        state.t("Apply")
    };
    let save_th = dm_text_size(ui, "Ag", &save_font).1.max(15.0);
    let save_bh = 6.0 + save_th + 6.0 + 2.0;
    let actions_h = if enabled {
        save_bh
    } else if proxy_msg.is_some() {
        dm_text_size(ui, "Ag", &FontId::new(12.0, egui::FontFamily::default())).1
    } else {
        0.0
    };

    // Box model: 1px border + `12px 14px` padding + 30px `.pf-row`.
    let mut h = 1.0 + 12.0 + 30.0 + 12.0 + 1.0;
    if enabled {
        // `.pf-controls` — `margin-top: 11; padding-top: 11; border-top: 1`,
        // then the 2 × `DM_FIELD_H` grid with an 8px gap.
        h += 11.0 + 1.0 + 11.0 + DM_FIELD_H * 2.0 + 8.0;
    }
    if let Some((_, msg)) = &detect_msg {
        h += 8.0 + dm_text_size(ui, msg, &FontId::new(11.5, egui::FontFamily::default())).1;
    }
    // `.pf-actions` — divider always drawn (empty content = just the rule).
    h += 11.0 + 1.0 + 11.0 + actions_h;

    let card = dm_body_row(ui, h);
    let (bg, border) = if enabled {
        (
            mk::blend(p.bg_card, p.accent, 0.04),
            mk::blend(p.border, p.accent, 0.18),
        )
    } else {
        (mk::blend(p.bg_card, Color32::WHITE, 0.02), p.border)
    };
    ui.painter().rect_filled(card, CornerRadius::same(12), bg);
    ui.painter().rect_stroke(
        card,
        CornerRadius::same(12),
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    let inner_l = card.min.x + 14.0;
    let inner_r = card.max.x - 14.0;
    let mut y = card.min.y + 12.0;

    // ── `.pf-row` — `web` glyph + label / detect button + toggle ──
    let row = Rect::from_min_max(pos2(inner_l, y), pos2(inner_r, y + 30.0));
    let cy = row.center().y;
    mi::paint_centered(
        ui.painter(),
        pos2(inner_l + 9.0, cy),
        "web",
        18.0,
        if enabled { p.accent_strong } else { p.text_dim },
    );
    ui.painter().text(
        pos2(inner_l + 18.0 + 8.0, cy),
        Align2::LEFT_CENTER,
        state.t("Enable Proxy"),
        FontId::new(13.0, egui::FontFamily::default()),
        if enabled { p.text } else { p.text_muted },
    );
    let toggle = Rect::from_min_size(pos2(inner_r - 42.0, cy - 12.0), vec2(42.0, 24.0));
    let detect = Rect::from_min_size(pos2(toggle.min.x - 8.0 - 30.0, cy - 15.0), vec2(30.0, 30.0));
    let dr = ui.interact(detect, egui::Id::new("dm-px-detect"), Sense::click());
    let detect_hot = dr.hovered() && !detecting;
    if detect_hot {
        // `.pf-detect-btn:hover` — veil + `--border` outline.
        ui.painter().rect_filled(detect, CornerRadius::same(8), p.hover_veil);
        ui.painter().rect_stroke(
            detect,
            CornerRadius::same(8),
            egui::Stroke::new(1.0, p.border),
            egui::StrokeKind::Inside,
        );
    }
    let icon_color = if detect_hot { p.accent_strong } else { p.text_dim };
    // `:disabled { opacity: .5 }`.
    let icon_color = if detecting {
        icon_color.gamma_multiply(0.5)
    } else {
        icon_color
    };
    mi::paint_centered(
        ui.painter(),
        detect.center(),
        if detecting { "hourglass_empty" } else { "wifi_find" },
        17.0,
        icon_color,
    );
    if dr.clicked() && !detecting {
        state.dm_detecting = true;
        state.dm_detect_msg = None;
        cctx.emit("dm-detect", |_app| async move {
            moonhunt_core::aria2::detect_system_proxy().await
        });
    }
    if nd_switch(ui, p, toggle, enabled, "dm-px-toggle") {
        // `watch(model.enabled)` — turning it off auto-applies (silently).
        state.dm_proxy_enabled = !enabled;
        if enabled {
            dm_apply_proxy(state, cctx, true);
        }
    }
    y = row.max.y;

    // ── `.pf-controls` (v-if enabled) ──
    if enabled {
        let (c_host, c_port) = (state.t("Host"), state.t("Port"));
        let (c_user, c_pass) = (state.t("Username"), state.t("Password"));
        let c_opt = state.t("(optional)");
        let c_ph = state.t("Leave blank if none");
        let c_ph_host = state.t("proxy.example.com");
        y += 11.0;
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(inner_l, y), pos2(inner_r, y + 1.0)),
            CornerRadius::same(0),
            p.border,
        );
        y += 1.0 + 11.0;
        let col_w = (inner_r - inner_l - 8.0) / 2.0;
        dm_field(
            ui,
            p,
            egui::Id::new("dm-pf-host"),
            pos2(inner_l, y),
            col_w,
            c_host,
            None,
            &mut state.dm_proxy_host,
            c_ph_host,
            false,
        );
        dm_field(
            ui,
            p,
            egui::Id::new("dm-pf-port"),
            pos2(inner_l + col_w + 8.0, y),
            col_w,
            c_port,
            None,
            &mut state.dm_proxy_port,
            "1080",
            false,
        );
        y += DM_FIELD_H + 8.0;
        dm_field(
            ui,
            p,
            egui::Id::new("dm-pf-user"),
            pos2(inner_l, y),
            col_w,
            c_user,
            Some(c_opt),
            &mut state.dm_proxy_user,
            c_ph,
            false,
        );
        dm_field(
            ui,
            p,
            egui::Id::new("dm-pf-pass"),
            pos2(inner_l + col_w + 8.0, y),
            col_w,
            c_pass,
            Some(c_opt),
            &mut state.dm_proxy_pass,
            c_ph,
            true,
        );
        y += DM_FIELD_H;
    }

    // ── `.pf-detect-msg` (`margin-top: 8px`) ──
    if let Some((ok, msg)) = &detect_msg {
        y += 8.0;
        let th = dm_text_size(ui, msg, &FontId::new(11.5, egui::FontFamily::default())).1;
        ui.painter().text(
            pos2(inner_l, y + th * 0.5),
            Align2::LEFT_CENTER,
            msg,
            FontId::new(11.5, egui::FontFamily::default()),
            if *ok { p.status_complete } else { p.text_dim },
        );
        y += th;
    }

    // ── `.pf-actions` — `margin-top: 11; padding-top: 11; border-top: 1` ──
    y += 11.0;
    ui.painter().rect_filled(
        Rect::from_min_max(pos2(inner_l, y), pos2(inner_r, y + 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    y += 1.0 + 11.0;
    if enabled {
        // `.dm-proxy-save` — accent wash + accent border, icon 15, gap 6.
        let bw = 14.0 + 15.0 + 6.0 + dm_text_w(ui, save_label, &save_font) + 14.0 + 2.0;
        let btn = Rect::from_min_size(pos2(inner_l, y), vec2(bw, save_bh));
        ui.painter()
            .rect_filled(btn, CornerRadius::same(8), mk::blend(p.bg_card, p.accent, 0.12));
        ui.painter().rect_stroke(
            btn,
            CornerRadius::same(8),
            egui::Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.25)),
            egui::StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(btn.min.x + 14.0 + 7.5, btn.center().y),
            if applying { "hourglass_top" } else { "check" },
            15.0,
            p.accent_strong,
        );
        ui.painter().text(
            pos2(btn.min.x + 14.0 + 15.0 + 6.0, btn.center().y),
            Align2::LEFT_CENTER,
            save_label,
            save_font,
            p.accent_strong,
        );
        if applying {
            // `:disabled { opacity: .5 }` — fade toward the card fill.
            ui.painter()
                .rect_filled(btn, CornerRadius::same(8), p.bg_card.gamma_multiply(0.5));
        }
        let rr = ui.interact(btn, egui::Id::new("dm-proxy-save"), Sense::click());
        if rr.clicked() && !applying {
            dm_apply_proxy(state, cctx, false);
        }
        // `.dm-proxy-actions { gap: 10px }`.
        if let Some((ok, msg)) = &proxy_msg {
            ui.painter().text(
                pos2(btn.max.x + 10.0, btn.center().y),
                Align2::LEFT_CENTER,
                msg,
                FontId::new(12.0, egui::FontFamily::default()),
                if *ok { p.status_complete } else { p.status_error },
            );
        }
    } else if let Some((ok, msg)) = &proxy_msg {
        ui.painter().text(
            pos2(inner_l, y + dm_text_size(ui, "Ag", &FontId::new(12.0, egui::FontFamily::default())).1 * 0.5),
            Align2::LEFT_CENTER,
            msg,
            FontId::new(12.0, egui::FontFamily::default()),
            if *ok { p.status_complete } else { p.status_error },
        );
    }
}

// ─── Download-limit alert (port of `DownloadLimitAlert.vue`) ────────────────

/// Side effects owed by the download-limit tracker, drained once per frame:
/// the one-shot `pauseAndNotify` and `persistUsedBytes` (immediately after a
/// reset, otherwise every 10s while the counter is running).
fn limit_tick(state: &mut UiState, cctx: &Ctx) {
    if std::mem::take(&mut state.limit_pause_pending) {
        // The old handler only logs a failure (`console.error`), no toast.
        cctx.rt.spawn(async {
            if let Err(e) = moonhunt_core::aria2::pause_all_downloads().await {
                eprintln!("[!] Failed to pause all downloads: {e}");
            }
        });
    }
    let persist = std::mem::take(&mut state.limit_persist_pending)
        || (state.settings.download_limit_enabled
            && state.settings.download_limit_used_bytes > 0
            && Instant::now() >= state.limit_next_persist);
    if persist {
        state.limit_next_persist = Instant::now() + Duration::from_secs(10);
        let settings = state.settings.clone();
        let core = cctx.core.clone();
        cctx.rt.spawn(async move {
            let _ = moonhunt_core::db::save_all_settings(settings, core).await;
        });
    }
}

/// `DownloadLimitAlert.vue` — blocking two-step dialog: acknowledge the hit,
/// then optionally reset the counter (downloads stay paused either way).
fn download_limit_alert(state: &mut UiState, ctx: &egui::Context) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let mut keep_paused = false;
    let mut acknowledged = false;
    let mut reset = false;

    // `.dla-overlay`: rgba(0,0,0,.75) + 12px backdrop blur (no egui
    // equivalent). The old guard only fires when the press *and* the release
    // land on the overlay — exactly egui's `clicked()` semantics, so dragging
    // a click out of / into the card never dismisses. Releasing out there
    // resolves `{reset:false}` = `handleKeepPaused`.
    egui::Area::new(egui::Id::new("dla-overlay"))
        .order(egui::Order::Foreground)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.painter()
                .rect_filled(screen, CornerRadius::same(0), Color32::from_black_alpha(191));
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                keep_paused = true;
            }
        });

    let (step, used, limit) = (state.dla_step, state.dla_used, state.dla_limit);
    // The steps paint from `&Palette` only, so hand the locale down.
    let lang = state.settings.language.clone();

    // `.dla-modal`: 420px max / 90% wide, 32px padding, dead-centered. Built
    // after the overlay so it stacks above it (egui orders same-`Order` areas
    // back to front by creation).
    egui::Area::new(egui::Id::new("dla-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(Margin::same(32))
                .show(ui, |ui| {
                    let outer = (screen.width() * 0.9).min(420.0);
                    ui.set_width(outer - 64.0);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        if step == 0 {
                            dla_step_alert(ui, &p, used, limit, &mut acknowledged, &lang);
                        } else {
                            dla_step_reset(ui, &p, &mut keep_paused, &mut reset, &lang);
                        }
                    });
                });
        });

    if acknowledged {
        // `handleAcknowledge()` — step 2 replaces the alert in place.
        state.dla_step = 1;
    }
    if reset {
        // `handleReset()` + MainLayout's `resetLimit()` / success notify.
        state.limit_reset();
        state.toast(true, "Download limit reset. Downloads remain paused.");
        state.dla_open = false;
    } else if keep_paused {
        // `handleKeepPaused()` (also the overlay's dismiss): the counter keeps
        // its latched value and downloads stay paused.
        state.dla_open = false;
    }
}

/// Step 1 — the alert itself (`template v-if="step === 'alert'"`).
fn dla_step_alert(
    ui: &mut Ui,
    p: &Palette,
    used: u64,
    limit: u64,
    acknowledged: &mut bool,
    lang: &str,
) {
    dla_icon_wrap(ui, p, "warning", false);
    ui.add_space(16.0);
    ui.label(
        RichText::new(crate::i18n::t(lang, "Download Limit Reached"))
            .size(20.0)
            .family(tokens::typography::bold_family())
            .color(p.text),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(crate::i18n::t(
            lang,
            "You've reached your configured download limit.",
        ))
            .size(14.0)
            .color(p.text_muted),
    );
    ui.add_space(20.0);
    dla_stats(ui, p, lang, used, limit);
    ui.add_space(16.0);
    dla_progress(ui, p);
    ui.add_space(12.0);
    ui.label(
        RichText::new(crate::i18n::t(lang, "All active downloads have been paused."))
            .size(12.0)
            .color(p.text_dim),
    );
    ui.add_space(24.0);
    // `.dla-footer { justify-content: center }`.
    let ack = crate::i18n::t(lang, "I Understand");
    let total = dla_btn_w(ui, ack);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.add_space(((ui.available_width() - total) * 0.5).max(0.0));
        if dla_btn(ui, p, "check", ack, true) {
            *acknowledged = true;
        }
    });
}

/// Step 2 — the reset prompt (`template v-else-if="step === 'reset'"`).
fn dla_step_reset(
    ui: &mut Ui,
    p: &Palette,
    keep_paused: &mut bool,
    reset: &mut bool,
    lang: &str,
) {
    dla_icon_wrap(ui, p, "restart_alt", true);
    ui.add_space(16.0);
    ui.label(
        RichText::new(crate::i18n::t(lang, "Reset Download Limit?"))
            .size(20.0)
            .family(tokens::typography::bold_family())
            .color(p.text),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(crate::i18n::t(
            lang,
            "Would you like to reset the download counter and continue downloading?",
        ))
            .size(14.0)
            .color(p.text_muted),
    );
    ui.add_space(20.0);
    dla_info(
        ui,
        p,
        crate::i18n::t(
            lang,
            "Downloads will remain paused. You can resume them manually after resetting.",
        ),
    );
    ui.add_space(24.0);
    // `.dla-footer.dual { gap: 10px }`.
    let keep = crate::i18n::t(lang, "Keep Paused");
    let reset_label = crate::i18n::t(lang, "Reset & Continue");
    let ghost_w = dla_btn_w(ui, keep);
    let primary_w = dla_btn_w(ui, reset_label);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let total = ghost_w + 10.0 + primary_w;
        ui.add_space(((ui.available_width() - total) * 0.5).max(0.0));
        if dla_btn(ui, p, "pause", keep, false) {
            *keep_paused = true;
        }
        ui.add_space(10.0);
        if dla_btn(ui, p, "restart_alt", reset_label, true) {
            *reset = true;
        }
    });
}

/// `.dla-icon-wrap` / `.reset` — 64px ringed circle carrying a 32px glyph.
fn dla_icon_wrap(ui: &mut Ui, p: &Palette, icon: &str, accent: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
    let tint = if accent { p.accent } else { p.status_error };
    let fg = if accent { p.accent_strong } else { p.status_error };
    ui.painter()
        .circle_filled(rect.center(), 32.0, tint.gamma_multiply(0.12));
    // `border: 2px` sits inside the 64px box (border-box sizing in the old CSS).
    ui.painter().circle_stroke(
        rect.center(),
        31.0,
        egui::Stroke::new(2.0, tint.gamma_multiply(0.25)),
    );
    mi::paint_centered(ui.painter(), rect.center(), icon, 32.0, fg);
}

/// `.dla-stats` — DOWNLOADED / LIMIT columns split by a 1×32 rule.
fn dla_stats(ui: &mut Ui, p: &Palette, lang: &str, used: u64, limit: u64) {
    let used_s = moonhunt_core::util::format_bytes(used);
    let limit_s = moonhunt_core::util::format_bytes(limit);
    let label_font = FontId::new(11.0, tokens::typography::semibold_family());
    let value_font = FontId::new(22.0, tokens::typography::bold_family());
    let downloaded = crate::i18n::t(lang, "DOWNLOADED");
    let limit_lbl = crate::i18n::t(lang, "LIMIT");
    let w1 = dm_text_w(ui, downloaded, &label_font).max(dm_text_w(ui, &used_s, &value_font));
    let w2 = dm_text_w(ui, limit_lbl, &label_font).max(dm_text_w(ui, &limit_s, &value_font));
    let lh = dm_text_size(ui, downloaded, &label_font)
        .1
        .max(dm_text_size(ui, "LIMIT", &label_font).1);
    let vh = dm_text_size(ui, &used_s, &value_font)
        .1
        .max(dm_text_size(ui, &limit_s, &value_font).1);
    let col_h = lh + 4.0 + vh;
    let content_h = col_h.max(32.0);
    let gap = 24.0;
    let total = w1 + gap + 1.0 + gap + w2;

    // `.dla-stats`: bg-surface, 1px border, 12r, 16px padding.
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, content_h + 34.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(12), p.bg_surface);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        egui::Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );

    let x0 = rect.min.x + 17.0 + ((w - 34.0 - total) * 0.5).max(0.0);
    // Columns are vertically centered in the box's content area.
    let top = rect.center().y - col_h / 2.0;
    dla_stat(
        ui,
        p,
        x0 + w1 / 2.0,
        top,
        "DOWNLOADED",
        &used_s,
        &label_font,
        &value_font,
        false,
    );
    let dx = x0 + w1 + gap;
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(dx, rect.center().y - 16.0), vec2(1.0, 32.0)),
        CornerRadius::same(0),
        p.border_strong,
    );
    dla_stat(
        ui,
        p,
        dx + 1.0 + gap + w2 / 2.0,
        top,
        "LIMIT",
        &limit_s,
        &label_font,
        &value_font,
        true,
    );
}

/// One `.dla-stat` column: uppercase 11px label over the 22px value.
fn dla_stat(
    ui: &mut Ui,
    p: &Palette,
    center_x: f32,
    top: f32,
    label: &str,
    value: &str,
    label_font: &FontId,
    value_font: &FontId,
    accent: bool,
) {
    let lh = dm_text_size(ui, label, label_font).1;
    let vh = dm_text_size(ui, value, value_font).1;
    ui.painter().text(
        pos2(center_x, top + lh / 2.0),
        Align2::CENTER_CENTER,
        label,
        label_font.clone(),
        p.text_dim,
    );
    ui.painter().text(
        pos2(center_x, top + lh + 4.0 + vh / 2.0),
        Align2::CENTER_CENTER,
        value,
        value_font.clone(),
        if accent { p.accent_strong } else { p.text },
    );
}

/// `.dla-progress-fill` — the alert only ever shows the moment the limit was
/// hit, so the 6px track is covered edge to edge in `#ef5350`.
fn dla_progress(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(3), p.status_error);
}

/// `.dla-reset-info` — accent-tinted note with a 16px glyph and 12.5px copy.
fn dla_info(ui: &mut Ui, p: &Palette, text: &str) {
    let w = ui.available_width();
    let font = FontId::new(12.5, egui::FontFamily::Proportional);
    // padding 14 + icon 16 + gap 8 — the copy wraps in what is left.
    let galley = ui
        .painter()
        .layout(text.to_owned(), font, p.text_muted, (w - 66.0).max(40.0));
    let (rect, _) = ui.allocate_exact_size(
        vec2(w, 24.0 + galley.rect.height().max(17.0)),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), p.accent.gamma_multiply(0.06));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        egui::Stroke::new(1.0, p.accent.gamma_multiply(0.12)),
        egui::StrokeKind::Inside,
    );
    // `.dla-reset-info .material-icons { margin-top: 1px }`.
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 22.0, rect.min.y + 20.5),
        "info_outline",
        16.0,
        p.accent_strong,
    );
    ui.painter()
        .galley(pos2(rect.min.x + 38.0, rect.min.y + 12.0), galley, p.text_muted);
}

/// `.dla-btn` width: 20px padding + 18px glyph + 6px gap + label + 20px.
fn dla_btn_w(ui: &Ui, label: &str) -> f32 {
    let font = FontId::new(14.0, tokens::typography::semibold_family());
    64.0 + dm_text_w(ui, label, &font)
}

/// One `.dla-btn`: `primary` = red fill with white ink and a 1px hover lift,
/// otherwise hover-veil + border-strong with dim ink that brightens on hover.
/// Returns whether it was clicked.
fn dla_btn(ui: &mut Ui, p: &Palette, icon: &str, label: &str, primary: bool) -> bool {
    let font = FontId::new(14.0, tokens::typography::semibold_family());
    let tw = dm_text_w(ui, label, &font);
    let (mut rect, resp) = ui.allocate_exact_size(vec2(64.0 + tw, 38.0), Sense::click());
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new("dla-btn").with(label),
        if resp.hovered() { 1.0 } else { 0.0 },
        0.2,
    );
    let (mut fill, mut icon_c, mut text_c) = if primary {
        (p.status_error, Color32::WHITE, Color32::WHITE)
    } else {
        (p.hover_veil, p.text_dim, p.text_dim)
    };
    if primary {
        // `:hover { background:#f44336; transform: translateY(-1px) }`.
        if resp.hovered() {
            rect = rect.translate(vec2(0.0, -1.0));
        }
        fill = mk::blend(fill, Color32::from_rgb(0xf4, 0x43, 0x36), t);
    } else {
        fill = mk::blend(fill, p.text.gamma_multiply(0.10), t);
        icon_c = mk::blend(icon_c, p.text, t);
        text_c = mk::blend(text_c, p.text, t);
    }
    ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
    if !primary {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(10),
            egui::Stroke::new(1.0, p.border_strong),
            egui::StrokeKind::Inside,
        );
    }
    // Icon + label centered as a group (`display:inline-flex; gap: 6px`).
    let x = rect.center().x - (18.0 + 6.0 + tw) / 2.0;
    mi::paint_centered(
        ui.painter(),
        pos2(x + 9.0, rect.center().y + 1.0),
        icon,
        18.0,
        icon_c,
    );
    ui.painter().text(
        pos2(x + 24.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        text_c,
    );
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::translate_badge;

    #[test]
    fn proxy_badge_speaks_the_active_locale() {
        // The check task builds `Proxy · 200 · 43ms` with no `UiState` in scope.
        assert_eq!(
            translate_badge("fa", "Proxy · 200 · 43ms"),
            crate::rtl::visual("پروکسی · 200 · 43ms")
        );
        assert_eq!(
            translate_badge("fa", "Proxy · Proxy test failed"),
            crate::rtl::visual("پروکسی · آزمایش پروکسی ناموفق بود")
        );
        // A label the state machine already translated is one word (`Direct`),
        // and one word is its own display order — the paint path hands this
        // helper backend-built strings only, so the pass happens exactly once.
        let direct = crate::i18n::t("fa", "Direct");
        assert_eq!(translate_badge("fa", direct), direct);
        // English locale keeps the literal.
        assert_eq!(translate_badge("en", "Proxy"), "Proxy");
    }
}
