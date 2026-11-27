//! Helper-card lists of the Media screen: `.media-binaries-card` stacks for
//! yt-dlp/EJS, JS runtimes and the media merger.
//!
//! Pixel port of `Media.vue` (v0.0.6): template 114–459, behaviour block
//! 686–1200 and the `/* Binaries */` CSS (1617–1971). Every metric below
//! quotes the scoped CSS value it reproduces.

use super::Ctx;
use crate::state::UiState;
use crate::ui::icons as mi;
use crate::ui::tokens::Palette;
use crate::ui::widgets as mk;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
};
use std::time::Duration;

// ── State (Media.vue refs) ──────────────────────────────────────────────

#[derive(Clone, Default)]
struct RuntimeRow {
    id: String,
    name: String,
    installed: bool,
    selected: bool,
    path: String,
    size: String,
}

#[derive(Clone, Default)]
struct SystemNode {
    version: String,
    path: String,
    selected: bool,
}

#[derive(Clone, Default)]
struct MergerRow {
    id: String,
    name: String,
    installed: bool,
    selected: bool,
    path: String,
    size: String,
}

#[derive(Clone, Default)]
struct SystemFfmpeg {
    path: String,
    selected: bool,
}

/// Tools-card state (`ytDlpExists`, `ejsExists`, install progress, runtimes,
/// mergers and the per-row busy flags).
#[derive(Default)]
pub struct ToolsUi {
    /// Vue's `onActivated` pass runs on entry; the port runs it once.
    loaded: bool,
    /// A refresh tick from an install/remove/select task is pending.
    dirty_yt: bool,
    dirty_ejs: bool,
    dirty_runtimes: bool,
    dirty_mergers: bool,

    // Helper binaries.
    yt_exists: bool,
    yt_path: String,
    ejs_exists: bool,
    ejs_path: String,
    installing_yt: bool,
    yt_percent: u32,
    installing_ejs: bool,

    // JS runtimes.
    runtimes: Vec<RuntimeRow>,
    system_node: Option<SystemNode>,
    installing_runtime: String,
    runtime_percent: u32,
    selecting_runtime: bool,
    removing_runtime: String,

    // Media mergers.
    mergers: Vec<MergerRow>,
    system_ffmpeg: Option<SystemFfmpeg>,
    installing_merger: String,
    merger_percent: u32,
    selecting_merger: bool,
    removing_merger: String,
}

#[derive(Clone)]
enum Act {
    OpenBinFolder,
    OpenYtLocation,
    OpenEjsLocation,
    InstallYt,
    CancelYt,
    UninstallYt,
    InstallEjs,
    CancelEjs,
    UninstallEjs,
    SelectRuntime(String),
    RemoveRuntime(String),
    InstallRuntime(String),
    CancelRuntime(String),
    SelectMerger(String),
    RemoveMerger(String),
    InstallMerger(String),
    CancelMerger(String),
}

// ── Font helpers ────────────────────────────────────────────────────────

fn bold(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::bold_family())
}

fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

fn draw_text(
    ui: &Ui,
    at: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, color);
    let size = galley.size();
    let ax = match anchor.x() {
        egui::Align::Min => 0.0,
        egui::Align::Center => -size.x / 2.0,
        egui::Align::Max => -size.x,
    };
    let ay = match anchor.y() {
        egui::Align::Min => 0.0,
        egui::Align::Center => -size.y / 2.0,
        egui::Align::Max => -size.y,
    };
    let origin = at + vec2(ax, ay);
    ui.painter().galley(origin, galley, color);
    Rect::from_min_size(origin, size)
}

fn text_width(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font, Color32::WHITE)
        .size()
        .x
}

/// CSS `letter-spacing` (egui has none): lay the characters out one by one.
fn draw_text_spaced(
    ui: &Ui,
    at: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
) -> f32 {
    let painter = ui.painter();
    // Cursive script: a per-character layout would break the joins and run the
    // letters left to right, so paint the whole string as one shaped run.
    if crate::rtl::has_rtl(text) {
        let g = painter.layout_no_wrap(text.to_owned(), font, color);
        painter.galley(at, g.clone(), color);
        return g.size().x;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0.0;
    }
    let galleys: Vec<std::sync::Arc<egui::Galley>> = chars
        .iter()
        .map(|c| painter.layout_no_wrap(c.to_string(), font.clone(), color))
        .collect();
    let advances: Vec<f32> = galleys.iter().map(|g| g.size().x).collect();
    let total: f32 = advances.iter().sum::<f32>() + spacing * (chars.len() as f32 - 1.0);
    let mut x = at.x;
    for (g, adv) in galleys.iter().zip(advances.iter()) {
        painter.galley(pos2(x, at.y), g.clone(), color);
        x += adv + spacing;
    }
    total
}

fn ellipsize(ui: &Ui, text: &str, font: &FontId, max_w: f32) -> String {
    if max_w <= 0.0 {
        return text.to_string();
    }
    crate::rtl::truncate(text, |s| text_width(ui, s, font.clone()) <= max_w)
}

// ── Painting helpers ────────────────────────────────────────────────────

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn white_alpha(a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, a)
}

/// CSS `opacity: .45` on a disabled control: fade the whole paint.
fn dim(c: Color32, enabled: bool) -> Color32 {
    if enabled {
        c
    } else {
        with_alpha(c, (c.a() as f32 * 0.45) as u8)
    }
}

/// Blurred box-shadow approximation: expanding translucent rounded rings.
fn soft_shadow(ui: &Ui, rect: Rect, radius: u8, color: Color32, offset_y: f32, blur: f32, alpha: u8) {
    super::soft_shadow(ui, rect, radius, color, offset_y, blur, alpha)
}

/// `.tools-card-header::before` — radial accent glow (circle, accent .16 at
/// the centre, transparent at 70 % of the farthest-corner radius ≈ 83.5 px).
/// Drawn as a vertex-coloured fan so the ramp stays smooth (72 stacked alpha
/// circles quantise too coarsely at this opacity).
fn radial_glow(ui: &Ui, clip: Rect, center: Pos2, radius: f32, color: Color32) {
    let painter = ui.painter().with_clip_rect(clip);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, with_alpha(color, 41));
    let n = 96usize;
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            center + vec2(a.cos(), a.sin()) * radius,
            Color32::TRANSPARENT,
        );
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(0, 1 + i as u32, 1 + j as u32);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn layout_row(ui: &mut Ui, w: f32, h: f32) -> Rect {
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    r
}

fn hline(ui: &Ui, rect: Rect, color: Color32) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(0), color);
}

/// `.tool-section-label::after` — `linear-gradient(90deg, border-strong,
/// transparent)`.
fn fade_line(ui: &Ui, rect: Rect, color: Color32) {
    let n = 32;
    let w = rect.width() / n as f32;
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32;
        let c = with_alpha(color, (color.a() as f32 * (1.0 - t)) as u8);
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(rect.min.x + i as f32 * w, rect.min.y), vec2(w + 0.5, rect.height())),
            CornerRadius::same(0),
            c,
        );
    }
}

