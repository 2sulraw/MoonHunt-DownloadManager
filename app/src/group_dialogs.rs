//! `DownloadsPage.vue` group dialogs: Create Group (`showCreateGroupDialog`),
//! Edit Group (`renameGroupId`) and the group Downloads sub-modal
//! (`showGroupDownloads`). All three share the `.nd-modal` shell from the
//! new-download dialog — 36px avatar tile, 14px title + 11px subtitle, hairline
//! divider, scrolling body and a footer — with the group-specific body
//! (`.color-picker-row`, `.group-downloads-btn`, the Options section with the
//! speed/download limits) ported 1:1 from the old markup.

use crate::screens::Ctx;
use crate::state::{GroupPendingDl, UiState};
use crate::ui::icons as mi;
use crate::ui::tokens::{self, Palette};
use crate::ui::widgets as mk;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Margin, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui,
};

/// `.nd-modal { max-width: 480px; width: 100% }`.
const GW: f32 = 480.0;
/// `.group-downloads-modal { max-width: 400px }`.
const GDM_W: f32 = 400.0;
/// `.nd-header` / `.nd-body` / `.nd-footer` side padding.
const PAD: f32 = 16.0;
const HEADER_H: f32 = 60.0;
const FOOTER_H: f32 = 62.0;
/// `.nd-label` line box (12px text + the 14px icon).
const LBL_H: f32 = 15.0;
/// `.nd-field { gap: 7px }`.
const FIELD_GAP: f32 = 7.0;
/// `.nd-section { gap: 6px }`.
const ROW_GAP: f32 = 6.0;
/// `.nd-section-title` row.
const SECT_H: f32 = 16.0;
/// `.nd-input` height.
const INPUT_H: f32 = 36.0;

/// `onCompleteOptions` — `[value, label]` in the reference order.
const ON_COMPLETE: [(&str, &str); 5] = [
    ("none", "Do nothing"),
    ("close", "Close app"),
    ("sleep", "Sleep"),
    ("shutdown", "Shutdown"),
    ("restart", "Restart"),
];

const UNIT_LABELS: [&str; 3] = ["KB", "MB", "GB"];

/// `groupDlMultipliers`.
fn unit_multiplier(unit: usize) -> u64 {
    match unit {
        0 => 1024,
        2 => 1_073_741_824,
        _ => 1_048_576,
    }
}

/// `formatGroupLimitDisplay`.
fn format_limit(bytes: u64) -> String {
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

/// Draw the dialogs that are open. Called from `modals::show` after the
/// Downloads screen so the overlays cover it; the Downloads sub-modal is drawn
/// last, so it stacks above its parent (`z-index: 11000`).
pub fn show(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    if state.g_create_open {
        form(state, ctx, cctx, false);
    } else if state.g_edit.is_some() {
        form(state, ctx, cctx, true);
    }
    if state.g_dl_open {
        downloads_modal(state, ctx, cctx);
    }
}

// ─── Shell ──────────────────────────────────────────────────────────────────

/// Full-width flow row inset by the modal padding (the body's own scrollbar
/// must not pull the padding inward).
fn row(ui: &mut Ui, w: f32, h: f32, pad: f32) -> Rect {
    let avail = ui.available_width().max(1.0);
    let (r, _) = ui.allocate_exact_size(vec2(avail, h), Sense::hover());
    Rect::from_min_size(pos2(r.min.x + pad, r.min.y), vec2(w - 2.0 * pad, h))
}

/// `.nd-divider`: 1px `linear-gradient(90deg, transparent, border, transparent)`.
fn gradient_rule(ui: &Ui, rect: Rect, color: Color32) {
    let painter = ui.painter();
    let steps = 48;
    let w = rect.width() / steps as f32;
    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32;
        let a = 1.0 - (t - 0.5).abs() * 2.0;
        let alpha = (color.a() as f32 * a).round() as u8;
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.min.x + i as f32 * w, rect.min.y), vec2(w, rect.height())),
            CornerRadius::same(0),
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
        );
    }
}

/// Paint a Material glyph rotated about its center (`transition: transform`).
fn rotated_icon(painter: &egui::Painter, center: egui::Pos2, name: &str, size: f32, color: Color32, angle: f32) {
    let galley = painter.layout_no_wrap(
        mi::glyph(name).to_string(),
        mi::font(size),
        color,
    );
    let half = galley.size() * 0.5;
    let (s, c) = angle.sin_cos();
    // The text box is rotated about `pos`; shift it so its center lands on
    // `center` after the rotation.
    let off = vec2(half.x * c - half.y * s, half.x * s + half.y * c);
    let mut shape = egui::epaint::TextShape::new(center - off, galley, color);
    shape.angle = angle;
    painter.add(shape);
}

