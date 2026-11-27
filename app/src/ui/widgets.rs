//! MoonKit widgets: custom-painted controls with hover animations so the app
//! feels engineered instead of "debug gray stock egui".

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, InnerResponse,
    Layout, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
};
use crate::ui::tokens::{self, Palette, RADIUS_CARD, RADIUS_CONTROL, RADIUS_PILL, spacing};

/// Current UI mode tracked at startup (theme switching is re-applied by the
/// shell when settings change).
static MODE: std::sync::Mutex<Option<tokens::Mode>> = std::sync::Mutex::new(None);
/// Accent preset name (`settings.theme` — "coral", "green", … or ""/matched
/// against `ACCENT_PRESETS`).
static ACCENT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

pub fn set_mode(mode: tokens::Mode) {
    *MODE.lock().unwrap_or_else(|e| e.into_inner()) = Some(mode);
}

pub fn set_accent_preset(name: &str) {
    *ACCENT.lock().unwrap_or_else(|e| e.into_inner()) = name.to_string();
}

fn current_mode() -> tokens::Mode {
    MODE.lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .copied()
        .unwrap_or(tokens::Mode::Dark)
}

/// Composed palette: mode base colors + accent preset override (the old app's
/// `--accent`/`--accent-strong` runtime CSS-var logic).
pub fn pal() -> Palette {
    let mode = current_mode();
    let mut p = tokens::palette(mode);
    let accent_name = ACCENT.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let (a, s) = tokens::accent_pair(mode, &accent_name);
    p.accent = a;
    p.accent_strong = s;
    p
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Surfaces Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Rounded card with MoonKit border + card fill. The workhorse container.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let p = pal();
    Frame::new()
        .fill(p.bg_card)
        .corner_radius(CornerRadius::same(RADIUS_CARD))
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(12))
        .show(ui, add)
}

/// Flat filled panel used for toolbars/wells.
pub fn well<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let p = pal();
    Frame::new()
        .fill(p.bg_card_inset)
        .corner_radius(CornerRadius::same(RADIUS_CONTROL))
        .inner_margin(Margin::same(8))
        .show(ui, add)
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Buttons Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

#[derive(Clone, Copy, PartialEq)]
pub enum Variant {
    /// Accent-tinted action (Save, AddÃ¢â‚¬Â¦).
    Primary,
    /// Plain control (pause, retryÃ¢â‚¬Â¦).
    Normal,
    /// Subtle text-only control.
    Ghost,
    /// Destructive (Cancel, Remove).
    Danger,
}

/// One-line animated MoonKit button. Height = CONTROL_H.
pub fn btn(ui: &mut Ui, label: &str, variant: Variant) -> Response {
    let p = pal();
    let font = FontId::new(13.0, tokens::typography::semibold_family());
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE);
    let width = galley.rect.width() + 22.0;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(width.max(64.0), spacing::CONTROL_H),
        Sense::click(),
    );

    let id = ui
        .auto_id_with("moonkit-btn")
        .with(egui::Id::new(label));
    let t = ui
        .ctx()
        .animate_value_with_time(id, if resp.hovered() { 1.0 } else { 0.0 }, 0.08);

    let (fill, text_c, stroke_c) = match variant {
        Variant::Primary => (
            blend(p.bg_card, p.accent, 0.16 + 0.16 * t),
            p.accent,
            p.border,
        ),
        Variant::Normal => (blend(p.bg_card_inset, p.hover_veil, t), p.text, p.border),
        Variant::Ghost => (Color32::TRANSPARENT, p.text_dim, Color32::TRANSPARENT),
        Variant::Danger => (
            blend(p.bg_card, p.status_error, 0.14 + 0.14 * t),
            p.status_error,
            p.status_error.gamma_multiply(0.4),
        ),
    };

    ui.painter()
        .rect_filled(rect, CornerRadius::same(RADIUS_CONTROL), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(RADIUS_CONTROL),
        Stroke::new(1.0, stroke_c),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        font,
        text_c,
    );
    resp
}