/// `.material-icons.spinning` — 0.8s linear icon rotation.
fn spinning_icon(ui: &Ui, center: Pos2, name: &str, size: f32, color: Color32) {
    let t = ui.ctx().input(|i| i.time) as f32;
    let angle = (t / 0.8).fract() * std::f32::consts::TAU;
    let galley = ui.painter().layout_no_wrap(
        mi::glyph(name).to_string(),
        mi::font(size),
        color,
    );
    // `with_angle_and_anchor` keeps the anchor at `pos + anchor_offset`, so
    // start from the top-left that centres the glyph on `center`.
    let top_left = center - galley.rect.center().to_vec2();
    let shape = egui::epaint::TextShape::new(top_left, galley, color)
        .with_angle_and_anchor(angle, Align2::CENTER_CENTER);
    ui.painter().add(shape);
    ui.ctx().request_repaint_after(Duration::from_millis(16));
}

// ── Common pieces ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum BtnKind {
    Primary,
    Secondary,
    Danger,
}

#[derive(Clone)]
struct BtnSpec {
    label: String,
    icon: &'static str,
    kind: BtnKind,
    enabled: bool,
    act: Act,
    spin: bool,
}

#[derive(Clone)]
enum Actions {
    /// A flex row of `.tool-btn`s, optionally preceded by `.tool-active-label`.
    Row { active: bool, btns: Vec<BtnSpec> },
    /// `.tool-install-state` + square cancel button.
    Install { text: String, cancel: Act },
}

fn btn_width(ui: &Ui, b: &BtnSpec) -> f32 {
    // `.tool-btn`: border 1px + padding 0 11px, gap 6px, icon 15px.
    let tw = text_width(ui, &b.label, bold(11.0));
    let iw = if b.icon.is_empty() { 0.0 } else { 15.0 + 6.0 };
    2.0 + 11.0 + iw + tw + 11.0
}

fn active_label_width(ui: &Ui, lang: &str) -> f32 {
    // padding 5px 9px, border 1px, icon 13px, gap 5px.
    let tw = text_width(ui, crate::i18n::t(lang, "In use"), bold(11.0));
    2.0 + 18.0 + 13.0 + 5.0 + tw
}

fn install_state_width(ui: &Ui, text: &str) -> f32 {
    // padding 0 2px, icon 15px, gap 6px.
    4.0 + 15.0 + 6.0 + text_width(ui, text, bold(11.5))
}

fn actions_width(ui: &Ui, actions: &Actions, lang: &str) -> f32 {
    match actions {
        Actions::Row { active, btns } => {
            let mut w = 0.0;
            if *active {
                w += active_label_width(ui, lang);
                if !btns.is_empty() {
                    w += 8.0;
                }
            }
            if !btns.is_empty() {
                w += btns.iter().map(|b| btn_width(ui, b)).sum::<f32>();
                w += 8.0 * (btns.len() as f32 - 1.0);
            }
            w
        }
        Actions::Install { text, .. } => install_state_width(ui, text) + 8.0 + 32.0,
    }
}

fn paint_btn(ui: &mut Ui, rect: Rect, b: &BtnSpec, p: &Palette, id: egui::Id) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = b.enabled && resp.hovered();
    let painter = ui.painter();
    match b.kind {
        BtnKind::Primary => {
            // box-shadow: 0 4px 12px accent(.18), hover 0 5px 16px (.24);
            // the button's `opacity: .45` also fades its shadow.
            let (dy, blur, alpha) = if hovered {
                (5.0, 16.0, 61.0)
            } else {
                (4.0, 12.0, 46.0)
            };
            let alpha = if b.enabled { alpha } else { alpha * 0.45 };
            soft_shadow(ui, rect, 9, p.accent, dy, blur, alpha as u8);
            let fill = if hovered {
                mk::blend(p.accent, Color32::WHITE, 0.08)
            } else {
                p.accent
            };
            painter.rect_filled(rect, CornerRadius::same(9), dim(fill, b.enabled));
        }
        BtnKind::Secondary => {
            let fill = if hovered {
                white_alpha(20)
            } else {
                p.hover_veil
            };
            painter.rect_filled(rect, CornerRadius::same(9), dim(fill, b.enabled));
            painter.rect_stroke(
                rect,
                CornerRadius::same(9),
                Stroke::new(1.0, dim(p.border_strong, b.enabled)),
                StrokeKind::Inside,
            );
        }
        BtnKind::Danger => {
            let a = if hovered { 41 } else { 26 };
            painter.rect_filled(
                rect,
                CornerRadius::same(9),
                dim(with_alpha(p.status_error, a), b.enabled),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(9),
                Stroke::new(1.0, dim(with_alpha(p.status_error, 41), b.enabled)),
                StrokeKind::Inside,
            );
        }
    }
    let color = match b.kind {
        BtnKind::Primary => Color32::from_rgb(0x07, 0x12, 0x1d),
        BtnKind::Secondary => {
            if hovered {
                p.text
            } else {
                p.text_muted
            }
        }
        BtnKind::Danger => p.status_error,
    };
    let color = dim(color, b.enabled);
    let font = bold(11.0);
    let tw = text_width(ui, &b.label, font.clone());
    let iw = if b.icon.is_empty() { 0.0 } else { 15.0 + 6.0 };
    let content = iw + tw;
    let mut x = rect.center().x - content / 2.0;
    if !b.icon.is_empty() {
        if b.spin {
            spinning_icon(ui, pos2(x + 7.5, rect.center().y), b.icon, 15.0, color);
        } else {
            mi::paint_centered(ui.painter(), pos2(x + 7.5, rect.center().y), b.icon, 15.0, color);
        }
        x += 15.0 + 6.0;
    }
    draw_text(ui, pos2(x, rect.center().y), Align2::LEFT_CENTER, &b.label, font, color);
    b.enabled && resp.clicked()
}

fn paint_active_label(ui: &Ui, left: f32, top: f32, p: &Palette, lang: &str) {
    let font = bold(11.0);
    let label = crate::i18n::t(lang, "In use");
    let tw = text_width(ui, label, font.clone());
    let rect = Rect::from_min_size(
        pos2(left, top),
        vec2(2.0 + 18.0 + 13.0 + 5.0 + tw, 28.0),
    );
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(14),
        with_alpha(p.status_complete, 20),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, with_alpha(p.status_complete, 46)),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 1.0 + 9.0 + 6.5, rect.center().y),
        "check",
        13.0,
        p.status_complete,
    );
    draw_text(
        ui,
        pos2(rect.min.x + 1.0 + 9.0 + 13.0 + 5.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        p.status_complete,
    );
}

/// `.tool-status` pill. Returns its width.
fn paint_status(
    ui: &Ui,
    x: f32,
    top: f32,
    icon: &str,
    text: &str,
    ok: bool,
    p: &Palette,
) -> f32 {
    let font = bold(10.5);
    let tw = text_width(ui, text, font.clone());
    let w = 34.0 + tw;
    let rect = Rect::from_min_size(pos2(x, top), vec2(w, 23.0));
    let (fill, border, color) = if ok {
        (
            with_alpha(p.status_complete, 20),
            with_alpha(p.status_complete, 36),
            p.status_complete,
        )
    } else {
        (p.hover_veil, p.border, p.text_muted)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(12), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 1.0 + 8.0 + 6.0, rect.center().y),
        icon,
        12.0,
        color,
    );
    draw_text(
        ui,
        pos2(rect.min.x + 1.0 + 8.0 + 12.0 + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    w
}

fn paint_icon_box(
    ui: &Ui,
    rect: Rect,
    icon: &str,
    color: Color32,
    tint: Color32,
    bg_a: u8,
    bd_a: u8,
) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(12), with_alpha(tint, bg_a));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, with_alpha(tint, bd_a)),
        StrokeKind::Inside,
    );
    mi::paint_centered(ui.painter(), rect.center(), icon, 20.0, color);
}