/// `.nd-header` + `.nd-divider`. Returns `(close_clicked, options_clicked)`.
fn header(
    ui: &mut Ui,
    p: &Palette,
    icon: &str,
    title: &str,
    options_open: Option<bool>,
    lang: &str,
) -> (bool, Option<bool>) {
    let (h_rect, _) = ui.allocate_exact_size(vec2(GW, HEADER_H), Sense::hover());
    let painter = ui.painter();
    let avatar = Rect::from_min_size(
        pos2(h_rect.min.x + PAD, h_rect.min.y + 12.0),
        vec2(36.0, 36.0),
    );
    // `.nd-avatar`: `linear-gradient(135deg, accent .12, accent .04)` + 1px ring.
    painter.rect_filled(avatar, CornerRadius::same(10), mk::blend(p.bg_card, p.accent, 0.10));
    painter.rect_stroke(
        avatar,
        CornerRadius::same(10),
        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.18)),
        StrokeKind::Inside,
    );
    mi::paint_centered(painter, avatar.center(), icon, 18.0, p.accent_strong);
    painter.text(
        pos2(avatar.max.x + 10.0, h_rect.min.y + 16.0),
        Align2::LEFT_TOP,
        title,
        FontId::new(14.0, tokens::typography::semibold_family()),
        p.text,
    );
    painter.text(
        pos2(avatar.max.x + 10.0, h_rect.min.y + 36.0),
        Align2::LEFT_TOP,
        crate::i18n::t(lang, "Group"),
        FontId::new(11.0, egui::FontFamily::default()),
        p.text_muted,
    );

    // `.nd-header-actions`: [tune][close], 28px squares, 6px apart.
    let close = Rect::from_center_size(
        pos2(h_rect.max.x - PAD - 14.0, h_rect.min.y + 12.0 + 18.0),
        vec2(28.0, 28.0),
    );
    let mut close_clicked = false;
    let mut opt_clicked = None;
    if let Some(open) = options_open {
        let tune = Rect::from_center_size(
            pos2(close.min.x - 6.0 - 14.0, close.center().y),
            vec2(28.0, 28.0),
        );
        let rr = ui.interact(tune, egui::Id::new("grp-options"), Sense::click());
        // `.group-options-toggle.active { color: accent-strong; background: accent .1 }`
        if open {
            painter.rect_filled(tune, CornerRadius::same(7), mk::blend(p.bg_card, p.accent, 0.10));
        } else if rr.hovered() {
            painter.rect_filled(tune, CornerRadius::same(7), mk::blend(p.bg_card, Color32::WHITE, 0.06));
        }
        // The CSS rotates the glyph -90° while the section is hidden.
        let angle = ui.ctx().animate_value_with_time(
            egui::Id::new("grp-options-rot"),
            if open { 0.0 } else { -std::f32::consts::FRAC_PI_2 },
            0.2,
        );
        rotated_icon(
            painter,
            tune.center(),
            "tune",
            18.0,
            if open || rr.hovered() { p.accent_strong } else { p.text_dim },
            angle,
        );
        if rr.clicked() {
            opt_clicked = Some(!open);
        }
    }
    let crr = ui.interact(close, egui::Id::new("grp-close"), Sense::click());
    if crr.hovered() {
        painter.rect_filled(close, CornerRadius::same(7), mk::blend(p.bg_card, p.status_error, 0.10));
    }
    mi::paint_centered(
        painter,
        close.center(),
        "close",
        16.0,
        if crr.hovered() { p.status_error } else { p.text_dim },
    );
    if crr.clicked() {
        close_clicked = true;
    }

    // `.nd-header { border-bottom: 1px solid border }` + `.nd-divider`.
    let dy = h_rect.max.y;
    painter.rect_filled(
        Rect::from_min_size(pos2(h_rect.min.x, dy), vec2(GW, 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    gradient_rule(
        ui,
        Rect::from_min_size(pos2(h_rect.min.x + PAD, dy), vec2(GW - 2.0 * PAD, 1.0)),
        p.border_strong,
    );
    ui.add_space(1.0);
    (close_clicked, opt_clicked)
}

/// `.nd-footer`: 1px top border, two 38px buttons split evenly.
fn footer(
    ui: &mut Ui,
    p: &Palette,
    w: f32,
    confirm: (bool, &str, &str),
    lang: &str,
) -> (bool, bool) {
    let (f_rect, _) = ui.allocate_exact_size(vec2(w, FOOTER_H), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_size(f_rect.min, vec2(w, 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    let bw = (w - 2.0 * PAD - 8.0) / 2.0;
    let by = f_rect.min.y + 10.0;
    let mut closed = false;
    if crate::modals::nd_button(
        ui,
        p,
        "grp-cancel",
        Rect::from_min_size(pos2(f_rect.min.x + PAD, by), vec2(bw, 38.0)),
        crate::i18n::t(lang, "Cancel"),
        Some("close"),
        false,
        true,
    ) {
        closed = true;
    }
    let (primary, label, icon) = confirm;
    let want = crate::modals::nd_button(
        ui,
        p,
        "grp-confirm",
        Rect::from_min_size(pos2(f_rect.min.x + PAD + bw + 8.0, by), vec2(bw, 38.0)),
        label,
        Some(icon),
        true,
        primary,
    );
    (closed, want)
}

/// `.nd-body { padding: 12px 16px 8px }` — the scroll region of both modals.
fn body_max_height(ctx: &egui::Context, footer: f32) -> f32 {
    (ctx.content_rect().height() * 0.85 - HEADER_H - footer).max(160.0)
}

// ─── Inputs ─────────────────────────────────────────────────────────────────

/// `.nd-input`: 36px box, faint white inset, accent stroke while focused.
/// `pad_r` reserves room for an overlapping affordance (the unit dropdown).
fn text_input(
    ui: &mut Ui,
    p: &Palette,
    id: egui::Id,
    rect: Rect,
    value: &mut String,
    hint: &str,
    pad_r: i8,
) {
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
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
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );
    let font = FontId::proportional(12.0);
    let line_h = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
    let text_rect = Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect).id(id), |ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .desired_width(text_rect.width())
                .font(font)
                .text_color(p.text)
                .hint_text(RichText::new(hint).size(12.0).color(p.text_dim))
                .frame(egui::Frame::NONE.inner_margin(Margin {
                    left: 10,
                    right: pad_r,
                    top: 0,
                    bottom: 0,
                })),
        );
    });
}

/// `.nd-label`: 14px dim icon + 12px muted text.
fn field_label(ui: &Ui, p: &Palette, rect: Rect, icon: &str, text: &str) {
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 7.0, rect.center().y),
        icon,
        14.0,
        p.text_muted.gamma_multiply(0.7),
    );
    ui.painter().text(
        pos2(rect.min.x + 19.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(12.0, egui::FontFamily::default()),
        p.text_muted,
    );
}

/// `.nd-section-title`: 10px uppercase, dimmed to 70%.
fn section_title(ui: &Ui, p: &Palette, rect: Rect, text: &str) {
    ui.painter().text(
        pos2(rect.min.x, rect.center().y),
        Align2::LEFT_CENTER,
        text.to_uppercase(),
        FontId::new(10.0, tokens::typography::bold_family()),
        p.text_dim.gamma_multiply(0.7),
    );
}

/// `.color-picker-row` + `.color-swatch`: 16 24px dots, 6px apart, wrapping at
/// the field width; the selected one gets a white ring on an accent halo and
/// scales to 1.1 (hover 1.15).
fn swatches(ui: &mut Ui, p: &Palette, rect: Rect, selected: usize) -> Option<usize> {
    const D: f32 = 24.0;
    const GAP: f32 = 6.0;
    let per_row = (((rect.width() + GAP) / (D + GAP)).floor() as usize).max(1);
    let mut picked = None;
    for i in 0..crate::groups::GROUP_COLORS.len() {
        let center = pos2(
            rect.min.x + (i % per_row) as f32 * (D + GAP) + D * 0.5,
            rect.min.y + (i / per_row) as f32 * (D + GAP) + D * 0.5,
        );
        let hit = Rect::from_center_size(center, vec2(D, D)).expand(3.0);
        let rr = ui.interact(hit, egui::Id::new("grp-swatch").with(i), Sense::click());
        let chosen = i == selected;
        let hover = ui.ctx().animate_value_with_time(
            egui::Id::new("grp-swatch-h").with(i),
            if rr.hovered() { 1.0 } else { 0.0 },
            0.15,
        );
        let scale = if chosen { 1.10 } else { 1.0 + 0.15 * hover };
        let rad = D * 0.5 * scale;
        let (r, g, b) = crate::groups::hex_to_rgb(crate::groups::GROUP_COLORS[i]);
        if chosen {
            // box-shadow: 0 0 0 2px accent, just outside the 2px white border.
            ui.painter()
                .circle_stroke(center, rad + 1.0, Stroke::new(2.0, p.accent));
        }
        ui.painter()
            .circle_filled(center, rad, Color32::from_rgb(r, g, b));
        // border: 2px solid #fff (inside the swatch box).
        if chosen {
            ui.painter()
                .circle_stroke(center, rad - 1.0, Stroke::new(2.0, Color32::WHITE));
        }
        if rr.clicked() {
            picked = Some(i);
        }
    }
    picked
}