/// Make `blend` visible to the title bar window controls.
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Pills & labels Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Rounded status pill (download states, tags).
pub fn pill(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        FontId::new(11.0, tokens::typography::semibold_family()),
        color,
    );
    let h = 20.0;
    let w = galley.rect.width() + 14.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(RADIUS_PILL),
        color.gamma_multiply(0.16),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(11.0, tokens::typography::semibold_family()),
        color,
    );
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Progress Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Thin progress bar with MoonKit track + status color fill.
pub fn progress(ui: &mut Ui, frac: f32, color: Color32, height: f32, width: f32) {
    let p = pal();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(3), p.bg_card_inset);
    let f = frac.clamp(0.0, 1.0);
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, vec2(rect.width() * f, height)),
        CornerRadius::same(3),
        color,
    );
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Switch Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Old `.toggle`/`.speed-toggle` metrics: 42Ãƒâ€”24, 12r track, 18px knob that
/// slides 18px right; off = hover-bg + border-strong + dim knob, on = accent
/// 25% track + accent knob.
pub fn switch(ui: &mut Ui, on: bool, id: &str) -> Response {
    let p = pal();
    let size = vec2(42.0, 24.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let t = ui
        .ctx()
        .animate_value_with_time(egui::Id::new(id).with("t"), if on { 1.0 } else { 0.0 }, 0.1);
    // `.pf-toggle` off track is `--hover-bg`, i.e. the card surface lifted by
    // a light veil. Composited opaque: `blend` round-trips through
    // premultiplied `Color32`, so a translucent veil loses its brightness and
    // the off track ends up the same colour as the page.
    let off = blend(p.bg_card, p.text, 0.08);
    let on = blend(p.bg_card, p.accent, 0.25);
    let bg = blend(off, on, t);
    ui.painter().rect_filled(rect, CornerRadius::same(12), bg);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, blend(p.border_strong, p.accent.gamma_multiply(0.4), t)),
        StrokeKind::Inside,
    );
    let x = rect.min.x + 11.0 + 18.0 * t;
    ui.painter().circle_filled(
        pos2(x, rect.center().y),
        9.0,
        blend(p.text_dim, p.accent, t),
    );
    resp
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Material icon button Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

/// Icon-only hover button (old `.titlebar-btn` / `.titlebar-action`:
/// 32Ã¢â‚¬â€œ36px square, 6r, dim icon, hover-bg veil; `hover_fill`/`hover_color`
/// override e.g. the close button and the primary action button).
pub fn icon_btn(
    ui: &mut Ui,
    name: &str,
    size: f32,
    icon_size: f32,
    color: Color32,
    hover_fill: Option<Color32>,
    hover_color: Option<Color32>,
) -> Response {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let id = egui::Id::new("icon-btn").with(name).with(ui.id());
    let t = ui.ctx().animate_value_with_time(
        id,
        if resp.hovered() { 1.0 } else { 0.0 },
        0.1,
    );
    if t > 0.01 {
        let fill = hover_fill.unwrap_or(p.hover_veil);
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill.gamma_multiply(t));
    }
    let col = blend(color, hover_color.unwrap_or(p.text), t);
    // The reference (Chromium) paints material glyphs ~1px lower than egui's
    // CENTER_CENTER galley placement for the same nominal center.
    crate::ui::icons::paint_centered(
        ui.painter(),
        rect.center() + vec2(0.0, 1.0),
        name,
        icon_size,
        col,
    );
    resp
}

/// Old `.seg-switch`: two buttons in a hover 10r pill (240px min), with a
/// bg-card indicator (7r) sliding between halves. Returns the selected arm.
pub fn seg_switch(ui: &mut Ui, id: &str, left: &str, right: &str, on_right: bool) -> bool {
    let p = pal();
    let width = 240.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 38.0), Sense::click());
    ui.painter().rect_filled(rect, CornerRadius::same(10), p.hover_veil.gamma_multiply(0.5));
    let t = ui
        .ctx()
        .animate_value_with_time(egui::Id::new(id).with("t"), if on_right { 1.0 } else { 0.0 }, 0.12);
    let half = (width - 8.0) / 2.0;
    let ind = Rect::from_min_size(
        pos2(rect.min.x + 4.0 + t * (half + 4.0), rect.min.y + 4.0),
        vec2(half, 30.0),
    );
    ui.painter()
        .rect_filled(ind, CornerRadius::same(7), p.bg_card);
    ui.painter().rect_stroke(
        ind,
        CornerRadius::same(7),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let color_l = blend(p.text_dim, p.accent_strong, 1.0 - t);
    let color_r = blend(p.text_dim, p.accent_strong, t);
    ui.painter().text(
        pos2(rect.min.x + 4.0 + half * 0.5, rect.center().y),
        Align2::CENTER_CENTER,
        left,
        FontId::new(12.0, tokens::typography::semibold_family()),
        color_l,
    );
    ui.painter().text(
        pos2(rect.max.x - 4.0 - half * 0.5, rect.center().y),
        Align2::CENTER_CENTER,
        right,
        FontId::new(12.0, tokens::typography::semibold_family()),
        color_r,
    );
    if resp.clicked() {
        let px = ui.input(|i| i.pointer.latest_pos().unwrap_or_default());
        let clicked_right = px.x > rect.center().x;
        return clicked_right;
    }
    on_right
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Small layout helpers Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

pub fn gap(ui: &mut Ui, px: f32) {
    ui.add_space(px);
}

/// Aligned right-end block inside a horizontal layout.
pub fn right<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.with_layout(Layout::right_to_left(Align::Center), add)
}

pub fn weak_small(text: &str) -> RichText {
    RichText::new(text).weak().size(12.0)
}