fn paint_progress(ui: &Ui, rect: Rect, percent: u32, p: &Palette) {
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(3),
        with_alpha(Color32::WHITE, 15),
    );
    let fw = rect.width() * (percent.min(100) as f32 / 100.0);
    if fw <= 0.5 {
        return;
    }
    let glow = Rect::from_min_size(
        pos2(rect.min.x, rect.min.y - 2.0),
        vec2(fw, rect.height() + 4.0),
    );
    ui.painter()
        .rect_filled(glow, CornerRadius::same(5), with_alpha(p.accent, 12));
    let n = (fw.ceil() as usize).max(1);
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32;
        let c = mk::blend(p.accent, with_alpha(p.accent, 153), t);
        // `border-radius: 100px` on a 5px bar = 2px caps.
        let cap = CornerRadius {
            nw: if i == 0 { 2 } else { 0 },
            sw: if i == 0 { 2 } else { 0 },
            ne: if i + 1 == n { 2 } else { 0 },
            se: if i + 1 == n { 2 } else { 0 },
        };
        ui.painter().rect_filled(
            Rect::from_min_size(
                pos2(rect.min.x + i as f32, rect.min.y),
                vec2(1.5, rect.height()),
            ),
            cap,
            c,
        );
    }
}

struct LocationSpec<'a> {
    path: &'a str,
    open: Option<Act>,
}

struct ItemSpec<'a> {
    key: &'a str,
    icon: &'a str,
    icon_color: Color32,
    icon_tint: Color32,
    icon_bg: u8,
    icon_border: u8,
    name: &'a str,
    status_icon: &'a str,
    status: &'a str,
    status_ok: bool,
    desc: &'a str,
    location: Option<LocationSpec<'a>>,
    actions: Actions,
    active: bool,
    installing: bool,
    progress: u32,
}