/// `.group-downloads-btn.nd-btn-secondary`: full-width row, icon + label and a
/// right-aligned member count pill.
fn downloads_button(ui: &mut Ui, p: &Palette, rect: Rect, count: usize, lang: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new("grp-dlbtn"), Sense::click());
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new("grp-dlbtn-t"),
        if rr.hovered() { 1.0 } else { 0.0 },
        0.12,
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(8),
        mk::blend(p.hover_veil, mk::blend(p.bg_card, p.accent, 0.08), t),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, mk::blend(p.border, mk::blend(p.bg_card, p.accent, 0.20), t)),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        painter,
        pos2(rect.min.x + 10.0 + 8.5, rect.center().y),
        "playlist_add",
        17.0,
        p.accent_strong,
    );
    painter.text(
        pos2(rect.min.x + 35.0, rect.center().y),
        Align2::LEFT_CENTER,
        crate::i18n::t(lang, "Downloads"),
        FontId::new(12.0, egui::FontFamily::default()),
        mk::blend(p.text, p.accent_strong, t),
    );
    if count > 0 {
        let font = FontId::new(11.0, tokens::typography::semibold_family());
        let label = count.to_string();
        let tw = painter
            .layout_no_wrap(label.clone(), font.clone(), Color32::WHITE)
            .rect
            .width();
        let w = tw + 16.0;
        let pill = Rect::from_min_size(
            pos2(rect.max.x - 10.0 - w, rect.center().y - 9.0),
            vec2(w, 18.0),
        );
        painter.rect_filled(
            pill,
            CornerRadius::same(9),
            mk::blend(p.bg_card, p.accent, 0.12),
        );
        painter.text(pill.center(), Align2::CENTER_CENTER, label, font, p.accent_strong);
    }
    rr.clicked()
}

/// `.speed-toggle`: 36×20 pill, 14px knob sliding 16px on accent.
fn speed_toggle(ui: &mut Ui, p: &Palette, rect: Rect, on: bool, id: &str) -> bool {
    let rr = ui.interact(rect, egui::Id::new(id), Sense::click());
    let t = ui
        .ctx()
        .animate_value_with_time(egui::Id::new(id).with("t"), if on { 1.0 } else { 0.0 }, 0.25);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(10),
        mk::blend(p.hover_veil, mk::blend(p.bg_card, p.accent, 0.25), t),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(
            1.0,
            mk::blend(p.border_strong, mk::blend(p.bg_card, p.accent, 0.40), t),
        ),
        StrokeKind::Inside,
    );
    let knob_x = rect.min.x + 2.0 + 14.0 * 0.5 + t * 16.0;
    painter.circle_filled(
        pos2(knob_x, rect.center().y),
        7.0,
        mk::blend(p.text_dim, p.accent, t),
    );
    rr.clicked()
}

/// `.speed-toggle-row`: toggle + On/Off label + right-aligned hint.
fn toggle_row(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    on: bool,
    hint: &str,
    id: &str,
    lang: &str,
) -> bool {
    let toggle = Rect::from_min_size(pos2(rect.min.x, rect.center().y - 10.0), vec2(36.0, 20.0));
    let clicked = speed_toggle(ui, p, toggle, on, id);
    let painter = ui.painter();
    painter.text(
        pos2(toggle.max.x + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        crate::i18n::t(lang, if on { "On" } else { "Off" }),
        FontId::new(12.0, tokens::typography::semibold_family()),
        p.text,
    );
    painter.text(
        pos2(rect.max.x, rect.center().y),
        Align2::RIGHT_CENTER,
        hint,
        FontId::new(11.0, egui::FontFamily::default()),
        p.text_dim,
    );
    clicked
}

/// `.stepper`: [−][value][+] in one 38px bordered strip, min 100, step 100.
fn stepper(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    value: &mut u64,
    text: &mut String,
    id: &str,
) {
    let strip = Rect::from_min_size(rect.min, vec2(128.0, 38.0));
    ui.painter().rect_filled(strip, CornerRadius::same(8), p.bg_card);
    ui.painter().rect_stroke(
        strip,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
    for (i, (icon, enabled)) in [("remove", *value > 100), ("add", true)].iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(strip.min.x + i as f32 * 92.0, strip.min.y),
            vec2(36.0, strip.height()),
        );
        let rr = ui.interact(r, egui::Id::new(id).with(("step", i)), Sense::click());
        // `.stepper button` sits under the strip's radius: only its outer
        // corners are rounded.
        let corner = if i == 0 {
            CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 }
        } else {
            CornerRadius { nw: 0, sw: 0, ne: 8, se: 8 }
        };
        if *enabled {
            ui.painter().rect_filled(
                r,
                corner,
                if rr.hovered() {
                    mk::blend(p.hover_veil, mk::blend(p.bg_card, p.accent, 0.10), 0.8)
                } else {
                    p.hover_veil
                },
            );
        }
        mi::paint_centered(
            ui.painter(),
            r.center(),
            icon,
            18.0,
            if !*enabled {
                p.text_dim.gamma_multiply(0.35)
            } else if rr.hovered() {
                p.accent_strong
            } else {
                p.text_dim
            },
        );
        if rr.clicked() && *enabled {
            if i == 0 {
                *value = (*value).saturating_sub(100).max(100);
            } else {
                *value += 100;
            }
            *text = value.to_string();
        }
    }
    // The input sits between the two buttons (56px, `--bg-surface`).
    let input = Rect::from_min_size(
        pos2(strip.min.x + 36.0, strip.min.y),
        vec2(56.0, strip.height()),
    );
    ui.painter().rect_filled(input, CornerRadius::same(0), p.bg_surface);
    ui.painter().vline(
        input.min.x,
        input.y_range(),
        Stroke::new(1.0, p.border_strong),
    );
    ui.painter().vline(
        input.max.x,
        input.y_range(),
        Stroke::new(1.0, p.border_strong),
    );
    let font = FontId::new(13.0, egui::FontFamily::default());
    let tid = egui::Id::new(id).with("text");
    let focused = ui.ctx().memory(|m| m.has_focus(tid));
    let line_h = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_rect = Rect::from_min_size(
        pos2(input.min.x + 4.0, input.center().y - line_h * 0.5),
        vec2(input.width() - 8.0, line_h),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect).id(tid), |ui| {
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(text_rect.width())
                .font(font)
                .horizontal_align(egui::Align::Center)
                .text_color(p.text)
                .frame(egui::Frame::NONE),
        );
    });
    if !focused {
        // `@change` clamps to `Math.max(100, Number(value) || 100)`.
        let parsed = text.trim().parse::<u64>().unwrap_or(100).max(100);
        *value = parsed;
        *text = parsed.to_string();
    } else if let Ok(parsed) = text.trim().parse::<u64>() {
        *value = parsed.max(100);
    }
}

/// One dropdown trigger + its popup list. Returns the picked index.
fn dropdown(
    ui: &mut Ui,
    p: &Palette,
    open: &mut Option<String>,
    id: &str,
    rect: Rect,
    options: &[&str],
    current: usize,
    style: DdStyle,
) -> Option<usize> {
    let is_open = open.as_deref() == Some(id);
    let rr = ui.interact(rect, egui::Id::new(id).with("trg"), Sense::click());
    let painter = ui.painter();
    if style.transparent {
        if rr.hovered() || is_open {
            painter.rect_filled(
                rect,
                CornerRadius::same(style.radius),
                mk::blend(p.bg_card, p.accent, 0.10),
            );
        }
    } else {
        painter.rect_filled(rect, CornerRadius::same(style.radius), mk::blend(p.bg_card, Color32::WHITE, 0.025));
        painter.rect_stroke(
            rect,
            CornerRadius::same(style.radius),
            Stroke::new(
                1.0,
                if is_open {
                    p.accent_strong
                } else if rr.hovered() {
                    mk::blend(p.border, Color32::WHITE, 0.4)
                } else {
                    p.border
                },
            ),
            StrokeKind::Inside,
        );
    }
    let label = options.get(current).copied().unwrap_or("");
    let font = FontId::new(style.font, egui::FontFamily::default());
    if style.centered {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            font,
            if style.transparent && (rr.hovered() || is_open) {
                p.accent_strong
            } else if style.transparent {
                p.text_muted
            } else {
                p.text
            },
        );
    } else {
        let clip = Rect::from_min_max(
            pos2(rect.min.x + 10.0, rect.min.y),
            pos2(rect.max.x - 26.0, rect.max.y),
        );
        ui.painter().with_clip_rect(clip).text(
            pos2(clip.min.x, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font,
            p.text,
        );
        if style.arrow {
            mi::paint_centered(
                ui.painter(),
                pos2(rect.max.x - 14.0, rect.center().y),
                "expand_more",
                16.0,
                p.text_dim,
            );
        }
    }
    if rr.clicked() {
        *open = if is_open { None } else { Some(id.to_owned()) };
    }
    if !is_open {
        return None;
    }

    // `.custom-dropdown-list`: 4px inset card, 30px items.
    let w = if style.popup_w > 0.0 {
        style.popup_w
    } else {
        rect.width()
    };
    let x = if style.popup_right {
        rect.max.x - w
    } else {
        rect.min.x
    };
    let item_h = 30.0;
    let h = 8.0 + item_h * options.len() as f32;
    let pop = Rect::from_min_size(pos2(x, rect.max.y + 4.0), vec2(w, h));
    let mut picked = None;
    egui::Area::new(egui::Id::new(id).with("pop"))
        .order(egui::Order::Foreground)
        .fixed_pos(pop.min)
        .show(ui.ctx(), |ui| {
            ui.set_width(w);
            ui.painter().rect_filled(pop, CornerRadius::same(10), p.bg_card);
            ui.painter().rect_stroke(
                pop,
                CornerRadius::same(10),
                Stroke::new(1.0, p.border_strong),
                StrokeKind::Inside,
            );
            for (i, label) in options.iter().enumerate() {
                let r = Rect::from_min_size(
                    pos2(pop.min.x + 4.0, pop.min.y + 4.0 + i as f32 * item_h),
                    vec2(w - 8.0, item_h),
                );
                let irr = ui.interact(
                    r,
                    egui::Id::new(id).with(("opt", i)),
                    Sense::click(),
                );
                let active = i == current;
                if irr.hovered() || active {
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
                ui.painter().text(
                    if style.centered {
                        r.center()
                    } else {
                        pos2(r.min.x + 8.0, r.center().y)
                    },
                    if style.centered {
                        Align2::CENTER_CENTER
                    } else {
                        Align2::LEFT_CENTER
                    },
                    *label,
                    FontId::new(if style.centered { 11.0 } else { 13.0 }, egui::FontFamily::default()),
                    if active { p.accent_strong } else { p.text },
                );
                if irr.clicked() {
                    picked = Some(i);
                }
            }
        });
    if picked.is_some() {
        *open = None;
        return picked;
    }
    // Outside press closes (skip the frame the trigger toggled).
    if !rr.clicked() {
        let (pressed, pos) =
            ui.ctx()
                .input(|i| (i.pointer.any_pressed(), i.pointer.interact_pos()));
        if pressed {
            if let Some(q) = pos {
                if !pop.contains(q) && !rect.contains(q) {
                    *open = None;
                }
            }
        }
    }
    None
}

/// Metrics of one dropdown flavour (`.group-nd-modal` restyles the trigger).
#[derive(Clone, Copy)]
struct DdStyle {
    radius: u8,
    font: f32,
    arrow: bool,
    centered: bool,
    /// `.dl-limit-unit-custom .custom-dropdown-trigger` paints no chrome.
    transparent: bool,
    /// `.custom-dropdown-list { left: auto; right: 0 }` for the unit picker.
    popup_right: bool,
    popup_w: f32,
}

/// `.limit-usage-bar`: 6px track + accent fill + "used / limit" line.
fn usage_bar(ui: &mut Ui, p: &Palette, rect: Rect, used: u64, limit: u64) {
    let track = Rect::from_min_size(rect.min, vec2(rect.width(), 6.0));
    ui.painter()
        .rect_filled(track, CornerRadius::same(3), Color32::from_rgba_unmultiplied(255, 255, 255, 15));
    let pct = if limit > 0 {
        (used as f32 / limit as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if pct > 0.0 {
        ui.painter().rect_filled(
            Rect::from_min_size(track.min, vec2(track.width() * pct, 6.0)),
            CornerRadius::same(3),
            p.accent,
        );
    }
    ui.painter().text(
        pos2(rect.min.x, track.max.y + 6.0 + 7.0),
        Align2::LEFT_CENTER,
        format!("{} / {}", format_limit(used), format_limit(limit)),
        FontId::new(12.0, tokens::typography::semibold_family()),
        p.text_dim,
    );
}

// ─── Create / Edit ──────────────────────────────────────────────────────────

/// `confirmCreateGroup` / `confirmRenameGroup` need the same dialog body; the
/// only differences are the title/icon and which staged list they own.
fn form(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx, edit: bool) {
    let p = mk::pal();
    let screen = ctx.content_rect();

    // Frame snapshot; written back at the end of the pass.
    let mut name = if edit {
        state.g_edit_name.clone()
    } else {
        state.g_create_name.clone()
    };
    let mut color = if edit {
        state.g_edit_color
    } else {
        state.g_create_color
    };
    let mut options = if edit {
        state.g_edit_options
    } else {
        state.g_create_options
    };
    let mut on_complete = if edit {
        state.g_edit_on_complete
    } else {
        state.g_create_on_complete
    };
    let mut speed = if edit {
        state.g_edit_speed
    } else {
        state.g_create_speed
    };
    let mut dl_on = if edit {
        state.g_edit_dl_on
    } else {
        state.g_create_dl_on
    };
    let mut dl_unit = if edit {
        state.g_edit_dl_unit
    } else {
        state.g_create_dl_unit
    };
    let dl_used = if edit {
        state.g_edit_dl_used
    } else {
        0
    };
    let mut members = if edit {
        state.g_edit_members.clone()
    } else {
        state.g_create_members.clone()
    };
    let mut speed_text = std::mem::take(&mut state.g_speed_text);
    let mut dl_value_text = std::mem::take(&mut state.g_dl_value_text);
    let mut dd_open = state.g_dd_open.take();

    let mut want_close = false;
    let mut want_save = false;
    let mut want_downloads = false;

    // `closeOutsidePress` / `closeOutsideRelease` — a click on the dim backdrop
    // dismisses the dialog.
    {
        let mut outside = false;
        egui::Area::new(egui::Id::new("grp-overlay").with(edit))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.painter().rect_filled(
                    screen,
                    CornerRadius::same(0),
                    Color32::from_black_alpha(140),
                );
                if ui.allocate_rect(screen, Sense::click()).clicked() {
                    outside = true;
                }
            });
        if outside {
            eprintln!("[DBG] group overlay outside click");
            want_close = true;
        }
    }

    egui::Area::new(egui::Id::new("grp-modal").with(edit))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.border_strong))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(GW);
                    // The header/body/footer are exact rows; default item
                    // spacing would add gaps and overshoot the 85vh cap.
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let (closed, tune) = header(
                        ui,
                        &p,
                        if edit { "edit" } else { "folder_shared" },
                        if edit { state.t("Edit Group") } else { state.t("Create Group") },
                        Some(options),
                        &state.settings.language,
                    );
                    if closed {
                        want_close = true;
                    }
                    if let Some(open) = tune {
                        options = open;
                    }

                    // ── Body (`.nd-body`: 12/16/8 padding, 12px section gap) ──
                    let s = &mut ui.spacing_mut().scroll;
                    s.bar_width = 4.0;
                    s.floating = false;
                    egui::ScrollArea::vertical()
                        .id_salt(("grp-body", edit))
                        .auto_shrink([false, true])
                        .max_height(body_max_height(ctx, FOOTER_H))
                        .show(ui, |ui| {
                            ui.set_width(GW);
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            ui.add_space(12.0);

                            // ── Details ──
                            let sect = row(ui, GW, SECT_H, PAD);
                            section_title(ui, &p, sect, state.t("Details"));
                            ui.add_space(ROW_GAP);

                            let lbl = row(ui, GW, LBL_H, PAD);
                            field_label(ui, &p, lbl, "badge", state.t("Group Name"));
                            ui.add_space(FIELD_GAP);
                            let name_rect = row(ui, GW, INPUT_H, PAD);
                            text_input(
                                ui,
                                &p,
                                egui::Id::new(if edit { "grp-name-e" } else { "grp-name-c" }),
                                name_rect,
                                &mut name,
                                if edit { state.t("Group name...") } else { state.t("Group name placeholder") },
                                10,
                            );
                            ui.add_space(ROW_GAP);

                            let lbl = row(ui, GW, LBL_H, PAD);
                            field_label(ui, &p, lbl, "palette", state.t("Color"));
                            ui.add_space(FIELD_GAP);
                            let inner_w = GW - 2.0 * PAD;
                            let per_row = (((inner_w + 6.0) / 30.0).floor() as usize).max(1);
                            let rows = crate::groups::GROUP_COLORS.len().div_ceil(per_row);
                            let sw_h = rows as f32 * 24.0 + (rows - 1) as f32 * 6.0;
                            let sw = row(ui, GW, sw_h, PAD);
                            if let Some(idx) = swatches(ui, &p, sw, color) {
                                color = idx;
                            }
                            ui.add_space(ROW_GAP);

                            let btn = row(ui, GW, 36.0, PAD);
                            if downloads_button(ui, &p, btn, members.len(), &state.settings.language) {
                                want_downloads = true;
                            }

                            // ── Options (revealed by the header's tune toggle) ──
                            if options {
                                ui.add_space(12.0);
                                let sect = row(ui, GW, SECT_H, PAD);
                                section_title(ui, &p, sect, state.t("Options"));
                                ui.add_space(ROW_GAP);

                                let lbl = row(ui, GW, LBL_H, PAD);
                                field_label(ui, &p, lbl, "done_all", state.t("When all downloads complete"));
                                ui.add_space(FIELD_GAP);
                                let dd = row(ui, GW, INPUT_H, PAD);
                                let labels: Vec<&str> =
                                    ON_COMPLETE.iter().map(|(_, label)| state.t(label)).collect();
                                if let Some(i) = dropdown(
                                    ui,
                                    &p,
                                    &mut dd_open,
                                    if edit { "grp-oncomplete-e" } else { "grp-oncomplete-c" },
                                    dd,
                                    &labels,
                                    on_complete,
                                    DdStyle {
                                        radius: 8,
                                        font: 12.0,
                                        arrow: true,
                                        centered: false,
                                        transparent: false,
                                        popup_right: false,
                                        popup_w: 0.0,
                                    },
                                ) {
                                    on_complete = i;
                                }
                                ui.add_space(ROW_GAP);

                                let lbl = row(ui, GW, LBL_H, PAD);
                                field_label(ui, &p, lbl, "speed", state.t("Speed Limit"));
                                ui.add_space(FIELD_GAP);
                                let hint = if speed > 0 {
                                    format!("{speed} KB/s")
                                } else {
                                    state.t("Unlimited").to_owned()
                                };
                                let trow = row(ui, GW, 20.0, PAD);
                                if toggle_row(
                                    ui,
                                    &p,
                                    trow,
                                    speed > 0,
                                    &hint,
                                    if edit { "grp-speed-e" } else { "grp-speed-c" },
                                    &state.settings.language,
                                ) {
                                    if speed > 0 {
                                        speed = 0;
                                    } else {
                                        speed = 200;
                                    }
                                    speed_text = speed.to_string();
                                }
                                if speed > 0 {
                                    ui.add_space(FIELD_GAP);
                                    let srect = row(ui, GW, 38.0, PAD);
                                    stepper(
                                        ui,
                                        &p,
                                        srect,
                                        &mut speed,
                                        &mut speed_text,
                                        if edit { "grp-stepper-e" } else { "grp-stepper-c" },
                                    );
                                }
                                ui.add_space(ROW_GAP);

                                let lbl = row(ui, GW, LBL_H, PAD);
                                field_label(ui, &p, lbl, "data_usage", state.t("Download Limit"));
                                ui.add_space(FIELD_GAP);
                                let dl_bytes = if dl_on {
                                    dl_value_bytes(&dl_value_text, dl_unit)
                                } else {
                                    0
                                };
                                let dl_hint = if dl_on {
                                    format_limit(dl_bytes)
                                } else {
                                    state.t("Unlimited").to_owned()
                                };
                                let drow = row(ui, GW, 20.0, PAD);
                                if toggle_row(
                                    ui,
                                    &p,
                                    drow,
                                    dl_on,
                                    &dl_hint,
                                    if edit { "grp-dl-e" } else { "grp-dl-c" },
                                    &state.settings.language,
                                ) {
                                    dl_on = !dl_on;
                                    // `if (enabled && bytes === 0) { bytes = 1MB }`
                                    if dl_on && dl_bytes == 0 {
                                        dl_value_text = "1".to_owned();
                                        dl_unit = 1;
                                    }
                                }

                                if dl_on {
                                    // `.field { margin-bottom: 16px }` inside the
                                    // section's 6px gap.
                                    ui.add_space(10.0);
                                    let lab = row(ui, GW, LBL_H, PAD);
                                    ui.painter().text(
                                        pos2(lab.min.x, lab.center().y),
                                        Align2::LEFT_CENTER,
                                        state.t("LIMIT"),
                                        FontId::new(12.0, tokens::typography::semibold_family()),
                                        p.text_muted,
                                    );
                                    ui.add_space(6.0);
                                    let lrect = row(ui, GW, INPUT_H, PAD);
                                    let unit_w = 48.0;
                                    let input_rect = Rect::from_min_size(
                                        lrect.min,
                                        vec2(lrect.width() - unit_w - 10.0, lrect.height()),
                                    );
                                    text_input(
                                        ui,
                                        &p,
                                        egui::Id::new(if edit { "grp-dlv-e" } else { "grp-dlv-c" }),
                                        input_rect,
                                        &mut dl_value_text,
                                        "e.g. 1",
                                        10,
                                    );
                                    let unit_rect = Rect::from_min_size(
                                        pos2(lrect.max.x - 10.0 - unit_w, lrect.center().y - 13.0),
                                        vec2(unit_w, 26.0),
                                    );
                                    if let Some(u) = dropdown(
                                        ui,
                                        &p,
                                        &mut dd_open,
                                        if edit { "grp-unit-e" } else { "grp-unit-c" },
                                        unit_rect,
                                        &UNIT_LABELS,
                                        dl_unit,
                                        DdStyle {
                                            radius: 7,
                                            font: 11.0,
                                            arrow: false,
                                            centered: true,
                                            transparent: true,
                                            popup_right: true,
                                            popup_w: unit_w,
                                        },
                                    ) {
                                        dl_unit = u;
                                    }
                                    ui.add_space(6.0);
                                    let bytes = dl_value_bytes(&dl_value_text, dl_unit);
                                    let hrow = row(ui, GW, 14.0, PAD);
                                    mi::paint_centered(
                                        ui.painter(),
                                        pos2(hrow.min.x + 7.0, hrow.center().y),
                                        "info_outline",
                                        13.0,
                                        p.accent_strong,
                                    );
                                    ui.painter().text(
                                        pos2(hrow.min.x + 17.0, hrow.center().y),
                                        Align2::LEFT_CENTER,
                                        state.t_of(&format!(
                                            "{} {}",
                                            state.raw("Equivalent to"),
                                            format_limit(bytes)
                                        )),
                                        FontId::new(11.0, egui::FontFamily::default()),
                                        p.text_dim,
                                    );

                                    ui.add_space(10.0);
                                    let ulab = row(ui, GW, LBL_H, PAD);
                                    ui.painter().text(
                                        pos2(ulab.min.x, ulab.center().y),
                                        Align2::LEFT_CENTER,
                                        state.t("USAGE"),
                                        FontId::new(12.0, tokens::typography::semibold_family()),
                                        p.text_muted,
                                    );
                                    ui.add_space(6.0);
                                    let urow = row(ui, GW, 28.0, PAD);
                                    usage_bar(ui, &p, urow, dl_used, bytes);
                                }
                            }
                            ui.add_space(8.0);
                        });

                    let (closed, saved) = footer(
                        ui,
                        &p,
                        GW,
                        (
                            !name.trim().is_empty(),
                            if edit { state.t("Save") } else { state.t("Create") },
                            if edit { "save" } else { "add" },
                        ),
                        &state.settings.language,
                    );
                    if closed {
                        want_close = true;
                    }
                    if saved {
                        want_save = true;
                    }
                });
        });

    // ── Commit this frame's edits ──
    if edit {
        state.g_edit_name = name;
        state.g_edit_color = color;
        state.g_edit_options = options;
        state.g_edit_on_complete = on_complete;
        state.g_edit_speed = speed;
        state.g_edit_dl_on = dl_on;
        state.g_edit_dl_unit = dl_unit;
        state.g_edit_dl_value = dl_value_text.trim().parse().unwrap_or(state.g_edit_dl_value);
        state.g_edit_members = members.clone();
    } else {
        state.g_create_name = name;
        state.g_create_color = color;
        state.g_create_options = options;
        state.g_create_on_complete = on_complete;
        state.g_create_speed = speed;
        state.g_create_dl_on = dl_on;
        state.g_create_dl_unit = dl_unit;
        state.g_create_dl_value = dl_value_text.trim().parse().unwrap_or(state.g_create_dl_value);
        state.g_create_members = members.clone();
    }
    state.g_speed_text = speed_text;
    state.g_dl_value_text = dl_value_text;
    state.g_dd_open = dd_open;

    // ── Actions ──
    if want_downloads {
        state.g_dl_open = true;
        state.g_dl_for_edit = edit;
        state.g_dl_url.clear();
    }
    if want_close {
        if edit {
            state.g_edit = None;
        } else {
            state.g_create_open = false;
        }
    }
    if want_save {
        if edit {
            save_edit(state, cctx);
        } else {
            save_create(state, cctx);
        }
    }
    // Esc closes, Enter confirms (`@keyup.enter="confirmCreateGroup"`).
    if !state.g_dl_open {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if edit {
                state.g_edit = None;
            } else {
                state.g_create_open = false;
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if edit {
                save_edit(state, cctx);
            } else {
                save_create(state, cctx);
            }
        }
    }
}