fn draw_item(ui: &mut Ui, spec: &ItemSpec, p: &Palette, w: f32, lang: &str) -> Option<Act> {
    let loc_h = match &spec.location {
        Some(l) if l.open.is_some() => 38.0,
        Some(_) => 28.0,
        None => 0.0,
    };
    // 12px padding + 1px border on both sides; the first flex row is 42px
    // tall unless the location block makes the main column taller.
    let main_h = if loc_h > 0.0 { 50.0 + loc_h } else { 42.0 };
    let h = 26.0 + main_h + if spec.installing { 17.0 } else { 0.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let hovered = ui.rect_contains_pointer(rect);

    let bg = if spec.active {
        // `.runtime-item.tool-item-active`: green .09→.02 flattened.
        with_alpha(p.status_complete, 14)
    } else if hovered {
        // `.tool-item:hover`: white .04→.012 flattened.
        with_alpha(Color32::WHITE, 7)
    } else {
        // `.tool-item`: white .025→.008 flattened.
        with_alpha(Color32::WHITE, 4)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(14), bg);
    let border = if spec.active {
        with_alpha(p.status_complete, 61)
    } else if hovered {
        p.border_strong
    } else {
        p.border
    };
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    let content = Rect::from_min_max(rect.min + vec2(13.0, 13.0), rect.max - vec2(13.0, 13.0));
    let icon_rect = Rect::from_min_size(content.min, vec2(42.0, 42.0));
    paint_icon_box(
        ui,
        icon_rect,
        spec.icon,
        spec.icon_color,
        spec.icon_tint,
        spec.icon_bg,
        spec.icon_border,
    );

    let aw = actions_width(ui, &spec.actions, lang);
    let main_w = (content.width() - 42.0 - 12.0 - aw - 12.0).max(0.0);
    let main = Rect::from_min_size(pos2(icon_rect.max.x + 12.0, content.min.y), vec2(main_w, main_h));

    // `.tool-item-title` (name + status pill, 22px row).
    let name_font = bold(13.5);
    let name_w = text_width(ui, spec.name, name_font.clone());
    draw_text(
        ui,
        pos2(main.min.x, content.min.y + 11.5),
        Align2::LEFT_CENTER,
        spec.name,
        name_font,
        p.text,
    );
    paint_status(
        ui,
        main.min.x + name_w + 8.0,
        content.min.y,
        spec.status_icon,
        spec.status,
        spec.status_ok,
        p,
    );

    // `.tool-item-desc`: margin-top 4px, 11px/1.4.
    draw_text(
        ui,
        pos2(main.min.x, content.min.y + 27.0),
        Align2::LEFT_TOP,
        spec.desc,
        regular(11.0),
        p.text_dim,
    );

    let mut clicked = None;
    if let Some(loc) = &spec.location {
        let lr = Rect::from_min_size(pos2(main.min.x, content.min.y + 50.0), vec2(main_w, loc_h));
        ui.painter().rect_filled(lr, CornerRadius::same(8), p.hover_veil);
        ui.painter().rect_stroke(
            lr,
            CornerRadius::same(8),
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(lr.min.x + 8.0 + 7.0, lr.center().y),
            "folder",
            14.0,
            p.text_dim,
        );
        let path_font = mono(10.0);
        let text_x = lr.min.x + 8.0 + 14.0 + 7.0;
        let mut text_right = lr.max.x - 8.0;
        if loc.open.is_some() {
            text_right -= 24.0 + 7.0;
            let or = Rect::from_min_size(
                pos2(lr.max.x - 8.0 - 24.0, lr.center().y - 12.0),
                vec2(24.0, 24.0),
            );
            let resp = ui.interact(or, ui.scope_id().with(("tools-loc-open", spec.key)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(
                    or,
                    CornerRadius::same(6),
                    with_alpha(p.accent, 31),
                );
            }
            mi::paint_centered(
                ui.painter(),
                or.center(),
                "open_in_new",
                13.0,
                if resp.hovered() { p.accent_strong } else { p.text_dim },
            );
            if resp.clicked() {
                clicked = loc.open.clone();
            }
        }
        let shown = ellipsize(ui, loc.path, &path_font, (text_right - text_x).max(0.0));
        draw_text(
            ui,
            pos2(text_x, lr.center().y),
            Align2::LEFT_CENTER,
            &shown,
            path_font,
            p.text_dim,
        );
    }

    // `.tool-actions` sits at the top of the row (flex-start) and is 32px
    // tall, so smaller leading labels are centred inside it.
    let actions_left = content.max.x - aw;
    match &spec.actions {
        Actions::Row { active, btns } => {
            let mut x = actions_left;
            if *active {
                paint_active_label(ui, x, content.min.y + 2.0, p, lang);
                x += active_label_width(ui, lang) + if btns.is_empty() { 0.0 } else { 8.0 };
            }
            for b in btns {
                let bw = btn_width(ui, b);
                let br = Rect::from_min_size(pos2(x, content.min.y), vec2(bw, 32.0));
                let id = ui
                    .scope_id()
                    .with(("tools-btn", spec.key, b.label.as_str()));
                if paint_btn(ui, br, b, p, id) {
                    clicked = Some(b.act.clone());
                }
                x += bw + 8.0;
            }
        }
        Actions::Install { text, cancel } => {
            spinning_icon(
                ui,
                pos2(actions_left + 2.0 + 7.5, content.min.y + 16.0),
                "downloading",
                15.0,
                p.accent_strong,
            );
            draw_text(
                ui,
                pos2(actions_left + 2.0 + 15.0 + 6.0, content.min.y + 16.0),
                Align2::LEFT_CENTER,
                text,
                bold(11.5),
                p.accent_strong,
            );
            let cr = Rect::from_min_size(
                pos2(content.max.x - 32.0, content.min.y),
                vec2(32.0, 32.0),
            );
            let b = BtnSpec {
                label: String::new(),
                icon: "close",
                kind: BtnKind::Danger,
                enabled: true,
                act: cancel.clone(),
                spin: false,
            };
            let id = ui.scope_id().with(("tools-cancel", spec.key));
            if paint_btn(ui, cr, &b, p, id) {
                clicked = Some(cancel.clone());
            }
        }
    }

    if spec.installing {
        let py = content.min.y + main_h + 12.0;
        let pr = Rect::from_min_size(pos2(content.min.x, py), vec2(content.width(), 5.0));
        paint_progress(ui, pr, spec.progress, p);
    }

    clicked
}

fn paint_summary(
    ui: &mut Ui,
    key: &str,
    right: f32,
    top: f32,
    kind: Summary,
    icon: &str,
    text: &str,
    p: &Palette,
) -> bool {
    let font = bold(11.0);
    let tw = text_width(ui, text, font.clone());
    // border 2 + padding 20 + open button (-4 margin) 16 + gaps 12 + icon 14.
    let w = 64.0 + tw;
    let rect = Rect::from_min_size(pos2(right - w, top), vec2(w, 32.0));
    let (fill, border, color) = match kind {
        Summary::Ok => (
            with_alpha(p.status_complete, 20),
            with_alpha(p.status_complete, 46),
            p.status_complete,
        ),
        Summary::Warn => (
            with_alpha(color_warn(), 20),
            with_alpha(color_warn(), 41),
            color_warn(),
        ),
        Summary::Idle => (p.hover_veil, p.border, p.text_dim),
    };
    ui.painter().rect_filled(rect, CornerRadius::same(16), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(16),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );

    let ob = Rect::from_min_size(pos2(rect.min.x + 7.0, rect.min.y + 6.0), vec2(20.0, 20.0));
    let resp = ui.interact(ob, ui.scope_id().with(("tools-summary-open", key)), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(ob, CornerRadius::same(6), with_alpha(p.accent, 31));
    }
    let open_color = if resp.hovered() {
        p.accent_strong
    } else {
        with_alpha(color, (color.a() as f32 * 0.7) as u8)
    };
    mi::paint_centered(ui.painter(), ob.center(), "folder_open", 14.0, open_color);

    let ix = ob.max.x + 6.0 + 7.0;
    mi::paint_centered(ui.painter(), pos2(ix, rect.center().y), icon, 14.0, color);
    draw_text(
        ui,
        pos2(ob.max.x + 6.0 + 14.0 + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    resp.clicked()
}

#[derive(Clone, Copy)]
enum Summary {
    Ok,
    Warn,
    Idle,
}

fn color_warn() -> Color32 {
    Color32::from_rgb(0xff, 0xd1, 0x80)
}

fn card_header(
    ui: &mut Ui,
    w: f32,
    key: &str,
    icon: &str,
    title: &str,
    subtitle: &str,
    kind: Summary,
    summary_icon: &str,
    summary_text: &str,
    p: &Palette,
) -> bool {
    let rect = layout_row(ui, w, 67.0);
    let content = Rect::from_min_size(rect.min, vec2(w, 66.0));

    // ::before radial glow, clipped to the header.
    radial_glow(
        ui,
        content,
        pos2(rect.min.x + w - 70.0, rect.min.y - 5.0),
        83.5,
        p.accent,
    );
    // border-bottom: 1px solid var(--border)
    hline(
        ui,
        Rect::from_min_size(pos2(rect.min.x, rect.min.y + 66.0), vec2(w, 1.0)),
        p.border,
    );

    let icon_rect = Rect::from_min_size(pos2(rect.min.x + 16.0, rect.min.y + 16.0), vec2(38.0, 38.0));
    super::grad135(
        ui,
        icon_rect,
        12,
        mk::blend(p.bg_card, p.accent, 0.18),
        mk::blend(p.bg_card, p.accent, 0.05),
    );
    ui.painter().rect_stroke(
        icon_rect,
        CornerRadius::same(12),
        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.16)),
        StrokeKind::Inside,
    );
    mi::paint_centered(ui.painter(), icon_rect.center(), icon, 20.0, p.accent_strong);

    // `.nd-title` 14px/1.2 + 2px + `.nd-subtitle` 11px, the pair centred on
    // the 38px icon row (same technique as the downloader card header).
    let title_font = bold(14.0);
    let sub_font = regular(11.0);
    let title_h = ui.fonts_mut(|f| f.row_height(&title_font)).max(16.8);
    let sub_h = ui.fonts_mut(|f| f.row_height(&sub_font)).max(15.0);
    let block_h = title_h + 2.0 + sub_h;
    let top = rect.min.y + 16.0 + (38.0 - block_h) * 0.5;
    let tx = icon_rect.max.x + 12.0;
    draw_text(
        ui,
        pos2(tx, top + title_h / 2.0),
        Align2::LEFT_CENTER,
        title,
        title_font,
        p.text,
    );
    draw_text(
        ui,
        pos2(tx, top + title_h + 2.0 + sub_h / 2.0),
        Align2::LEFT_CENTER,
        subtitle,
        sub_font,
        p.text_muted,
    );

    paint_summary(
        ui,
        key,
        rect.min.x + w - 16.0,
        rect.min.y + 19.0,
        kind,
        summary_icon,
        summary_text,
        p,
    )
}

fn section_label(ui: &mut Ui, w: f32, text: &str, p: &Palette) {
    // padding: 6px 2px 0; 10px/1.4 label with a 10px gap to the fade line.
    let rect = layout_row(ui, w, 20.0);
    let tw = draw_text_spaced(
        ui,
        pos2(rect.min.x + 2.0, rect.min.y + 6.0),
        text,
        bold(10.0),
        p.text_dim,
        0.8,
    );
    let line_x = rect.min.x + 2.0 + tw + 10.0;
    if line_x < rect.max.x - 2.0 {
        fade_line(
            ui,
            Rect::from_min_size(pos2(line_x, rect.min.y + 12.5), vec2(rect.max.x - 2.0 - line_x, 1.0)),
            p.border_strong,
        );
    }
}

fn runtime_icon(id: &str) -> &'static str {
    match id {
        "quickjs" => "bolt",
        "node" => "play_circle",
        "bun" => "rocket_launch",
        "deno" => "lens_blur",
        _ => "terminal",
    }
}

// ── Cards ───────────────────────────────────────────────────────────────

/// The three `.tools-card` sections, in template order.
pub fn cards(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    {
        let t = &mut state.media.tools;
        if !t.loaded {
            t.loaded = true;
            t.dirty_yt = true;
            t.dirty_ejs = true;
            t.dirty_runtimes = true;
            t.dirty_mergers = true;
        }
        // One concurrent pass, like Vue's `Promise.allSettled` on activation.
        if t.dirty_yt {
            t.dirty_yt = false;
            load_yt_state(ctx);
        }
        if t.dirty_ejs {
            t.dirty_ejs = false;
            load_ejs_state(ctx);
        }
        if t.dirty_runtimes {
            t.dirty_runtimes = false;
            load_runtimes(ctx);
        }
        if t.dirty_mergers {
            t.dirty_mergers = false;
            load_mergers(ctx);
        }
    }

    let act = helper_card(&state.media.tools, ui, w, &state.settings.language);
    if let Some(a) = act {
        perform(state, ctx, a);
    }
    let act = runtimes_card(&state.media.tools, ui, w, &state.settings.language);
    if let Some(a) = act {
        perform(state, ctx, a);
    }
    let act = mergers_card(&state.media.tools, ui, w, &state.settings.language);
    if let Some(a) = act {
        perform(state, ctx, a);
    }
}

fn helper_card(t: &ToolsUi, ui: &mut Ui, w: f32, lang: &str) -> Option<Act> {
    let p = mk::pal();
    let mut act = None;
    super::card_frame()
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.set_width(w - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let ready = t.yt_exists && t.ejs_exists;
            if card_header(
                ui,
                w - 2.0,
                "helper",
                "build_circle",
                crate::i18n::t(lang, "Helper Binaries"),
                crate::i18n::t(lang, "Required for media downloads to work."),
                if ready { Summary::Ok } else { Summary::Warn },
                if ready { "check_circle" } else { "error_outline" },
                if ready { crate::i18n::t(lang, "Ready") } else { crate::i18n::t(lang, "Needs setup") },
                &p,
            ) {
                act = Some(Act::OpenBinFolder);
            }

            ui.add_space(12.0);
            let iw = w - 2.0 - 24.0;

            let yt_actions = if t.installing_yt {
                Actions::Install {
                    text: if t.yt_percent > 0 {
                        format!("{}%", t.yt_percent)
                    } else {
                        crate::i18n::t(lang, "Installing...").to_string()
                    },
                    cancel: Act::CancelYt,
                }
            } else if !t.yt_exists {
                Actions::Row {
                    active: false,
                    btns: vec![BtnSpec {
                        label: crate::i18n::t(lang, "Install").into(),
                        icon: "download",
                        kind: BtnKind::Primary,
                        enabled: true,
                        act: Act::InstallYt,
                        spin: false,
                    }],
                }
            } else {
                Actions::Row {
                    active: false,
                    btns: vec![
                        BtnSpec {
                            label: crate::i18n::t(lang, "Open location").into(),
                            icon: "folder_open",
                            kind: BtnKind::Secondary,
                            enabled: true,
                            act: Act::OpenYtLocation,
                            spin: false,
                        },
                        BtnSpec {
                            label: crate::i18n::t(lang, "Remove").into(),
                            icon: "delete",
                            kind: BtnKind::Danger,
                            enabled: true,
                            act: Act::UninstallYt,
                            spin: false,
                        },
                    ],
                }
            };
            let item = ItemSpec {
                key: "yt",
                icon: "smart_display",
                icon_color: p.accent_strong,
                icon_tint: p.accent,
                icon_bg: 31,
                icon_border: 46,
                name: "yt-dlp",
                status_icon: if t.yt_exists { "check" } else { "download" },
                status: if t.yt_exists { crate::i18n::t(lang, "Installed") } else { crate::i18n::t(lang, "Not installed") },
                status_ok: t.yt_exists,
                desc: crate::i18n::t(lang, "Media download engine"),
                location: if t.yt_exists && !t.yt_path.is_empty() {
                    Some(LocationSpec {
                        path: &t.yt_path,
                        open: Some(Act::OpenYtLocation),
                    })
                } else {
                    None
                },
                actions: yt_actions,
                active: false,
                installing: t.installing_yt,
                progress: t.yt_percent,
            };
            if let Some(a) = draw_item(ui, &item, &p, iw, lang) {
                act = Some(a);
            }

            ui.add_space(10.0);

            let ejs_actions = if t.installing_ejs {
                Actions::Install {
                    text: crate::i18n::t(lang, "Installing...").to_string(),
                    cancel: Act::CancelEjs,
                }
            } else if !t.ejs_exists {
                Actions::Row {
                    active: false,
                    btns: vec![BtnSpec {
                        label: crate::i18n::t(lang, "Install").into(),
                        icon: "download",
                        kind: BtnKind::Primary,
                        enabled: true,
                        act: Act::InstallEjs,
                        spin: false,
                    }],
                }
            } else {
                Actions::Row {
                    active: false,
                    btns: vec![
                        BtnSpec {
                            label: crate::i18n::t(lang, "Open location").into(),
                            icon: "folder_open",
                            kind: BtnKind::Secondary,
                            enabled: true,
                            act: Act::OpenEjsLocation,
                            spin: false,
                        },
                        BtnSpec {
                            label: crate::i18n::t(lang, "Remove").into(),
                            icon: "delete",
                            kind: BtnKind::Danger,
                            enabled: true,
                            act: Act::UninstallEjs,
                            spin: false,
                        },
                    ],
                }
            };
            let item = ItemSpec {
                key: "ejs",
                icon: "code",
                icon_color: Color32::from_rgb(0xb3, 0x9d, 0xdb),
                icon_tint: Color32::from_rgb(0xb3, 0x9d, 0xdb),
                icon_bg: 26,
                icon_border: 41,
                name: "EJS",
                status_icon: if t.ejs_exists { "check" } else { "download" },
                status: if t.ejs_exists { crate::i18n::t(lang, "Installed") } else { crate::i18n::t(lang, "Not installed") },
                status_ok: t.ejs_exists,
                desc: crate::i18n::t(lang, "JavaScript runtime for YouTube signature solving"),
                location: if t.ejs_exists && !t.ejs_path.is_empty() {
                    Some(LocationSpec {
                        path: &t.ejs_path,
                        open: Some(Act::OpenEjsLocation),
                    })
                } else {
                    None
                },
                actions: ejs_actions,
                active: false,
                // Vue's EJS row has no `.tool-progress` (only the install
                // state), so the row keeps its 68px height while installing.
                installing: false,
                progress: 0,
            };
            if let Some(a) = draw_item(ui, &item, &p, iw, lang) {
                act = Some(a);
            }

            ui.add_space(12.0);
        });
    act
}