/// `groupDlBytesFromInput(value, unit)`.
fn dl_value_bytes(text: &str, unit: usize) -> u64 {
    let value = text.trim().parse::<u64>().unwrap_or(0);
    value.saturating_mul(unit_multiplier(unit))
}

/// Apply one group speed cap to its members (`applyGroupLimits`).
fn apply_limits(cctx: &Ctx, gids: Vec<String>, speed_kb: u64) {
    if gids.is_empty() || speed_kb == 0 {
        return;
    }
    let bytes = speed_kb.saturating_mul(1024);
    cctx.run(move |_| async move {
        for gid in gids {
            let _ = moonhunt_core::aria2::apply_download_speed_limit(gid, bytes).await;
        }
        Ok::<String, String>(String::new())
    });
}

/// `submitPendingGroupDownloads` — queue every staged URL and attach the
/// resulting gid to `group_id` once aria2 answers.
fn submit_pending(state: &mut UiState, cctx: &Ctx, group_id: &str, speed_kb: u64) {
    let pending: Vec<GroupPendingDl> = state.g_pending.drain(..).collect();
    for p in pending {
        let group = group_id.to_owned();
        let limit = speed_kb.saturating_mul(1024);
        cctx.emit("grp-pending-add", move |app| async move {
            let resp = moonhunt_core::aria2::add_download(
                app,
                p.url.clone(),
                Some(p.filename.clone()),
                Some(1),
                None,
                None,
                None,
                None,
                None,
            )
            .await?;
            let gid = serde_json::from_str::<serde_json::Value>(&resp)
                .ok()
                .and_then(|v| v.get("gid").and_then(|g| g.as_str()).map(str::to_owned))
                .unwrap_or_default();
            if !gid.is_empty() && limit > 0 {
                let _ = moonhunt_core::aria2::apply_download_speed_limit(gid.clone(), limit).await;
            }
            Ok(serde_json::json!({ "group": group, "gid": gid }))
        });
    }
}