fn runtimes_card(t: &ToolsUi, ui: &mut Ui, w: f32, lang: &str) -> Option<Act> {
    let p = mk::pal();
    let mut act = None;
    let active_name = t
        .system_node
        .as_ref()
        .filter(|n| n.selected)
        .map(|_| crate::i18n::t(lang, "System Node.js").to_string())
        .or_else(|| {
            t.runtimes
                .iter()
                .find(|r| r.selected && r.installed)
                .map(|r| r.name.clone())
        });
    super::card_frame()
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.set_width(w - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            if card_header(
                ui,
                w - 2.0,
                "runtimes",
                "terminal",
                crate::i18n::t(lang, "JS Runtimes"),
                crate::i18n::t(lang, "Runtime used by yt-dlp to solve YouTube challenges."),
                if active_name.is_some() { Summary::Ok } else { Summary::Idle },
                if active_name.is_some() { "check_circle" } else { "tune" },
                active_name.as_deref().unwrap_or(crate::i18n::t(lang, "Not configured")),
                &p,
            ) {
                act = Some(Act::OpenBinFolder);
            }

            ui.add_space(12.0);
            let iw = w - 2.0 - 24.0;

            // `.system-tool-item` — System Node.js.
            let node_actions = match &t.system_node {
                Some(n) if n.selected => Actions::Row {
                    active: true,
                    btns: Vec::new(),
                },
                Some(_) => Actions::Row {
                    active: false,
                    btns: vec![BtnSpec {
                        label: crate::i18n::t(lang, "Use").into(),
                        icon: "check",
                        kind: BtnKind::Secondary,
                        enabled: !t.selecting_runtime && t.removing_runtime.is_empty(),
                        act: Act::SelectRuntime("system-node".into()),
                        spin: false,
                    }],
                },
                None => Actions::Row {
                    active: false,
                    btns: Vec::new(),
                },
            };
            let node = ItemSpec {
                key: "node",
                icon: "memory",
                icon_color: Color32::from_rgb(0x9f, 0xb8, 0xff),
                icon_tint: Color32::from_rgb(0x9f, 0xb8, 0xff),
                icon_bg: 26,
                icon_border: 41,
                name: crate::i18n::t(lang, "System Node.js"),
                status_icon: if t.system_node.is_some() {
                    "check_circle"
                } else {
                    "help_outline"
                },
                status: if t.system_node.is_some() {
                    crate::i18n::t(lang, "Detected")
                } else {
                    crate::i18n::t(lang, "Not detected")
                },
                status_ok: t.system_node.is_some(),
                desc: t
                    .system_node
                    .as_ref()
                    .map(|n| n.version.as_str())
                    .unwrap_or(crate::i18n::t(lang, "Not detected")),
                location: t
                    .system_node
                    .as_ref()
                    .filter(|n| !n.path.is_empty())
                    .map(|n| LocationSpec {
                        path: &n.path,
                        open: None,
                    }),
                actions: node_actions,
                active: false,
                installing: false,
                progress: 0,
            };
            if let Some(a) = draw_item(ui, &node, &p, iw, lang) {
                act = Some(a);
            }

            ui.add_space(10.0);
            section_label(ui, iw, &crate::i18n::t(lang, "Runtime options").to_uppercase(), &p);
            if !t.runtimes.is_empty() {
                ui.add_space(10.0);
            }

            for (idx, rt) in t.runtimes.iter().enumerate() {
                let installing = t.installing_runtime == rt.id;
                let actions = if installing {
                    Actions::Install {
                        text: if t.runtime_percent > 0 {
                            format!("{}%", t.runtime_percent)
                        } else {
                            crate::i18n::t(lang, "Installing...").to_string()
                        },
                        cancel: Act::CancelRuntime(rt.id.clone()),
                    }
                } else {
                    let mut btns = Vec::new();
                    let is_active = rt.selected && rt.installed;
                    if !is_active {
                        if rt.installed {
                            btns.push(BtnSpec {
                                label: crate::i18n::t(lang, "Use").into(),
                                icon: "check",
                                kind: BtnKind::Secondary,
                                enabled: !t.selecting_runtime && t.removing_runtime.is_empty(),
                                act: Act::SelectRuntime(rt.id.clone()),
                                spin: false,
                            });
                        } else {
                            btns.push(BtnSpec {
                                label: crate::i18n::t(lang, "Download").into(),
                                icon: "download",
                                kind: BtnKind::Primary,
                                enabled: t.installing_runtime.is_empty()
                                    && t.removing_runtime.is_empty(),
                                act: Act::InstallRuntime(rt.id.clone()),
                                spin: false,
                            });
                        }
                    }
                    if rt.installed {
                        let removing = t.removing_runtime == rt.id;
                        btns.push(BtnSpec {
                            label: if removing {
                                crate::i18n::t(lang, "Removing...").into()
                            } else {
                                crate::i18n::t(lang, "Remove").into()
                            },
                            icon: "delete",
                            kind: BtnKind::Danger,
                            enabled: t.removing_runtime.is_empty()
                                && t.installing_runtime.is_empty(),
                            act: Act::RemoveRuntime(rt.id.clone()),
                            spin: removing,
                        });
                    }
                    Actions::Row {
                        active: is_active,
                        btns,
                    }
                };
                let (status_icon, status) = if rt.selected && rt.installed {
                    ("check_circle", crate::i18n::t(lang, "In use"))
                } else if rt.installed {
                    ("check", crate::i18n::t(lang, "Installed"))
                } else {
                    ("download", crate::i18n::t(lang, "Not installed"))
                };
                let item = ItemSpec {
                    key: &rt.id,
                    icon: runtime_icon(&rt.id),
                    icon_color: if rt.installed {
                        p.status_complete
                    } else {
                        p.accent_strong
                    },
                    icon_tint: if rt.installed {
                        p.status_complete
                    } else {
                        p.accent
                    },
                    icon_bg: if rt.installed { 26 } else { 20 },
                    icon_border: if rt.installed { 41 } else { 36 },
                    name: &rt.name,
                    status_icon,
                    status,
                    status_ok: rt.installed,
                    desc: &rt.size,
                    location: if rt.installed && !rt.path.is_empty() {
                        Some(LocationSpec {
                            path: &rt.path,
                            open: None,
                        })
                    } else {
                        None
                    },
                    actions,
                    active: rt.selected && rt.installed,
                    installing,
                    progress: t.runtime_percent,
                };
                if let Some(a) = draw_item(ui, &item, &p, iw, lang) {
                    act = Some(a);
                }
                if idx + 1 < t.runtimes.len() {
                    ui.add_space(10.0);
                }
            }

            ui.add_space(12.0);
        });
    act
}