/// `confirmCreateGroup`.
fn save_create(state: &mut UiState, cctx: &Ctx) {
    let name = state.g_create_name.trim().to_owned();
    if name.is_empty() {
        return;
    }
    let members: Vec<String> = state
        .g_create_members
        .iter()
        .filter(|m| !m.starts_with("pending-"))
        .cloned()
        .collect();
    let speed = state.g_create_speed;
    let color = crate::groups::GROUP_COLORS
        .get(state.g_create_color)
        .copied()
        .unwrap_or(crate::groups::GROUP_COLORS[0]);
    let id = state.group_create(&name, color, members.clone());
    let on_complete = ON_COMPLETE
        .get(state.g_create_on_complete)
        .map(|(v, _)| *v)
        .unwrap_or("none");
    let dl_bytes = if state.g_create_dl_on {
        state
            .g_create_dl_value
            .saturating_mul(unit_multiplier(state.g_create_dl_unit))
    } else {
        0
    };
    let dl_on = state.g_create_dl_on;
    state.group_update(&id, move |g| {
        g.on_complete = on_complete.to_owned();
        g.speed_limit = speed;
        g.download_limit_enabled = dl_on;
        g.download_limit_bytes = dl_bytes;
    });
    apply_limits(cctx, members, speed);
    submit_pending(state, cctx, &id, speed);

    // `confirmCreateGroup` resets the form but keeps the colour.
    state.g_create_name.clear();
    state.g_create_on_complete = 0;
    state.g_create_speed = 0;
    state.g_create_dl_on = false;
    state.g_create_dl_value = 1;
    state.g_create_dl_unit = 1;
    state.g_create_members.clear();
    state.g_create_open = false;
    state.g_speed_text.clear();
    state.g_dl_value_text.clear();
    state.g_dd_open = None;
    state.toast(true, "Group created!");
}

/// `confirmRenameGroup`.
fn save_edit(state: &mut UiState, cctx: &Ctx) {
    let Some(id) = state.g_edit.clone() else {
        return;
    };
    let name = state.g_edit_name.trim().to_owned();
    if name.is_empty() {
        return;
    }
    let Some(old) = state.group_by_id(&id).cloned() else {
        state.g_edit = None;
        return;
    };
    let members: Vec<String> = state
        .g_edit_members
        .iter()
        .filter(|m| !m.starts_with("pending-"))
        .cloned()
        .collect();
    let speed = state.g_edit_speed;
    let color = crate::groups::GROUP_COLORS
        .get(state.g_edit_color)
        .copied()
        .unwrap_or(crate::groups::GROUP_COLORS[0]);
    let on_complete = ON_COMPLETE
        .get(state.g_edit_on_complete)
        .map(|(v, _)| *v)
        .unwrap_or("none");
    let dl_on = state.g_edit_dl_on;
    let dl_bytes = if dl_on {
        state
            .g_edit_dl_value
            .saturating_mul(unit_multiplier(state.g_edit_dl_unit))
    } else {
        0
    };
    state.group_update(&id, |g| {
        g.name = name;
        g.color = color.to_owned();
        g.on_complete = on_complete.to_owned();
        g.member_gids = members.clone();
        g.speed_limit = speed;
        g.download_limit_enabled = dl_on;
        g.download_limit_bytes = dl_bytes;
        // `resetGroupDownloadLimit` when the limit is switched off.
        if !dl_on && old.download_limit_enabled {
            g.download_limit_used_bytes = 0;
        }
    });
    if speed != old.speed_limit {
        apply_limits(cctx, members, speed);
    }
    submit_pending(state, cctx, &id, speed);

    state.g_edit = None;
    state.g_dl_open = false;
    state.toast(true, "Group updated");
}

// ─── Group Downloads sub-modal ──────────────────────────────────────────────

/// `displayGroupMembers` — resolve the active dialog's member list against the
/// live downloads and the staged URLs.
struct MemberRow {
    gid: String,
    label: String,
    status: String,
    status_color: Color32,
    pending: bool,
}

fn member_rows(state: &UiState, edit: bool) -> Vec<MemberRow> {
    let members = if edit {
        &state.g_edit_members
    } else {
        &state.g_create_members
    };
    members
        .iter()
        .map(|m| {
            if let Some(seq) = m.strip_prefix("pending-").and_then(|s| s.parse::<u64>().ok()) {
                let url = state
                    .g_pending
                    .iter()
                    .find(|p| p.seq == seq)
                    .map(|p| p.url.clone())
                    .unwrap_or_default();
                return MemberRow {
                    gid: m.clone(),
                    label: url,
                    status: "Pending".to_owned(),
                    status_color: Color32::PLACEHOLDER,
                    pending: true,
                };
            }
            match state.downloads.iter().find(|d| &d.gid == m) {
                Some(d) => MemberRow {
                    gid: m.clone(),
                    label: d.filename.clone(),
                    status: d.status.label().to_owned(),
                    status_color: d.status.color(),
                    pending: false,
                },
                None => MemberRow {
                    gid: m.clone(),
                    label: m.clone(),
                    status: "Waiting".to_owned(),
                    status_color: mk::pal().status_waiting,
                    pending: false,
                },
            }
        })
        .collect()
}