fn mergers_card(t: &ToolsUi, ui: &mut Ui, w: f32, lang: &str) -> Option<Act> {
    let p = mk::pal();
    let mut act = None;
    let active_name = t
        .system_ffmpeg
        .as_ref()
        .filter(|f| f.selected)
        .map(|_| crate::i18n::t(lang, "System FFmpeg").to_string())
        .or_else(|| {
            t.mergers
                .iter()
                .find(|m| m.selected && m.installed)
                .map(|m| m.name.clone())
        });
    super::card_frame()
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.set_width(w - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            if card_header(
                ui,
                w - 2.0,
                "mergers",
                "merge_type",
                crate::i18n::t(lang, "Media Merger"),
                crate::i18n::t(lang, "Tool yt-dlp uses to combine video + audio into one file."),
                if active_name.is_some() { Summary::Ok } else { Summary::Idle },
                if active_name.is_some() { "check_circle" } else { "tune" },
                active_name.as_deref().unwrap_or(crate::i18n::t(lang, "Not configured")),
                &p,
            ) {
                act = Some(Act::OpenBinFolder);
            }

            ui.add_space(12.0);
            let iw = w - 2.0 - 24.0;

            // `.system-tool-item` — System FFmpeg.
            let ff_actions = match &t.system_ffmpeg {
                Some(f) if f.selected => Actions::Row {
                    active: true,
                    btns: Vec::new(),
                },
                Some(_) => Actions::Row {
                    active: false,
                    btns: vec![BtnSpec {
                        label: crate::i18n::t(lang, "Use").into(),
                        icon: "check",
                        kind: BtnKind::Secondary,
                        enabled: !t.selecting_merger && t.removing_merger.is_empty(),
                        act: Act::SelectMerger("system-ffmpeg".into()),
                        spin: false,
                    }],
                },
                None => Actions::Row {
                    active: false,
                    btns: Vec::new(),
                },
            };
            let ff = ItemSpec {
                key: "ffmpeg",
                icon: "memory",
                icon_color: Color32::from_rgb(0x9f, 0xb8, 0xff),
                icon_tint: Color32::from_rgb(0x9f, 0xb8, 0xff),
                icon_bg: 26,
                icon_border: 41,
                name: crate::i18n::t(lang, "System FFmpeg"),
                status_icon: if t.system_ffmpeg.is_some() {
                    "check_circle"
                } else {
                    "help_outline"
                },
                status: if t.system_ffmpeg.is_some() {
                    crate::i18n::t(lang, "Detected")
                } else {
                    crate::i18n::t(lang, "Not detected")
                },
                status_ok: t.system_ffmpeg.is_some(),
                desc: if t.system_ffmpeg.is_some() {
                    crate::i18n::t(lang, "Use an already-installed FFmpeg")
                } else {
                    crate::i18n::t(lang, "Not detected")
                },
                location: t
                    .system_ffmpeg
                    .as_ref()
                    .filter(|f| !f.path.is_empty())
                    .map(|f| LocationSpec {
                        path: &f.path,
                        open: None,
                    }),
                actions: ff_actions,
                active: false,
                installing: false,
                progress: 0,
            };
            if let Some(a) = draw_item(ui, &ff, &p, iw, lang) {
                act = Some(a);
            }

            ui.add_space(10.0);
            section_label(ui, iw, &crate::i18n::t(lang, "Merger options").to_uppercase(), &p);
            if !t.mergers.is_empty() {
                ui.add_space(10.0);
            }

            for (idx, m) in t.mergers.iter().enumerate() {
                let installing = t.installing_merger == m.id;
                let actions = if installing {
                    Actions::Install {
                        text: if t.merger_percent > 0 {
                            format!("{}%", t.merger_percent)
                        } else {
                            crate::i18n::t(lang, "Installing...").to_string()
                        },
                        cancel: Act::CancelMerger(m.id.clone()),
                    }
                } else {
                    let mut btns = Vec::new();
                    let is_active = m.selected && m.installed;
                    if !is_active {
                        if m.installed {
                            btns.push(BtnSpec {
                                label: crate::i18n::t(lang, "Use").into(),
                                icon: "check",
                                kind: BtnKind::Secondary,
                                enabled: !t.selecting_merger && t.removing_merger.is_empty(),
                                act: Act::SelectMerger(m.id.clone()),
                                spin: false,
                            });
                        } else {
                            btns.push(BtnSpec {
                                label: crate::i18n::t(lang, "Download").into(),
                                icon: "download",
                                kind: BtnKind::Primary,
                                enabled: t.installing_merger.is_empty()
                                    && t.removing_merger.is_empty(),
                                act: Act::InstallMerger(m.id.clone()),
                                spin: false,
                            });
                        }
                    }
                    if m.installed {
                        let removing = t.removing_merger == m.id;
                        btns.push(BtnSpec {
                            label: if removing {
                                crate::i18n::t(lang, "Removing...").into()
                            } else {
                                crate::i18n::t(lang, "Remove").into()
                            },
                            icon: "delete",
                            kind: BtnKind::Danger,
                            enabled: t.removing_merger.is_empty()
                                && t.installing_merger.is_empty(),
                            act: Act::RemoveMerger(m.id.clone()),
                            spin: removing,
                        });
                    }
                    Actions::Row {
                        active: is_active,
                        btns,
                    }
                };
                let (status_icon, status) = if m.selected && m.installed {
                    ("check_circle", crate::i18n::t(lang, "In use"))
                } else if m.installed {
                    ("check", crate::i18n::t(lang, "Installed"))
                } else {
                    ("download", crate::i18n::t(lang, "Not installed"))
                };
                let item = ItemSpec {
                    key: &m.id,
                    icon: "movie_filter",
                    icon_color: if m.installed {
                        p.status_complete
                    } else {
                        p.accent_strong
                    },
                    icon_tint: if m.installed {
                        p.status_complete
                    } else {
                        p.accent
                    },
                    icon_bg: if m.installed { 26 } else { 20 },
                    icon_border: if m.installed { 41 } else { 36 },
                    name: &m.name,
                    status_icon,
                    status,
                    status_ok: m.installed,
                    desc: &m.size,
                    location: if m.installed && !m.path.is_empty() {
                        Some(LocationSpec {
                            path: &m.path,
                            open: None,
                        })
                    } else {
                        None
                    },
                    actions,
                    active: m.selected && m.installed,
                    installing,
                    progress: t.merger_percent,
                };
                if let Some(a) = draw_item(ui, &item, &p, iw, lang) {
                    act = Some(a);
                }
                if idx + 1 < t.mergers.len() {
                    ui.add_space(10.0);
                }
            }

            ui.add_space(12.0);
        });
    act
}