fn downloads_modal(state: &mut UiState, ctx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ctx.content_rect();
    let edit = state.g_dl_for_edit;
    let rows = member_rows(state, edit);
    let mut url = std::mem::take(&mut state.g_dl_url);
    let mut want_close = false;
    let mut want_add = false;
    let mut remove: Option<String> = None;

    {
        let mut outside = false;
        egui::Area::new(egui::Id::new("grp-dl-overlay"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.painter().rect_filled(
                    screen,
                    CornerRadius::same(0),
                    Color32::from_black_alpha(140),
                );
                if ui.allocate_rect(screen, Sense::click()).clicked() {
                    outside = true;
                }
            });
        if outside {
            want_close = true;
        }
    }

    egui::Area::new(egui::Id::new("grp-dl-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.border_strong))
                .inner_margin(Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(GDM_W);
                    // The sub-modal has no tune toggle.
                    let (h_rect, _) = ui.allocate_exact_size(vec2(GDM_W, HEADER_H), Sense::hover());
                    let painter = ui.painter();
                    let avatar = Rect::from_min_size(
                        pos2(h_rect.min.x + PAD, h_rect.min.y + 12.0),
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
                        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.18)),
                        StrokeKind::Inside,
                    );
                    mi::paint_centered(
                        painter,
                        avatar.center(),
                        "playlist_add",
                        18.0,
                        p.accent_strong,
                    );
                    painter.text(
                        pos2(avatar.max.x + 10.0, h_rect.min.y + 16.0),
                        Align2::LEFT_TOP,
                        state.t("Downloads"),
                        FontId::new(14.0, tokens::typography::semibold_family()),
                        p.text,
                    );
                    painter.text(
                        pos2(avatar.max.x + 10.0, h_rect.min.y + 36.0),
                        Align2::LEFT_TOP,
                        state.t("Group"),
                        FontId::new(11.0, egui::FontFamily::default()),
                        p.text_muted,
                    );
                    let close = Rect::from_center_size(
                        pos2(h_rect.max.x - PAD - 14.0, h_rect.min.y + 12.0 + 18.0),
                        vec2(28.0, 28.0),
                    );
                    let crr = ui.interact(close, egui::Id::new("grp-dl-close"), Sense::click());
                    if crr.hovered() {
                        ui.painter().rect_filled(
                            close,
                            CornerRadius::same(7),
                            mk::blend(p.bg_card, p.status_error, 0.10),
                        );
                    }
                    mi::paint_centered(
                        ui.painter(),
                        close.center(),
                        "close",
                        16.0,
                        if crr.hovered() { p.status_error } else { p.text_dim },
                    );
                    if crr.clicked() {
                        want_close = true;
                    }
                    let dy = h_rect.max.y;
                    ui.painter().rect_filled(
                        Rect::from_min_size(pos2(h_rect.min.x, dy), vec2(GDM_W, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );
                    gradient_rule(
                        ui,
                        Rect::from_min_size(pos2(h_rect.min.x + PAD, dy), vec2(GDM_W - 2.0 * PAD, 1.0)),
                        p.border_strong,
                    );
                    ui.add_space(1.0);

                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    ui.add_space(12.0);

                    // `.nd-input-row`: URL field + 34px add button.
                    let irow = row(ui, GDM_W, 36.0, PAD);
                    let btn_w = 34.0;
                    let input_rect = Rect::from_min_size(
                        irow.min,
                        vec2(irow.width() - 6.0 - btn_w, irow.height()),
                    );
                    text_input(
                        ui,
                        &p,
                        egui::Id::new("grp-dl-url"),
                        input_rect,
                        &mut url,
                        state.t("Paste download URL..."),
                        10,
                    );
                    // `.nd-btn-secondary` add button, disabled while the field
                    // is empty (`opacity: .5`).
                    let add_rect = Rect::from_min_size(
                        pos2(irow.max.x - btn_w, irow.min.y),
                        vec2(btn_w, irow.height()),
                    );
                    let enabled = !url.trim().is_empty();
                    let arr = ui.interact(add_rect, egui::Id::new("grp-dl-add"), Sense::click());
                    ui.painter().rect_filled(
                        add_rect,
                        CornerRadius::same(8),
                        mk::blend(
                            p.hover_veil,
                            mk::blend(p.bg_card, p.accent, 0.08),
                            if arr.hovered() && enabled { 1.0 } else { 0.0 },
                        ),
                    );
                    ui.painter().rect_stroke(
                        add_rect,
                        CornerRadius::same(8),
                        Stroke::new(
                            1.0,
                            mk::blend(
                                p.border,
                                mk::blend(p.bg_card, p.accent, 0.20),
                                if arr.hovered() && enabled { 1.0 } else { 0.0 },
                            ),
                        ),
                        StrokeKind::Inside,
                    );
                    let fg = if !enabled {
                        mk::blend(p.bg_card, p.text_dim, 0.5)
                    } else if arr.hovered() {
                        p.accent_strong
                    } else {
                        p.text_dim
                    };
                    mi::paint_centered(ui.painter(), add_rect.center(), "add", 16.0, fg);
                    if arr.clicked() && enabled {
                        want_add = true;
                    }

                    ui.add_space(12.0);

                    // `.nd-member-list { gap: 2px; max-height: 240px }`
                    let s = &mut ui.spacing_mut().scroll;
                    s.bar_width = 4.0;
                    s.floating = false;
                    egui::ScrollArea::vertical()
                        .id_salt("grp-dl-members")
                        .auto_shrink([false, true])
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.set_width(GDM_W);
                            ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                            if rows.is_empty() {
                                let (r, _) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), 63.0),
                                    Sense::hover(),
                                );
                                ui.painter().text(
                                    r.center(),
                                    Align2::CENTER_CENTER,
                                    state.t("No downloads in this group yet"),
                                    FontId::new(12.0, egui::FontFamily::default()),
                                    p.text_dim,
                                );
                            }
                            for (i, m) in rows.iter().enumerate() {
                                let avail = ui.available_width();
                                let (r, _) = ui.allocate_exact_size(vec2(avail, 47.0), Sense::hover());
                                // Rows align with the body content (`.group-member-row`
                                // sits inside `.nd-body`'s 16px).
                                let r = Rect::from_min_size(
                                    pos2(r.min.x + PAD, r.min.y),
                                    vec2(r.width() - 2.0 * PAD, 47.0),
                                );
                                let rr = ui.interact(
                                    r,
                                    egui::Id::new("grp-dl-row").with(i),
                                    Sense::hover(),
                                );
                                if rr.hovered() {
                                    ui.painter().rect_filled(
                                        r,
                                        CornerRadius::same(8),
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                                    );
                                }
                                let rm = Rect::from_center_size(
                                    pos2(r.max.x - 10.0 - 12.0, r.center().y),
                                    vec2(24.0, 24.0),
                                );
                                let rmr = ui.interact(
                                    rm,
                                    egui::Id::new("grp-dl-rm").with(i),
                                    Sense::click(),
                                );
                                let text_w = rm.min.x - 10.0 - (r.min.x + 10.0);
                                let clip = Rect::from_min_size(
                                    pos2(r.min.x + 10.0, r.min.y),
                                    vec2(text_w.max(10.0), r.height()),
                                );
                                ui.painter().text(
                                    pos2(clip.min.x, r.center().y - 8.5),
                                    Align2::LEFT_CENTER,
                                    &ellipsize(ui, &m.label, 12.0, text_w),
                                    FontId::new(12.0, tokens::typography::semibold_family()),
                                    p.text,
                                );
                                ui.painter().text(
                                    pos2(clip.min.x, r.center().y + 8.5),
                                    Align2::LEFT_CENTER,
                                    state.t(&m.status),
                                    FontId::new(11.0, egui::FontFamily::default()),
                                    if m.pending { p.text_dim } else { m.status_color },
                                );
                                if rmr.hovered() {
                                    ui.painter().rect_filled(
                                        rm,
                                        CornerRadius::same(4),
                                        mk::blend(p.bg_card, p.status_error, 0.10),
                                    );
                                }
                                mi::paint_centered(
                                    ui.painter(),
                                    rm.center(),
                                    "close",
                                    16.0,
                                    if rmr.hovered() { p.status_error } else { p.text_dim },
                                );
                                if rmr.clicked() {
                                    remove = Some(m.gid.clone());
                                }
                            }
                        });
                    ui.add_space(8.0);

                    // Footer: one full-width "Done" button.
                    let (f_rect, _) = ui.allocate_exact_size(vec2(GDM_W, FOOTER_H), Sense::hover());
                    ui.painter().rect_filled(
                        Rect::from_min_size(f_rect.min, vec2(GDM_W, 1.0)),
                        CornerRadius::same(0),
                        p.border,
                    );
                    if crate::modals::nd_button(
                        ui,
                        &p,
                        "grp-dl-done",
                        Rect::from_min_size(
                            pos2(f_rect.min.x + PAD, f_rect.min.y + 10.0),
                            vec2(GDM_W - 2.0 * PAD, 38.0),
                        ),
                        state.t("Done"),
                        Some("check"),
                        true,
                        true,
                    ) {
                        want_close = true;
                    }
                });
        });

    state.g_dl_url = url;

    if want_add {
        add_pending(state, edit);
    }
    if let Some(gid) = remove {
        remove_member(state, cctx, edit, &gid);
    }
    if want_close || ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.g_dl_open = false;
    } else if ctx.input(|i| i.key_pressed(Key::Enter)) {
        add_pending(state, edit);
    }
}

/// `addPendingGroupDownload` — stage the URL under a `pending-<seq>` gid.
fn add_pending(state: &mut UiState, edit: bool) {
    let url = state.g_dl_url.trim().to_owned();
    if url.is_empty() {
        return;
    }
    state.g_pending_seq += 1;
    let seq = state.g_pending_seq;
    state.g_pending.push(GroupPendingDl {
        seq,
        url,
        filename: String::new(),
    });
    let list = if edit {
        &mut state.g_edit_members
    } else {
        &mut state.g_create_members
    };
    list.push(format!("pending-{seq}"));
    state.g_dl_url.clear();
}

/// `removeGroupMember` — staging lists only drop the row; the edit dialog also
/// detaches the gid and clears the group-imposed speed cap right away.
fn remove_member(state: &mut UiState, cctx: &Ctx, edit: bool, gid: &str) {
    if let Some(seq) = gid.strip_prefix("pending-").and_then(|s| s.parse::<u64>().ok()) {
        state.g_pending.retain(|p| p.seq != seq);
    } else if edit {
        let Some(id) = state.g_edit.clone() else { return };
        state.group_remove_member(&id, gid);
        let gid = gid.to_owned();
        cctx.run(move |_| async move {
            let _ = moonhunt_core::aria2::apply_download_speed_limit(gid, 0).await;
            Ok::<String, String>(String::new())
        });
    }
    let list = if edit {
        &mut state.g_edit_members
    } else {
        &mut state.g_create_members
    };
    list.retain(|m| m != gid);
}

/// Width-capped text with an ellipsis (`text-overflow: ellipsis`).
fn ellipsize(ui: &Ui, text: &str, size: f32, width: f32) -> String {
    let font = FontId::new(size, tokens::typography::semibold_family());
    let measure = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .rect
            .width()
    };
    crate::rtl::truncate(text, |s| measure(s) <= width)
}