// ── Loaders ─────────────────────────────────────────────────────────────

fn load_yt_state(ctx: &Ctx) {
    ctx.emit("media-tools-yt-state", |app| async move {
        let exists = moonhunt_core::ytdlp::check_yt_dlp_exists(app.clone())?;
        let path = if exists {
            moonhunt_core::ytdlp::get_yt_dlp_path(app).unwrap_or_default()
        } else {
            String::new()
        };
        Ok(serde_json::json!({ "exists": exists, "path": path }))
    });
}

fn load_ejs_state(ctx: &Ctx) {
    ctx.emit("media-tools-ejs-state", |app| async move {
        let exists = moonhunt_core::ytdlp::check_ejs_exists(app.clone())?;
        let path = if exists {
            moonhunt_core::ytdlp::get_ejs_path(app).unwrap_or_default()
        } else {
            String::new()
        };
        Ok(serde_json::json!({ "exists": exists, "path": path }))
    });
}

fn load_runtimes(ctx: &Ctx) {
    ctx.emit("media-tools-runtimes", |app| async move {
        moonhunt_core::js_runtime::get_js_runtimes(app)
    });
}

fn load_mergers(ctx: &Ctx) {
    ctx.emit("media-tools-mergers", |app| async move {
        moonhunt_core::merger::get_mergers(app)
    });
}

// ── Action handling ─────────────────────────────────────────────────────

fn perform(state: &mut UiState, ctx: &Ctx, act: Act) {
    match act {
        Act::OpenBinFolder => ctx.run(|app| async move {
            moonhunt_core::ytdlp::open_app_bin_folder(app)
                .map(|_| String::new())
                .map_err(|e| format!("Failed to open folder: {e}"))
        }),
        Act::OpenYtLocation => ctx.run(|app| async move {
            moonhunt_core::ytdlp::open_yt_dlp_location(app)
                .map(|_| String::new())
                .map_err(|e| format!("Failed to open folder: {e}"))
        }),
        Act::OpenEjsLocation => ctx.run(|app| async move {
            moonhunt_core::ytdlp::open_ejs_location(app)
                .map(|_| String::new())
                .map_err(|e| format!("Failed to open folder: {e}"))
        }),
        Act::InstallYt => {
            state.media.tools.installing_yt = true;
            state.media.tools.yt_percent = 0;
            ctx.run(|app| async move {
                let evt = app.clone();
                let res = moonhunt_core::ytdlp::install_yt_dlp(app).await;
                evt.emit("media-tools-reload-yt", ());
                match res {
                    Ok(msg) => Ok(msg),
                    Err(e) if e.to_lowercase().contains("cancelled") => {
                        Ok("Install cancelled".to_string())
                    }
                    Err(e) => Err(format!("Failed to install yt-dlp: {e}")),
                }
            });
        }
        Act::CancelYt => ctx.run(|_app| async move {
            let _ = moonhunt_core::ytdlp::cancel_yt_dlp_install();
            Ok(String::new())
        }),
        Act::UninstallYt => ctx.run(|app| async move {
            let evt = app.clone();
            let res = moonhunt_core::ytdlp::uninstall_yt_dlp(app);
            evt.emit("media-tools-reload-yt", ());
            res.map_err(|e| format!("Failed to uninstall yt-dlp: {e}"))
        }),
        Act::InstallEjs => {
            state.media.tools.installing_ejs = true;
            ctx.run(|app| async move {
                let evt = app.clone();
                let res = moonhunt_core::ytdlp::install_ejs(app).await;
                evt.emit("media-tools-reload-ejs", ());
                match res {
                    Ok(msg) => Ok(msg),
                    Err(e) if e.to_lowercase().contains("cancelled") => {
                        Ok("Install cancelled".to_string())
                    }
                    Err(e) => Err(format!("Failed to install EJS: {e}")),
                }
            });
        }
        Act::CancelEjs => ctx.run(|_app| async move {
            let _ = moonhunt_core::ytdlp::cancel_ejs_install();
            Ok(String::new())
        }),
        Act::UninstallEjs => ctx.run(|app| async move {
            let evt = app.clone();
            let res = moonhunt_core::ytdlp::uninstall_ejs(app);
            evt.emit("media-tools-reload-ejs", ());
            res.map_err(|e| format!("Failed to uninstall EJS: {e}"))
        }),
        Act::SelectRuntime(id) => {
            state.media.tools.selecting_runtime = true;
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::js_runtime::select_js_runtime(app, id);
                evt.emit("media-tools-reload-runtimes", ());
                res.map_err(|e| format!("Failed to select runtime: {e}"))
            });
        }
        Act::RemoveRuntime(id) => {
            state.media.tools.removing_runtime = id.clone();
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::js_runtime::remove_js_runtime(app, id);
                evt.emit("media-tools-reload-runtimes", ());
                res.map_err(|e| format!("Failed to remove runtime: {e}"))
            });
        }
        Act::InstallRuntime(id) => {
            state.media.tools.installing_runtime = id.clone();
            state.media.tools.runtime_percent = 0;
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::js_runtime::install_js_runtime(app, id).await;
                evt.emit("media-tools-reload-runtimes", ());
                match res {
                    Ok(msg) => Ok(msg),
                    Err(e) if e.to_lowercase().contains("cancelled") => {
                        Ok("Install cancelled".to_string())
                    }
                    Err(e) => Err(format!("Failed to install runtime: {e}")),
                }
            });
        }
        Act::CancelRuntime(id) => ctx.run(move |_app| async move {
            let _ = moonhunt_core::js_runtime::cancel_js_runtime_install(id);
            Ok(String::new())
        }),
        Act::SelectMerger(id) => {
            state.media.tools.selecting_merger = true;
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::merger::select_merger(app, id);
                evt.emit("media-tools-reload-mergers", ());
                res.map_err(|e| format!("Failed to select merger: {e}"))
            });
        }
        Act::RemoveMerger(id) => {
            state.media.tools.removing_merger = id.clone();
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::merger::remove_merger(app, id);
                evt.emit("media-tools-reload-mergers", ());
                res.map_err(|e| format!("Failed to remove merger: {e}"))
            });
        }
        Act::InstallMerger(id) => {
            state.media.tools.installing_merger = id.clone();
            state.media.tools.merger_percent = 0;
            ctx.run(move |app| async move {
                let evt = app.clone();
                let res = moonhunt_core::merger::install_merger(app, id).await;
                evt.emit("media-tools-reload-mergers", ());
                match res {
                    Ok(msg) => Ok(msg),
                    Err(e) if e.to_lowercase().contains("cancelled") => {
                        Ok("Install cancelled".to_string())
                    }
                    Err(e) => Err(format!("Failed to install merger: {e}")),
                }
            });
        }
        Act::CancelMerger(id) => ctx.run(move |_app| async move {
            let _ = moonhunt_core::merger::cancel_merger_install(id);
            Ok(String::new())
        }),
    }
}

// ── Events ──────────────────────────────────────────────────────────────

fn percent_of(payload: &serde_json::Value) -> u32 {
    payload
        .get("percent")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 100.0) as u32
}

fn runtime_from(v: &serde_json::Value) -> Option<RuntimeRow> {
    let id = v.get("id").and_then(|x| x.as_str())?.to_string();
    Some(RuntimeRow {
        id,
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        installed: v
            .get("installed")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        selected: v
            .get("selected")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        path: v
            .get("path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        size: v
            .get("size")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn merger_from(v: &serde_json::Value) -> Option<MergerRow> {
    let id = v.get("id").and_then(|x| x.as_str())?.to_string();
    Some(MergerRow {
        id,
        name: v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        installed: v
            .get("installed")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        selected: v
            .get("selected")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        path: v
            .get("path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        size: v
            .get("size")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Installer progress events (`yt_install_progress`, `ejs_install_progress`,
/// `js_runtime_install_progress`, `merger_install_progress`) plus the fetch
/// and refresh ticks this card spawns itself.
pub fn apply_event(state: &mut UiState, name: &str, payload: &serde_json::Value) -> bool {
    let t = &mut state.media.tools;
    let data = || payload.get("data");
    match name {
        "media-tools-yt-state" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(d) = data() {
                    t.yt_exists = d
                        .get("exists")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    t.yt_path = d
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
            true
        }
        "media-tools-ejs-state" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(d) = data() {
                    t.ejs_exists = d
                        .get("exists")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    t.ejs_path = d
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
            true
        }
        "media-tools-runtimes" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(d) = data() {
                    t.runtimes = d
                        .get("runtimes")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(runtime_from).collect())
                        .unwrap_or_default();
                    t.system_node = d.get("systemNode").and_then(|v| {
                        if v.is_null() {
                            return None;
                        }
                        Some(SystemNode {
                            version: v
                                .get("version")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            path: v
                                .get("path")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            selected: v
                                .get("selected")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false),
                        })
                    });
                }
            }
            true
        }
        "media-tools-mergers" => {
            if payload.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(d) = data() {
                    t.mergers = d
                        .get("mergers")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(merger_from).collect())
                        .unwrap_or_default();
                    t.system_ffmpeg = d.get("systemFfmpeg").and_then(|v| {
                        if v.is_null() {
                            return None;
                        }
                        Some(SystemFfmpeg {
                            path: v
                                .get("path")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            selected: v
                                .get("selected")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false),
                        })
                    });
                }
            }
            true
        }
        "media-tools-reload-yt" => {
            t.installing_yt = false;
            t.yt_percent = 0;
            t.dirty_yt = true;
            true
        }
        "media-tools-reload-ejs" => {
            t.installing_ejs = false;
            t.dirty_ejs = true;
            true
        }
        "media-tools-reload-runtimes" => {
            t.installing_runtime.clear();
            t.runtime_percent = 0;
            t.selecting_runtime = false;
            t.removing_runtime.clear();
            t.dirty_runtimes = true;
            true
        }
        "media-tools-reload-mergers" => {
            t.installing_merger.clear();
            t.merger_percent = 0;
            t.selecting_merger = false;
            t.removing_merger.clear();
            t.dirty_mergers = true;
            true
        }
        "yt_install_progress" => {
            t.yt_percent = percent_of(payload);
            true
        }
        "js_runtime_install_progress" => {
            let id = payload
                .get("runtimeId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if id == t.installing_runtime {
                t.runtime_percent = percent_of(payload);
            }
            true
        }
        "merger_install_progress" => {
            let id = payload
                .get("mergerId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if id == t.installing_merger {
                t.merger_percent = percent_of(payload);
            }
            true
        }
        _ => false,
    }
}
