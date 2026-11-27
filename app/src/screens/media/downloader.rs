//! URL card + format-scan modal of the Media screen (`.media-downloader-card`).
//!
//! Pixel port of `Media.vue` (v0.0.6): template 4–112 (card), 462–585 (modal)
//! and the `<script setup>` behaviour block (588–950). Every metric below
//! quotes the scoped CSS value it reproduces.

use super::Ctx;
use crate::state::UiState;
use crate::ui::icons as mi;
use crate::ui::tokens::Palette;
use crate::ui::widgets as mk;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextEdit, Ui,
};

// ── State (Media.vue refs) ──────────────────────────────────────────────

/// `downloadState` — `idle | scanning | selecting | downloading | running |
/// error`.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DlState {
    #[default]
    Idle,
    Scanning,
    Selecting,
    Downloading,
    Running,
    Error,
}

/// `selectedBrowser` — `'firefox' | 'chrome'` (default firefox).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Firefox,
    Chrome,
}

impl Browser {
    fn key(self) -> &'static str {
        match self {
            Browser::Firefox => "firefox",
            Browser::Chrome => "chrome",
        }
    }
}

impl Default for Browser {
    fn default() -> Self {
        Browser::Firefox
    }
}

/// One `YtFormat` row of the scan result (`ScanFormatsResult.formats`).
#[derive(Clone, Default)]
pub struct YtFormat {
    pub format_id: String,
    pub resolution: String,
    pub has_video: bool,
    pub has_audio: bool,
    pub filesize: Option<u64>,
    pub label: String,
    pub requires_merger: bool,
}

impl YtFormat {
    fn from_value(v: &serde_json::Value) -> Option<Self> {
        let s = |k: &str| {
            v.get(k)
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        };
        let format_id = s("format_id");
        if format_id.is_empty() {
            return None;
        }
        Some(Self {
            format_id,
            resolution: s("resolution"),
            has_video: v.get("has_video").and_then(|x| x.as_bool()).unwrap_or(false),
            has_audio: v.get("has_audio").and_then(|x| x.as_bool()).unwrap_or(false),
            filesize: v.get("filesize").and_then(|x| x.as_u64()),
            label: s("label"),
            requires_merger: v
                .get("requires_merger")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        })
    }
}

/// Downloader-card state (`Media.vue` template refs + script refs).
#[derive(Default)]
pub struct DlUi {
    /// `url`
    pub url: String,
    /// `path`
    pub path: String,
    /// `selectedBrowser`
    pub browser: Browser,
    /// `downloadState`
    pub state: DlState,
    /// `showMediaModal`
    pub show_modal: bool,
    /// `isStarting`
    pub is_starting: bool,
    /// `downloadError`
    pub error: String,
    /// `dialedUrl`
    pub dialed_url: String,
    /// `ytLog`
    pub log: Vec<String>,
    /// `scannedFormats`
    pub formats: Vec<YtFormat>,
    /// `scanMergerAvailable`
    pub merger_available: bool,
    /// `selectedFormatId`
    pub selected_format: Option<String>,
    /// `activeScanId`
    pub active_scan_id: Option<String>,
    /// `ytStarted`
    pub started: bool,
    /// `activeTaskId` (stale-result guard; the JS version is a plain counter).
    pub task_id: u64,
    /// egui context stashed by [`card`] — [`modal`] renders through `Area`s and
    /// this module's signature has no `Ui` handle.
    egui_ctx: Option<egui::Context>,
}

impl DlUi {
    /// `visibleFormats` — without a merger, video-only high-res formats (which
    /// need merging to carry audio) are hidden.
    fn visible_formats(&self) -> Vec<&YtFormat> {
        self.formats
            .iter()
            .filter(|f| self.merger_available || !f.requires_merger)
            .collect()
    }

    /// `resumeTaskVisible`: the strip is hidden while the modal is open and
    /// only exists for in-flight / failed tasks.
    fn resume_visible(&self) -> bool {
        !self.show_modal
            && matches!(
                self.state,
                DlState::Scanning | DlState::Selecting | DlState::Downloading | DlState::Error
            )
    }

    /// `resumeTaskLabel`
    fn resume_label(&self) -> &'static str {
        match self.state {
            DlState::Scanning => "Scanning in progress",
            DlState::Selecting => "Formats ready",
            DlState::Downloading => "Starting download",
            DlState::Error => "Media task needs attention",
            _ => "",
        }
    }
}

/// `detected` — YouTube URL sniff (`/(?:youtube\.com\/watch\?v=|youtu\.be\/|
/// youtube\.com\/shorts\/)/i`).
fn detected(dl: &DlUi) -> Option<&'static str> {
    if dl.url.is_empty() {
        return None;
    }
    let u = dl.url.to_lowercase();
    let is_yt = u.contains("youtube.com/watch?v=")
        || u.contains("youtu.be/")
        || u.contains("youtube.com/shorts/");
    Some(if is_yt {
        "YouTube"
    } else {
        "Unknown/unsupported"
    })
}

// ── Font helpers ────────────────────────────────────────────────────────

fn bold(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::bold_family())
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::semibold_family())
}

fn regular(size: f32) -> FontId {
    FontId::proportional(size)
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
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
) -> Rect {
    let painter = ui.painter();
    // Cursive script: a per-character layout would break the joins and run the
    // letters left to right, so paint the whole string as one shaped run.
    if crate::rtl::has_rtl(text) {
        let g = painter.layout_no_wrap(text.to_owned(), font, color);
        let r = anchor.anchor_size(at, g.size());
        painter.galley(r.min, g, color);
        return r;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Rect::from_min_size(at, vec2(0.0, 0.0));
    }
    let galleys: Vec<std::sync::Arc<egui::Galley>> = chars
        .iter()
        .map(|c| painter.layout_no_wrap(c.to_string(), font.clone(), color))
        .collect();
    let advances: Vec<f32> = galleys.iter().map(|g| g.size().x).collect();
    let total: f32 = advances.iter().sum::<f32>() + spacing * (chars.len() as f32 - 1.0);
    let height = galleys[0].size().y;
    let ax = match anchor.x() {
        egui::Align::Min => 0.0,
        egui::Align::Center => -total / 2.0,
        egui::Align::Max => -total,
    };
    let ay = match anchor.y() {
        egui::Align::Min => 0.0,
        egui::Align::Center => -height / 2.0,
        egui::Align::Max => -height,
    };
    let mut x = at.x + ax;
    let y = at.y + ay;
    for (g, adv) in galleys.iter().zip(advances.iter()) {
        painter.galley(pos2(x, y), g.clone(), color);
        x += adv + spacing;
    }
    Rect::from_min_size(pos2(at.x + ax, y), vec2(total, height))
}

// ── Painting helpers ────────────────────────────────────────────────────

fn inset(r: Rect, pad: f32) -> Rect {
    Rect::from_min_max(pos2(r.min.x + pad, r.min.y), pos2(r.max.x - pad, r.max.y))
}

fn row(ui: &mut Ui, h: f32) -> Rect {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    r
}

/// `.nd-divider` — `linear-gradient(90deg, transparent, border-strong,
/// transparent)`.
fn hgrad_line(ui: &Ui, rect: Rect, color: Color32) {
    let n = 48;
    let w = rect.width() / n as f32;
    for i in 0..n {
        let t = (i as f32 + 0.5) / n as f32;
        // 0 → 1 over the first half, 1 → 0 over the second.
        let a = (1.0 - (t * 2.0 - 1.0).abs()).clamp(0.0, 1.0);
        let c = color.gamma_multiply(a);
        ui.painter().rect_filled(
            Rect::from_min_size(pos2(rect.min.x + i as f32 * w, rect.min.y), vec2(w + 0.5, rect.height())),
            CornerRadius::same(0),
            c,
        );
    }
}

/// `.nd-spinner` / `.media-spinner` / `.media-big-spinner`: 0.8s rotating
/// ring (track accent@.15, 90° head in accent-strong).
fn spinner(ui: &Ui, center: Pos2, diameter: f32, thickness: f32) {
    let p = mk::pal();
    let t = ui.ctx().input(|i| i.time) as f32;
    let rot = (t / 0.8).fract() * std::f32::consts::TAU;
    let r = (diameter - thickness) * 0.5;
    ui.painter()
        .circle_stroke(center, r, Stroke::new(thickness, mk::blend(p.bg_card, p.accent, 0.15)));
    let n = 14;
    let start = rot - std::f32::consts::FRAC_PI_2 - std::f32::consts::FRAC_PI_4;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = start + (i as f32 / n as f32) * std::f32::consts::FRAC_PI_2;
        pts.push(center + vec2(r * a.cos(), r * a.sin()));
    }
    ui.painter()
        .add(egui::Shape::line(pts, Stroke::new(thickness, p.accent_strong)));
    ui.ctx().request_repaint();
}

// ── Buttons ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum BtnKind {
    Primary,
    Secondary,
    Danger,
}

fn btn_height() -> f32 {
    // `.nd-btn` padding 10px 16px; the inherited `line-height: 1.5` gives a
    // 18.75px line box for the 12.5px label → 38.75, i.e. 39px box (measured
    // off the reference capture). The secondary border is drawn inside.
    10.0 + 19.0 + 10.0
}

/// `.nd-btn` (primary / secondary / danger). Returns true when clicked.
fn nd_button(
    ui: &mut Ui,
    id: egui::Id,
    rect: Rect,
    icon: Option<&str>,
    label: &str,
    kind: BtnKind,
    enabled: bool,
) -> bool {
    let p = mk::pal();
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = enabled && resp.hovered();
    let painter = ui.painter();
    match kind {
        BtnKind::Primary => {
            let (c0, c1) = (p.accent, mk::blend(p.bg_card, p.accent, 0.85));
            // `.nd-btn-primary`: `box-shadow 0 2px 6px rgba(accent,.2)`. CSS
            // `opacity: .5` fades fill and shadow together, and an outer shadow
            // is clipped to outside the border box — so erase the rings from
            // under the (semi-transparent) fill before painting it.
            super::soft_shadow(
                ui,
                rect,
                10,
                p.accent,
                2.0,
                6.0,
                // The ring stack accumulates to ~2× the CSS alpha.
                if enabled { 26 } else { 13 },
            );
            painter.rect_filled(rect, CornerRadius::same(10), p.bg_card);
            // `opacity: .5` must halve *alpha* only — `gamma_multiply` also
            // scales the premultiplied RGB, which double-darkens the fill.
            let a = if enabled { 255 } else { 128 };
            let fade = |c: Color32| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
            super::grad135(ui, rect, 10, fade(c0), fade(c1));
            // `inset 0 1px 0 rgba(255,255,255,.15)`: a hairline on the top arc.
            painter.line_segment(
                [
                    pos2(rect.min.x + 10.0, rect.min.y + 1.0),
                    pos2(rect.max.x - 10.0, rect.min.y + 1.0),
                ],
                Stroke::new(1.0, Color32::from_white_alpha(if enabled { 38 } else { 19 })),
            );
        }
        BtnKind::Secondary => {
            let fill = if hovered {
                mk::blend(p.bg_card, p.accent, 0.08)
            } else {
                mk::blend(p.bg_card, Color32::WHITE, 0.03)
            };
            painter.rect_filled(rect, CornerRadius::same(10), fill);
            let border = if hovered {
                mk::blend(p.bg_card, p.accent, 0.2)
            } else {
                p.border
            };
            painter.rect_stroke(
                rect,
                CornerRadius::same(10),
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
        }
        BtnKind::Danger => {
            let a = if hovered { 0.16 } else { 0.08 };
            painter.rect_filled(
                rect,
                CornerRadius::same(10),
                mk::blend(p.bg_card, p.status_error, a),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(10),
                Stroke::new(
                    1.0,
                    mk::blend(p.bg_card, p.status_error, if hovered { 0.32 } else { 0.2 }),
                ),
                StrokeKind::Inside,
            );
        }
    }
    let mut color = match kind {
        BtnKind::Primary => Color32::from_rgb(0x0a, 0x0e, 0x1a),
        BtnKind::Secondary => {
            if hovered {
                p.accent_strong
            } else {
                p.text_muted
            }
        }
        BtnKind::Danger => {
            if hovered {
                Color32::from_rgb(0xff, 0x8a, 0x80)
            } else {
                p.status_error
            }
        }
    };
    if !enabled {
        // `.nd-btn:disabled { opacity: .5 }` — pre-composite the label against
        // the card, because the fill below it is already the 50% blend (a
        // group opacity would fade text and fill as one layer).
        color = mk::blend(p.bg_card, color, 0.5);
    }
    let font = bold(12.5);
    let tw = text_width(ui, label, font.clone());
    let content = if icon.is_some() { 16.0 + 6.0 + tw } else { tw };
    let mut x = rect.center().x - content / 2.0;
    let cy = rect.center().y;
    if let Some(icon) = icon {
        mi::paint_centered(ui.painter(), pos2(x + 8.0, cy), icon, 16.0, color);
        x += 22.0;
    }
    draw_text(ui, pos2(x, cy), Align2::LEFT_CENTER, label, font, color);
    enabled && resp.clicked()
}

// ── Inputs ──────────────────────────────────────────────────────────────

/// `.nd-input` shell + the text row. Returns true when Enter was pressed.
fn text_edit_in(
    ui: &mut Ui,
    fid: egui::Id,
    rect: Rect,
    value: &mut String,
    placeholder: &str,
    pad: (i8, i8),
    size: f32,
) -> bool {
    let p = mk::pal();
    let font = FontId::proportional(size);
    let line_h = ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
    let text_rect = Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
    let mut enter = false;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .scope_id(fid.with("scope")),
        |ui| {
            let resp = ui.add(
                TextEdit::singleline(value)
                    .id(fid)
                    .desired_width(text_rect.width())
                    .font(font.clone())
                    .text_color(p.text)
                    .hint_text(
                        egui::RichText::new(placeholder)
                            .size(size)
                            .color(p.text_dim.gamma_multiply(0.55)),
                    )
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                        left: pad.0,
                        right: pad.1,
                        top: 0,
                        bottom: 0,
                    })),
            );
            // A singleline `TextEdit` surrenders focus on Enter, so accept a
            // `lost_focus` frame too (Vue binds `@keyup.enter="start"` on the
            // URL input only).
            let hit_enter = ui.input(|i| i.key_pressed(Key::Enter));
            if hit_enter && (resp.has_focus() || resp.lost_focus()) {
                enter = true;
            }
        },
    );
    // The scoped TextEdit shrinks the parent's cursor to the text line box
    // (`scope_builder` max_rect) instead of the input row — restore the row.
    ui.advance_cursor_after_rect(rect);
    enter
}

fn input_shell(ui: &Ui, rect: Rect, focused: bool, hovered: bool) {
    let p = mk::pal();
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
        // `.nd-input:hover { border-color: rgba(255,255,255,0.1) }`
        mk::blend(p.bg_card, Color32::WHITE, 0.1)
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
}

// ── Card ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum CardAct {
    Clear,
    Start,
    StartOrOpen,
    Browse,
    View,
    Cancel,
}

/// `.media-downloader-card` — URL field, browser-cookie tabs, save folder,
/// footer actions and the resume strip.
pub fn card(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    let mut act: Option<CardAct> = None;
    super::card_frame()
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.set_width(w - 2.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            // t() before the &mut borrow below.
            let sub_label = state.t("Paste a YouTube (or supported) URL to download.");
            let lang = state.settings.language.clone();
            let dl = &mut state.media.dl;
            dl.egui_ctx = Some(ui.ctx().clone());

            // ── Header: `padding: 14px 16px 10px` + 36px avatar ──────────
            let (h, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::hover());
            let avatar = Rect::from_min_size(pos2(h.min.x + 16.0, h.min.y + 14.0), vec2(36.0, 36.0));
            // `.nd-avatar`: linear-gradient(135deg, accent .12, accent .04) +
            // 1px accent .12 border, radius 10.
            super::grad135(
                ui,
                avatar,
                10,
                mk::blend(p.bg_card, p.accent, 0.12),
                mk::blend(p.bg_card, p.accent, 0.04),
            );
            ui.painter().rect_stroke(
                avatar,
                CornerRadius::same(10),
                Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.12)),
                StrokeKind::Inside,
            );
            mi::paint_centered(ui.painter(), avatar.center(), "play_circle", 18.0, p.accent_strong);

            // `.nd-card-header .nd-title { font-size: 15px }` (700, lh 1.2),
            // `.nd-subtitle` 11px muted + 2px margin.
            let title_font = bold(15.0);
            let sub_font = regular(11.0);
            let title_h = ui.fonts_mut(|f| f.row_height(&title_font)).max(18.0);
            let sub_h = ui.fonts_mut(|f| f.row_height(&sub_font));
            let block = title_h + 2.0 + sub_h;
            let top = h.min.y + 14.0 + (36.0 - block) / 2.0;
            let tx = avatar.max.x + 10.0;
            draw_text(
                ui,
                pos2(tx, top + title_h / 2.0),
                Align2::LEFT_CENTER,
                crate::i18n::t(&lang, "Media Downloader"),
                title_font,
                p.text,
            );
            draw_text(
                ui,
                pos2(tx, top + title_h + 2.0 + sub_h / 2.0),
                Align2::LEFT_CENTER,
                sub_label,
                sub_font,
                p.text_muted,
            );

            // ── `.nd-divider` (margin: 0 16px) ───────────────────────────
            let d = row(ui, 1.0);
            hgrad_line(
                ui,
                Rect::from_min_max(pos2(d.min.x + 16.0, d.min.y), pos2(d.max.x - 16.0, d.max.y)),
                p.border_strong,
            );

            // ── `.nd-card-body`: padding 12px 16px 8px; gap 14px ─────────
            ui.add_space(12.0);

            // Video URL field.
            label_row(ui, &p, "link", crate::i18n::t(&lang, "Video URL"), None);
            ui.add_space(7.0);
            let mut want_start = false;
            {
                let r = row(ui, 36.0);
                let rect = inset(r, 16.0);
                let fid = egui::Id::new("media-url");
                let hovered = ui.rect_contains_pointer(rect);
                let focused = ui.ctx().memory(|m| m.has_focus(fid));
                if ui
                    .interact(rect, fid.with("bg"), Sense::click())
                    .clicked()
                {
                    ui.ctx().memory_mut(|m| m.request_focus(fid));
                }
                input_shell(ui, rect, focused, hovered);
                let show_clear = (hovered || focused) && !dl.url.is_empty();
                let pad_r: i8 = if show_clear { 36 } else { 10 };
                if text_edit_in(
                    ui,
                    fid,
                    rect,
                    &mut dl.url,
                    "https://www.youtube.com/watch?v=...",
                    (10, pad_r),
                    12.0,
                ) {
                    want_start = true;
                }
                if show_clear {
                    // `.nd-input-clear`: 24×24 r7, close 13px, right 8.
                    let c = Rect::from_min_size(
                        pos2(rect.max.x - 8.0 - 24.0, rect.center().y - 12.0),
                        vec2(24.0, 24.0),
                    );
                    let rr = ui.interact(c, fid.with("clear"), Sense::click());
                    if rr.hovered() {
                        ui.painter().rect_filled(
                            c,
                            CornerRadius::same(7),
                            mk::blend(p.bg_card, p.accent, 0.15),
                        );
                    }
                    mi::paint_centered(
                        ui.painter(),
                        c.center(),
                        "close",
                        13.0,
                        if rr.hovered() { p.text } else { p.text_muted },
                    );
                    if rr.clicked() {
                        dl.url.clear();
                        ui.ctx().memory_mut(|m| m.request_focus(fid));
                    }
                }
            }
            if want_start {
                act = Some(CardAct::Start);
            }

            ui.add_space(14.0);

            // `.media-detected` chip (only when the URL sniffs as YouTube).
            if let Some(label) = detected(dl) {
                let label = crate::i18n::t(&lang, label);
                let (r, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
                let font = bold(11.0);
                let tw = text_width(ui, label, font.clone());
                let chip = Rect::from_min_size(
                    pos2(r.min.x + 16.0, r.min.y),
                    // padding 5/10 + icon 14 + gap 6 + 2px border.
                    vec2(10.0 + 14.0 + 6.0 + tw + 10.0 + 2.0, 26.0),
                );
                ui.painter().rect_filled(
                    chip,
                    CornerRadius::same(100),
                    mk::blend(p.bg_card, p.status_complete, 0.08),
                );
                ui.painter().rect_stroke(
                    chip,
                    CornerRadius::same(100),
                    Stroke::new(1.0, mk::blend(p.bg_card, p.status_complete, 0.18)),
                    StrokeKind::Inside,
                );
                mi::paint_centered(
                    ui.painter(),
                    pos2(chip.min.x + 10.0 + 7.0, chip.center().y),
                    "check_circle",
                    14.0,
                    p.status_complete,
                );
                draw_text(
                    ui,
                    pos2(chip.min.x + 10.0 + 14.0 + 6.0, chip.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    font,
                    p.status_complete,
                );
                ui.add_space(14.0);
            }

            // Browser cookies field.
            label_row(
                ui,
                &p,
                "cookie",
                crate::i18n::t(&lang, "Browser Cookies"),
                Some(crate::i18n::t(&lang, "(for age-restricted content)")),
            );
            ui.add_space(7.0 + 2.0); // `.nd-field` gap 7 + `.media-browser-tabs` margin-top 2
            browser_tabs(ui, &p, dl);
            ui.add_space(7.0 + 2.0); // `.nd-field` gap 7 + hint margin-top 2
            {
                let r = row(ui, 16.0);
                draw_text(
                    ui,
                    pos2(r.min.x + 16.0, r.min.y + 8.0),
                    Align2::LEFT_CENTER,
                    crate::i18n::t(&lang, "Firefox is recommended. Select your browser to use its cookies for YouTube access."),
                    regular(11.0),
                    p.text_dim,
                );
            }
            ui.add_space(14.0);

            // Save folder field.
            label_row(
        ui,
        &p,
        "folder",
        crate::i18n::t(&lang, "Save Folder"),
        Some(crate::i18n::t(&lang, "(optional)")),
    );
            ui.add_space(7.0);
            {
                let r = row(ui, 36.0);
                let rect = inset(r, 16.0);
                let fid = egui::Id::new("media-path");
                let hovered = ui.rect_contains_pointer(rect);
                let focused = ui.ctx().memory(|m| m.has_focus(fid));
                if ui
                    .interact(rect, fid.with("bg"), Sense::click())
                    .clicked()
                {
                    ui.ctx().memory_mut(|m| m.request_focus(fid));
                }
                input_shell(ui, rect, focused, hovered);
                // `.nd-path-input { padding-right: 36px }`
                text_edit_in(ui, fid, rect, &mut dl.path, "C:\\Downloads", (10, 36), 12.0);
                // `.nd-input-action`: 26×26 r7 at right 10, folder_open 18px.
                let c = Rect::from_min_size(
                    pos2(rect.max.x - 10.0 - 26.0, rect.center().y - 13.0),
                    vec2(26.0, 26.0),
                );
                let rr = ui.interact(c, fid.with("browse"), Sense::click());
                if rr.hovered() {
                    ui.painter()
                        .rect_filled(c, CornerRadius::same(7), mk::blend(p.bg_card, p.accent, 0.10));
                }
                mi::paint_centered(
                    ui.painter(),
                    c.center(),
                    "folder_open",
                    18.0,
                    if rr.hovered() { p.accent_strong } else { p.text_muted },
                );
                if rr.clicked() {
                    act = Some(CardAct::Browse);
                }
            }
            ui.add_space(8.0); // `.nd-card-body` bottom padding (12px 16px 8px)

            // ── `.nd-card-footer`: border-top, padding 12 16 16, gap 8 ───
            let ft = row(ui, 1.0);
            ui.painter().rect_filled(
                Rect::from_min_size(ft.min, vec2(ft.width(), 1.0)),
                CornerRadius::same(0),
                p.border,
            );
            ui.add_space(12.0);
            {
                let h = btn_height();
                let r = row(ui, h);
                // `.nd-card-footer` pads 16px each side; both buttons are
                // `.nd-btn { flex: 1 }` with an 8px gap (the `.media-footer-btn`
                // flex:0 override at the top of the sheet loses to `.nd-btn`,
                // which is defined later with equal specificity).
                let inner = r.width() - 32.0;
                let w = ((inner - 8.0) / 2.0).max(0.0);
                let y = r.min.y;
                let secondary = Rect::from_min_size(pos2(r.min.x + 16.0, y), vec2(w, h));
                let primary = Rect::from_min_size(pos2(secondary.max.x + 8.0, y), vec2(w, h));
                if nd_button(
                    ui,
                    egui::Id::new("media-clear"),
                    secondary,
                    Some("restart_alt"),
                    crate::i18n::t(&lang, "Clear"),
                    BtnKind::Secondary,
                    true,
                ) {
                    act = Some(CardAct::Clear);
                }
                let enabled = dl.is_starting || !dl.url.trim().is_empty();
                if nd_button(
                    ui,
                    egui::Id::new("media-start"),
                    primary,
                    Some(if dl.is_starting { "hourglass_top" } else { "download" }),
                    if dl.is_starting {
                    crate::i18n::t(&lang, "Scanning...")
                } else {
                    crate::i18n::t(&lang, "Scan Formats")
                },
                    BtnKind::Primary,
                    enabled,
                ) {
                    act = Some(CardAct::StartOrOpen);
                }
            }
            ui.add_space(16.0);

            // ── `.media-resume`: margin 0 16 16, padding 10 12 ──────────
            if dl.resume_visible() {
                let (r, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 54.0), Sense::hover());
                let strip = inset(r, 16.0);
                ui.painter().rect_filled(
                    strip,
                    CornerRadius::same(10),
                    mk::blend(p.bg_card, Color32::WHITE, 0.02),
                );
                ui.painter().rect_stroke(
                    strip,
                    CornerRadius::same(10),
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );
                // Left: optional 14px spinner + 12/600 dim label.
                let mut x = strip.min.x + 12.0;
                let cy = strip.center().y;
                if dl.state == DlState::Scanning {
                    spinner(ui, pos2(x + 7.0, cy), 14.0, 2.0);
                    x += 14.0 + 8.0;
                }
                draw_text(
                    ui,
                    pos2(x, cy),
                    Align2::LEFT_CENTER,
                    crate::i18n::t(&lang, dl.resume_label()),
                    semibold(12.0),
                    p.text_dim,
                );

                // Right: View (secondary) + Cancel (danger), 32px / 11.5px.
                let view_label = crate::i18n::t(&lang, "View");
                let cancel_label = crate::i18n::t(&lang, "Cancel");
                let btn_font = bold(11.5); // `.media-resume-btn { font-size: 11.5px }` (700)
                let view_w = 16.0 + 6.0 + text_width(ui, view_label, btn_font.clone()) + 24.0 + 2.0;
                let cancel_w = 16.0 + 6.0 + text_width(ui, cancel_label, btn_font) + 24.0 + 2.0;
                let rr = strip.max.x - 12.0;
                let cancel = Rect::from_min_size(
                    pos2(rr - cancel_w, cy - 16.0),
                    vec2(cancel_w, 32.0),
                );
                let view = Rect::from_min_size(
                    pos2(cancel.min.x - 8.0 - view_w, cy - 16.0),
                    vec2(view_w, 32.0),
                );
                if small_button(
                    ui,
                    egui::Id::new("media-resume-view"),
                    view,
                    "visibility",
                    view_label,
                    BtnKind::Secondary,
                ) {
                    act = Some(CardAct::View);
                }
                if small_button(
                    ui,
                    egui::Id::new("media-resume-cancel"),
                    cancel,
                    "close",
                    cancel_label,
                    BtnKind::Danger,
                ) {
                    act = Some(CardAct::Cancel);
                }
                ui.add_space(16.0); // `margin-bottom: 16px`
            }
        });

    match act {
        Some(CardAct::Start) => start(state, ctx),
        // `startOrOpen()`: while a scan runs the button re-opens the modal.
        Some(CardAct::StartOrOpen) => {
            if state.media.dl.is_starting {
                state.media.dl.show_modal = true;
            } else {
                start(state, ctx);
            }
        }
        Some(CardAct::Clear) => clear(state),
        Some(CardAct::Browse) => browse_folder(state),
        Some(CardAct::Cancel) => cancel_task(state, ctx),
        Some(CardAct::View) => state.media.dl.show_modal = true,
        None => {}
    }
}

/// `.nd-label`: 12/600 muted + 14px icon (opacity .7), 5px gaps; optional
/// `(… )` suffix in 11/400 dim.
fn label_row(ui: &mut Ui, p: &Palette, icon: &str, text: &str, optional: Option<&str>) -> Rect {
    let font = semibold(12.0);
    // `.nd-label` is a flex row whose 12px line box measures 16px in the
    // reference build (17.5px per the button's 1.5 line box is NOT used here).
    let h = 16.0;
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let x = r.min.x + 16.0;
    let cy = r.center().y;
    mi::paint_centered(
        ui.painter(),
        pos2(x + 7.0, cy),
        icon,
        14.0,
        p.text_muted.gamma_multiply(0.7),
    );
    let t = draw_text(
        ui,
        pos2(x + 19.0, cy),
        Align2::LEFT_CENTER,
        text,
        font,
        p.text_muted,
    );
    if let Some(opt) = optional {
        draw_text(
            ui,
            pos2(t.max.x + 5.0, cy),
            Align2::LEFT_CENTER,
            opt,
            regular(11.0),
            p.text_dim,
        );
    }
    r
}

/// `.nd-tabs.media-browser-tabs`: 32px segmented control in a 4px-padded
/// `--hover-bg` track (radius 10) with the sliding `.nd-tab-indicator`.
fn browser_tabs(ui: &mut Ui, p: &Palette, dl: &mut DlUi) {
    let r = row(ui, 40.0);
    let rect = inset(r, 16.0);
    let idx = match dl.browser {
        Browser::Firefox => 0.0_f32,
        Browser::Chrome => 1.0_f32,
    };
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new("media-browser-tab"),
        idx,
        0.22,
    );
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(10),
        p.hover_veil, // `background: var(--hover-bg)`
    );

    // Indicator: `left: 4px; width: calc(50% - 6px); height: 32px; radius 7`.
    let track_w = rect.width();
    let ind_w = track_w / 2.0 - 6.0;
    let x0 = rect.min.x + 4.0;
    let x1 = rect.min.x + track_w / 2.0 + 2.0;
    let ix = x0 + (x1 - x0) * t;
    let ind = Rect::from_min_size(pos2(ix, rect.min.y + 4.0), vec2(ind_w, 32.0));
    // `box-shadow: 0 1px 3px rgba(0,0,0,.18)`
    ui.painter().rect_filled(
        ind.translate(vec2(0.0, 1.0)),
        CornerRadius::same(7),
        Color32::from_black_alpha(46),
    );
    ui.painter()
        .rect_filled(ind, CornerRadius::same(7), p.bg_card);
    ui.painter().rect_stroke(
        ind,
        CornerRadius::same(7),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    let half = (track_w - 8.0) / 2.0;
    // The Vue source renders 🦊 / 🌐 emoji; the bundled fonts have no emoji
    // coverage (they came out as tofu boxes), so use Material stand-ins.
    for (i, (icon, label)) in [
        ("local_fire_department", "Firefox"),
        ("public", "Chrome"),
    ]
    .into_iter()
    .enumerate()
    {
        // `.nd-tab { flex: 1 }` with a 4px gap and 4px track padding.
        let cell = Rect::from_min_size(
            pos2(rect.min.x + 4.0 + i as f32 * (half + 2.0), rect.min.y + 4.0),
            vec2(half - 2.0, 32.0),
        );
        let rr = ui.interact(cell, egui::Id::new(("media-browser", i)), Sense::click());
        let active = (i == 0) == matches!(dl.browser, Browser::Firefox);
        if rr.hovered() && !active {
            // `.nd-tab:hover:not(.active) { background: rgba(255,255,255,.04) }`
            ui.painter().rect_filled(
                cell,
                CornerRadius::same(7),
                mk::blend(p.bg_card, Color32::WHITE, 0.04),
            );
        }
        let color = if active {
            p.accent_strong
        } else if rr.hovered() {
            p.text_muted
        } else {
            p.text_dim
        };
        let lf = semibold(12.0);
        let ew = 14.0; // `.browser-emoji { font-size: 13px }` → 14px icon
        let lw = text_width(ui, label, lf.clone());
        let start = cell.center().x - (ew + 6.0 + lw) / 2.0;
        mi::paint_centered(
            ui.painter(),
            pos2(start + ew / 2.0, cell.center().y),
            icon,
            ew,
            color,
        );
        draw_text(
            ui,
            pos2(start + ew + 6.0, cell.center().y),
            Align2::LEFT_CENTER,
            label,
            lf,
            color,
        );
        if rr.clicked() {
            dl.browser = if i == 0 {
                Browser::Firefox
            } else {
                Browser::Chrome
            };
        }
    }
}

/// `.media-resume-btn`: 32px tall, 7/12 padding, 8r, 11.5px/700.
fn small_button(
    ui: &mut Ui,
    id: egui::Id,
    rect: Rect,
    icon: &str,
    label: &str,
    kind: BtnKind,
) -> bool {
    let p = mk::pal();
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = resp.hovered();
    match kind {
        BtnKind::Secondary => {
            let fill = if hovered {
                mk::blend(p.bg_card, p.accent, 0.08)
            } else {
                mk::blend(p.bg_card, Color32::WHITE, 0.03)
            };
            ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(
                    1.0,
                    if hovered {
                        mk::blend(p.bg_card, p.accent, 0.2)
                    } else {
                        p.border
                    },
                ),
                StrokeKind::Inside,
            );
        }
        BtnKind::Danger => {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(8),
                mk::blend(p.bg_card, p.status_error, if hovered { 0.16 } else { 0.08 }),
            );
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(
                    1.0,
                    mk::blend(p.bg_card, p.status_error, if hovered { 0.32 } else { 0.2 }),
                ),
                StrokeKind::Inside,
            );
        }
        BtnKind::Primary => {}
    }
    let color = match kind {
        BtnKind::Danger => {
            if hovered {
                Color32::from_rgb(0xff, 0x8a, 0x80)
            } else {
                p.status_error
            }
        }
        _ => {
            if hovered {
                p.accent_strong
            } else {
                p.text_muted
            }
        }
    };
    let font = bold(11.5);
    let tw = text_width(ui, label, font.clone());
    let mut x = rect.center().x - (16.0 + 6.0 + tw) / 2.0;
    let cy = rect.center().y;
    mi::paint_centered(ui.painter(), pos2(x + 8.0, cy), icon, 16.0, color);
    x += 22.0;
    draw_text(ui, pos2(x, cy), Align2::LEFT_CENTER, label, font, color);
    resp.clicked()
}

// ── Modal ───────────────────────────────────────────────────────────────

const MODAL_W: f32 = 480.0; // `.nd-modal { max-width: 480px }`
const MODAL_HEADER: f32 = 61.0; // 14 + 36 + 10 + 1px border

#[derive(Clone, Copy, PartialEq)]
enum ModalAct {
    Minimize,
    Cancel,
    Download,
    Retry,
}

/// `.nd-overlay` + `.nd-modal` — steps, format picker, log box and footer.
pub fn modal(state: &mut UiState, ctx: &Ctx) {
    if !state.media.dl.show_modal {
        return;
    }
    let Some(gctx) = state.media.dl.egui_ctx.clone() else {
        return;
    };
    let p = mk::pal();
    let screen = gctx.content_rect();
    let mut act: Option<ModalAct> = None;
    let mut overlay_clicked = false;
    let mut modal_rect = Rect::NOTHING;

    // `.nd-overlay`: fixed inset 0, rgba(0,0,0,.55), click-outside closes.
    egui::Area::new(egui::Id::new("media-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(&gctx, |ui| {
            ui.painter().rect_filled(
                screen,
                CornerRadius::same(0),
                Color32::from_black_alpha(140),
            );
            // `@mousedown.self` / `@mouseup.self`: only a press *and* release
            // on the backdrop itself closes; the modal panel absorbs its own.
            overlay_clicked = ui.allocate_rect(screen, Sense::click()).clicked();
        });

    if gctx.input(|i| i.key_pressed(Key::Escape)) {
        act = Some(ModalAct::Minimize);
    }

    // Hoisted: the body closure borrows `state.media.dl` mutably.
    let lang = state.settings.language.clone();
    let footer_h = 1.0 + 10.0 + 38.0 + 14.0; // border + 10/14 padding + buttons
    let body_max = (screen.height() * 0.85 - MODAL_HEADER - 1.0 - footer_h - 2.0).max(120.0);
    // `.nd-overlay { padding: 24px }` + `.nd-modal { max-width: 480px; width: 100% }`.
    let modal_w = (MODAL_W - 2.0).min(screen.width() - 48.0);

    egui::Area::new(egui::Id::new("media-modal"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(&gctx, |ui| {
            let panel = egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.border_strong))
                .inner_margin(egui::Margin::same(1))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 32,
                    spread: 0,
                    color: Color32::from_black_alpha(115),
                })
                .show(ui, |ui| {
                    ui.set_width(modal_w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let dl = &mut state.media.dl;

                    // `.nd-glow`: radial accent wash at the top edge.
                    let mr = ui.max_rect();
                    glow(ui, mr, p.accent);

                    modal_header(ui, &p, &mut act, &lang);
                    let d = row(ui, 1.0);
                    hgrad_line(
                        ui,
                        Rect::from_min_max(
                            pos2(d.min.x + 16.0, d.min.y),
                            pos2(d.max.x - 16.0, d.max.y),
                        ),
                        p.border_strong,
                    );

                    // `.nd-body`: padding 12 16 8, gap 12, scrolls.
                    {
                        let s = &mut ui.spacing_mut().scroll;
                        s.bar_width = 4.0;
                        s.floating = false;
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("media-modal-body")
                        .auto_shrink([false, true])
                        .max_height(body_max)
                        .show(ui, |ui| {
                            // The bar (4px, `::-webkit-scrollbar`) eats into
                            // the body width only while it is shown.
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            ui.add_space(12.0);
                            modal_body(ui, dl, &p, &mut act, &lang);
                            ui.add_space(8.0);
                        });

                    modal_footer(ui, dl, &mut act, &lang);
                });
            modal_rect = panel.response.rect;
        });

    // Clicks that land on the modal panel (padding, glow, gaps) must not be
    // seen as backdrop clicks — only outside the panel minimizes.
    let pointer = gctx.input(|i| i.pointer.interact_pos());
    if overlay_clicked && pointer.map_or(true, |pt| !modal_rect.contains(pt)) {
        act = Some(ModalAct::Minimize);
    }

    if gctx.input(|i| i.key_pressed(Key::Escape)) {
        act = Some(ModalAct::Minimize);
    }

    match act {
        Some(ModalAct::Minimize) => state.media.dl.show_modal = false,
        Some(ModalAct::Cancel) => cancel_task(state, ctx),
        Some(ModalAct::Download) => download_with_format(state, ctx),
        Some(ModalAct::Retry) => start(state, ctx),
        None => {}
    }
}

/// `.nd-glow` (top -60px, 220×130, accent .18 → transparent 70%, blur 20).
fn glow(ui: &Ui, modal: Rect, color: Color32) {
    let center = pos2(modal.center().x, modal.min.y + 5.0);
    let (rx, ry) = (110.0_f32, 65.0_f32);
    let clip = ui.painter().clip_rect();
    let painter = ui.painter().with_clip_rect(clip.intersect(modal));
    let steps = 28;
    for i in 0..steps {
        let t = (i + 1) as f32 / steps as f32;
        let a = 0.012 * (1.0 - t);
        let mut pts = Vec::with_capacity(24);
        for k in 0..24 {
            let ang = k as f32 / 24.0 * std::f32::consts::TAU;
            pts.push(center + vec2(rx * t * ang.cos(), ry * t * ang.sin()));
        }
        painter.add(egui::Shape::convex_polygon(
            pts,
            color.gamma_multiply(a),
            Stroke::NONE,
        ));
    }
}

fn modal_header(ui: &mut Ui, p: &Palette, act: &mut Option<ModalAct>, lang: &str) {
    let (h, _) = ui.allocate_exact_size(vec2(ui.available_width(), MODAL_HEADER), Sense::hover());
    let avatar = Rect::from_min_size(pos2(h.min.x + 16.0, h.min.y + 14.0), vec2(36.0, 36.0));
    super::grad135(
        ui,
        avatar,
        10,
        mk::blend(p.bg_card, p.accent, 0.12),
        mk::blend(p.bg_card, p.accent, 0.04),
    );
    ui.painter().rect_stroke(
        avatar,
        CornerRadius::same(10),
        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.12)),
        StrokeKind::Inside,
    );
    mi::paint_centered(ui.painter(), avatar.center(), "play_circle", 18.0, p.accent_strong);
    let tx = avatar.max.x + 10.0;
    let title_font = bold(14.0);
    let sub_font = regular(11.0);
    let title_h = ui.fonts_mut(|f| f.row_height(&title_font)).max(17.0);
    let sub_h = ui.fonts_mut(|f| f.row_height(&sub_font));
    let block = title_h + 2.0 + sub_h;
    let top = h.min.y + 14.0 + (36.0 - block) / 2.0;
    draw_text(
        ui,
        pos2(tx, top + title_h / 2.0),
        Align2::LEFT_CENTER,
        crate::i18n::t(lang, "Media Download"),
        title_font,
        p.text,
    );
    draw_text(
        ui,
        pos2(tx, top + title_h + 2.0 + sub_h / 2.0),
        Align2::LEFT_CENTER,
        crate::i18n::t(lang, "Paste a YouTube (or supported) URL to download."),
        sub_font,
        p.text_muted,
    );

    // `.nd-icon-btn.nd-close`: 28×28 r7, minimize 16, red hover.
    let close = Rect::from_min_size(
        pos2(h.max.x - 16.0 - 28.0, h.min.y + 18.0),
        vec2(28.0, 28.0),
    );
    let rr = ui.interact(close, egui::Id::new("media-modal-min"), Sense::click());
    if rr.hovered() {
        // `.nd-close:hover { background: rgba(239,83,80,.1); color: status-error }`
        ui.painter().rect_filled(
            close,
            CornerRadius::same(7),
            Color32::from_rgba_unmultiplied(239, 83, 80, 26),
        );
    }
    mi::paint_centered(
        ui.painter(),
        close.center(),
        "minimize",
        16.0,
        if rr.hovered() {
            p.status_error
        } else {
            p.text_dim
        },
    );
    if rr.clicked() {
        *act = Some(ModalAct::Minimize);
    }
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(h.min.x, h.max.y), vec2(h.width(), 1.0)),
        CornerRadius::same(0),
        p.border,
    );
}

/// `.nd-body` contents. `gap: 12px` between every child.
fn modal_body(ui: &mut Ui, dl: &mut DlUi, p: &Palette, act: &mut Option<ModalAct>, lang: &str) {
    // `.media-modal-url`: mono 11, padding 8 10, radius 8,
    // `word-break: break-all`.
    if !dl.dialed_url.is_empty() {
        let f = FontId::monospace(11.0);
        let inner_w = ui.available_width() - 32.0 - 20.0;
        let galley = {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = inner_w;
            job.wrap.break_anywhere = true;
            job.append(
                &dl.dialed_url,
                0.0,
                egui::text::TextFormat {
                    font_id: f,
                    color: p.text_dim,
                    ..Default::default()
                },
            );
            ui.painter().layout_job(job)
        };
        let bh = galley.size().y + 16.0 + 2.0;
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), bh), Sense::hover());
        let boxr = inset(r, 16.0);
        ui.painter().rect_filled(
            boxr,
            CornerRadius::same(8),
            mk::blend(p.bg_card, Color32::WHITE, 0.02),
        );
        ui.painter().rect_stroke(
            boxr,
            CornerRadius::same(8),
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
        ui.painter()
            .galley(pos2(boxr.min.x + 10.0, boxr.min.y + 8.0), galley, p.text_dim);
        ui.add_space(12.0);
    }

    // Steps row (hidden in the error state).
    if dl.state != DlState::Error {
        steps_row(ui, dl, p, lang);
        ui.add_space(12.0);
    }

    match dl.state {
        DlState::Scanning => center_block(ui, p, crate::i18n::t(lang, "Scanning available qualities..."), false),
        DlState::Selecting => {
            format_select(ui, dl, p, lang);
        }
        DlState::Downloading => center_block(ui, p, crate::i18n::t(lang, "Starting download..."), false),
        DlState::Running => center_block(ui, p, crate::i18n::t(lang, "Download started!"), true),
        DlState::Error => error_box(ui, p, &dl.error, lang),
        DlState::Idle => {}
    }
    ui.add_space(12.0);

    // Log section.
    log_section(ui, dl, p, lang);

    let _ = act;
}

/// `.media-steps`: Scan → Select → Download (8px dots, 36px lines).
fn steps_row(ui: &mut Ui, dl: &DlUi, p: &Palette, lang: &str) {
    let r = row(ui, 18.0);
    let labels = [
            crate::i18n::t(lang, "Scan"),
            crate::i18n::t(lang, "Select"),
            crate::i18n::t(lang, "Download"),
        ];
    let stage = match dl.state {
        DlState::Selecting => 1,
        DlState::Downloading | DlState::Running => 2,
        _ => 0,
    };
    let f = semibold(11.5);
    let widths: Vec<f32> = labels
        .iter()
        .map(|l| 8.0 + 6.0 + text_width(ui, l, f.clone()))
        .collect();
    let total = widths.iter().sum::<f32>() + 2.0 * (36.0 + 16.0);
    let mut x = r.center().x - total / 2.0;
    let cy = r.center().y;
    for (i, label) in labels.iter().enumerate() {
        let active = i == stage;
        let done = i < stage;
        let color = if active || done {
            p.accent_strong.gamma_multiply(if active { 1.0 } else { 0.6 })
        } else {
            p.text_dim
        };
        if active {
            // `box-shadow: 0 0 8px rgba(accent,.4)`
            ui.painter()
                .circle_filled(pos2(x + 4.0, cy), 7.0, mk::blend(p.bg_card, p.accent, 0.12));
        }
        // `.media-step-dot`: 8px, `border: 2px solid` (border-box).
        let center = pos2(x + 4.0, cy);
        if active || done {
            ui.painter().circle_filled(center, 4.0, p.accent_strong);
            ui.painter().circle_filled(center, 2.0, p.accent);
        } else {
            ui.painter().circle_filled(center, 4.0, p.border_strong);
        }
        draw_text(
            ui,
            pos2(x + 14.0, cy),
            Align2::LEFT_CENTER,
            label,
            f.clone(),
            color,
        );
        x += widths[i];
        if i < 2 {
            let line = Rect::from_min_size(pos2(x + 8.0, cy - 1.0), vec2(36.0, 2.0));
            ui.painter().rect_filled(
                line,
                CornerRadius::same(1),
                if i < stage { p.accent } else { p.border_strong },
            );
            x += 36.0 + 16.0;
        }
    }
}

/// `.media-center` — 30px spinner (or 40px check) + 13px status text.
fn center_block(ui: &mut Ui, p: &Palette, text: &str, done: bool) {
    // `padding: 6px 0 2px; gap: 10px` + a 13px/1.2 text line.
    let h = if done { 74.0 } else { 64.0 };
    let r = row(ui, h);
    let cx = r.center().x;
    if done {
        // `.media-check`: 40px circle, status-complete @ .12, check 24.
        let cy = r.min.y + 6.0 + 20.0;
        ui.painter().circle_filled(
            pos2(cx, cy),
            20.0,
            mk::blend(p.bg_card, p.status_complete, 0.12),
        );
        mi::paint_centered(ui.painter(), pos2(cx, cy), "check", 24.0, p.status_complete);
        draw_text(
            ui,
            pos2(cx, r.min.y + 6.0 + 40.0 + 10.0 + 7.8),
            Align2::LEFT_CENTER,
            text,
            regular(13.0),
            p.text_dim,
        );
    } else {
        spinner(ui, pos2(cx, r.min.y + 6.0 + 15.0), 30.0, 3.0);
        draw_text(
            ui,
            pos2(cx, r.min.y + 6.0 + 30.0 + 10.0 + 7.8),
            Align2::LEFT_CENTER,
            text,
            regular(13.0),
            p.text_dim,
        );
    }
}

/// `.media-error-box` + `.media-error-pre`.
fn error_box(ui: &mut Ui, p: &Palette, error: &str, lang: &str) {
    let inner_w = ui.available_width() - 32.0 - 24.0 - 20.0;
    let pre_font = FontId::monospace(11.0);
    let pre_galley = ui.painter().layout(error.to_string(), pre_font, p.status_error, inner_w);
    let pre_h = (pre_galley.size().y + 16.0).min(120.0);
    let bh = 12.0 + 18.0 + 8.0 + pre_h + 12.0 + 2.0;
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), bh), Sense::hover());
    let boxr = inset(r, 16.0);
    ui.painter().rect_filled(
        boxr,
        CornerRadius::same(10),
        mk::blend(p.bg_card, p.status_error, 0.05),
    );
    ui.painter().rect_stroke(
        boxr,
        CornerRadius::same(10),
        Stroke::new(1.0, mk::blend(p.bg_card, p.status_error, 0.18)),
        StrokeKind::Inside,
    );
    // `.media-error-header`: icon 18 + 8 gap + 12.5/700 red, 8px below.
    mi::paint_centered(
        ui.painter(),
        pos2(boxr.min.x + 12.0 + 9.0, boxr.min.y + 12.0 + 9.0),
        "error",
        18.0,
        p.status_error,
    );
    draw_text(
        ui,
        pos2(boxr.min.x + 12.0 + 18.0 + 8.0, boxr.min.y + 12.0 + 9.0),
        Align2::LEFT_CENTER,
        crate::i18n::t(lang, "Download failed"),
        bold(12.5),
        p.status_error,
    );
    let pre = Rect::from_min_size(
        pos2(boxr.min.x + 12.0, boxr.min.y + 12.0 + 18.0 + 8.0),
        vec2(boxr.width() - 24.0, pre_h),
    );
    ui.painter().rect_filled(
        pre,
        CornerRadius::same(8),
        mk::blend(p.bg_card, p.status_error, 0.06),
    );
    let painter = ui.painter().with_clip_rect(pre);
    painter.galley(pos2(pre.min.x + 10.0, pre.min.y + 8.0), pre_galley, p.status_error);
}

/// `.media-format-select` — section header, merger hint and the format list.
fn format_select(ui: &mut Ui, dl: &mut DlUi, p: &Palette, lang: &str) {
    // `.nd-section-header` row (title 10/700 + merger pill).
    let r = row(ui, 20.0);
    let title_font = bold(10.0);
    draw_text_spaced(
        ui,
        pos2(r.min.x + 16.0, r.center().y),
        Align2::LEFT_CENTER,
        &crate::i18n::t(lang, "Available qualities").to_uppercase(),
        title_font,
        p.text_dim.gamma_multiply(0.7),
        0.05 * 10.0,
    );
    if !dl.merger_available {
        let font = semibold(10.0);
        let label = crate::i18n::t(lang, "Merger off — high-res hidden");
        let tw = text_width(ui, label, font.clone());
        let w = 9.0 + 12.0 + 5.0 + tw + 9.0 + 2.0; // padding + icon/gap + border
        let pill = Rect::from_min_size(
            pos2(r.max.x - 16.0 - w, r.center().y - 10.0),
            vec2(w, 20.0),
        );
        let orange = Color32::from_rgb(0xff, 0xb7, 0x4d);
        ui.painter().rect_filled(
            pill,
            CornerRadius::same(100),
            mk::blend(p.bg_card, Color32::from_rgb(0xff, 0x98, 0x00), 0.08),
        );
        ui.painter().rect_stroke(
            pill,
            CornerRadius::same(100),
            Stroke::new(
                1.0,
                mk::blend(p.bg_card, Color32::from_rgb(0xff, 0x98, 0x00), 0.2),
            ),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(pill.min.x + 9.0 + 6.0, pill.center().y),
            "info",
            12.0,
            orange,
        );
        draw_text(
            ui,
            pos2(pill.min.x + 9.0 + 12.0 + 5.0, pill.center().y),
            Align2::LEFT_CENTER,
            label,
            font,
            orange,
        );
    }
    ui.add_space(8.0);

    // `.media-format-list`: border, radius 10, bg white .015, padding 4.
    let formats: Vec<YtFormat> = dl.visible_formats().into_iter().cloned().collect();
    // Item box: `padding: 9px 10px`, `border: 1px` (border-box), a 12.5px
    // label line and an optional 10.5px size line (`margin-top: 2px`).
    let label_font = regular(12.5);
    let size_font = regular(10.5);
    let label_h = ui.fonts_mut(|f| f.row_height(&label_font));
    let size_h = ui.fonts_mut(|f| f.row_height(&size_font));
    let has_size = |f: &YtFormat| f.filesize.unwrap_or(0) > 0;
    let item_h = move |f: &YtFormat| {
        let mut h = 9.0 + 9.0 + label_h + 2.0; // + 1px border top/bottom
        if has_size(f) {
            h += 2.0 + size_h;
        }
        h
    };
    let content_h: f32 = formats
        .iter()
        .map(|f| item_h(f))
        .sum::<f32>()
        + 2.0 * (formats.len().saturating_sub(1) as f32);
    let list_outer = (content_h + 8.0 + 2.0).min(210.0).max(20.0);
    let (lr, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), list_outer), Sense::hover());
    let list = inset(lr, 16.0);
    ui.painter().rect_filled(
        list,
        CornerRadius::same(10),
        mk::blend(p.bg_card, Color32::WHITE, 0.015),
    );
    ui.painter().rect_stroke(
        list,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    let mut clicked: Option<String> = None;
    let inner = Rect::from_min_max(list.min + vec2(5.0, 5.0), list.max - vec2(5.0, 5.0));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_width(inner.width());
    child.spacing_mut().item_spacing = vec2(0.0, 0.0);
    {
        let s = &mut child.spacing_mut().scroll;
        s.bar_width = 4.0;
        s.floating = false;
    }
    egui::ScrollArea::vertical()
        .id_salt("media-format-list")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for (i, f) in formats.iter().enumerate() {
                if i > 0 {
                    ui.add_space(2.0);
                }
                let w = ui.available_width();
                let (ir, _) = ui.allocate_exact_size(vec2(w, item_h(f)), Sense::hover());
                let selected = dl.selected_format.as_deref() == Some(f.format_id.as_str());
                let resp = ui.interact(ir, egui::Id::new(("media-fmt", i)), Sense::click());
                if selected {
                    ui.painter().rect_filled(
                        ir,
                        CornerRadius::same(8),
                        mk::blend(p.bg_card, p.accent, 0.08),
                    );
                    ui.painter().rect_stroke(
                        ir,
                        CornerRadius::same(8),
                        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.18)),
                        StrokeKind::Inside,
                    );
                } else if resp.hovered() {
                    ui.painter().rect_filled(
                        ir,
                        CornerRadius::same(8),
                        mk::blend(p.bg_card, Color32::WHITE, 0.04),
                    );
                }
                // Radio: `accent-color: accent-strong` (1px border + 10px pad).
                let rc = pos2(ir.min.x + 1.0 + 10.0 + 6.5, ir.center().y);
                ui.painter().circle_stroke(
                    rc,
                    6.0,
                    Stroke::new(
                        1.5,
                        if selected {
                            p.accent_strong
                        } else {
                            mk::blend(p.bg_card, p.text_dim, 0.7)
                        },
                    ),
                );
                if selected {
                    ui.painter().circle_filled(rc, 3.0, p.accent_strong);
                }

                // Label + optional size line.
                let tx = ir.min.x + 1.0 + 10.0 + 13.0 + 10.0;
                let text_color = if selected { p.text } else { p.text_muted };
                let label = if f.label.is_empty() {
                    f.resolution.clone()
                } else {
                    f.label.clone()
                };
                let label_cy = ir.min.y + 1.0 + 9.0 + label_h / 2.0;
                if has_size(f) {
                    draw_text(
                        ui,
                        pos2(tx, label_cy),
                        Align2::LEFT_CENTER,
                        &label,
                        label_font.clone(),
                        text_color,
                    );
                    draw_text(
                        ui,
                        pos2(tx, label_cy + label_h / 2.0 + 2.0 + size_h / 2.0),
                        Align2::LEFT_CENTER,
                        &moonhunt_core::util::format_bytes(f.filesize.unwrap_or(0)),
                        size_font.clone(),
                        p.text_dim,
                    );
                } else {
                    draw_text(
                        ui,
                        pos2(tx, ir.center().y),
                        Align2::LEFT_CENTER,
                        &label,
                        label_font.clone(),
                        text_color,
                    );
                }

                // `.media-format-tag`: 9/700 uppercase, padding 2 6, 100r.
                let (tag, merger) = if f.requires_merger {
                    ("merger", true)
                } else {
                    ("direct", false)
                };
                let tag_font = bold(9.0);
                let tw = text_width(ui, tag, tag_font.clone());
                // padding 2px 6px + 1px border → 17px tall (border-box).
                let tag_h = (ui.fonts_mut(|f| f.row_height(&tag_font)) + 4.0 + 2.0).round();
                let pill = Rect::from_min_size(
                    pos2(
                        ir.max.x - 1.0 - 10.0 - (6.0 + tw + 6.0),
                        ir.center().y - tag_h / 2.0,
                    ),
                    vec2(6.0 + tw + 6.0, tag_h),
                );
                if merger {
                    ui.painter().rect_filled(
                        pill,
                        CornerRadius::same(100),
                        mk::blend(p.bg_card, p.accent, 0.10),
                    );
                    ui.painter().rect_stroke(
                        pill,
                        CornerRadius::same(100),
                        Stroke::new(1.0, mk::blend(p.bg_card, p.accent, 0.25)),
                        StrokeKind::Inside,
                    );
                } else {
                    ui.painter().rect_filled(
                        pill,
                        CornerRadius::same(100),
                        p.hover_veil, // `background: var(--hover-bg)`
                    );
                    ui.painter().rect_stroke(
                        pill,
                        CornerRadius::same(100),
                        Stroke::new(1.0, p.border),
                        StrokeKind::Inside,
                    );
                }
                draw_text_spaced(
                    ui,
                    pos2(pill.min.x + 6.0, pill.center().y),
                    Align2::LEFT_CENTER,
                    tag,
                    tag_font,
                    if merger { p.accent_strong } else { p.text_dim },
                    0.04 * 9.0,
                );

                if resp.clicked() {
                    clicked = Some(f.format_id.clone());
                }
            }
        });
    if let Some(fid) = clicked {
        dl.selected_format = Some(fid);
    }
}

/// Lay out monospace text with an explicit CSS `line-height` (egui's default
/// row height is tighter than the web reference).
fn mono_galley(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    wrap: f32,
    line_h: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap;
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id: font,
            color,
            line_height: Some(line_h),
            ..Default::default()
        },
    );
    ui.painter().layout_job(job)
}

/// `.media-log-section` — header + `pre` box (4px accent scrollbar, sticks to
/// the bottom as lines arrive).
fn log_section(ui: &mut Ui, dl: &mut DlUi, p: &Palette, lang: &str) {
    let font = FontId::monospace(10.5);
    let inner_w = ui.available_width() - 32.0 - 20.0;
    let text = dl.log.join("\n");
    let galley = if text.is_empty() {
        None
    } else {
        // `.media-log-box { line-height: 1.5 }` → 15.75px rows.
        Some(mono_galley(ui, &text, font.clone(), p.text_dim, inner_w, 15.75))
    };
    let body_h = galley
        .as_ref()
        .map(|g| (g.size().y + 16.0).min(150.0))
        .unwrap_or(32.0);
    let header_h = 26.0; // 7/7 padding + 12px title line (border drawn below)
    let total = header_h + body_h + 3.0; // + header border + section border-box
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
    let sect = inset(r, 16.0);
    ui.painter().rect_filled(
        sect,
        CornerRadius::same(10),
        mk::blend(p.bg_card, Color32::WHITE, 0.015),
    );
    ui.painter().rect_stroke(
        sect,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let head = Rect::from_min_size(sect.min + vec2(1.0, 1.0), vec2(sect.width() - 2.0, header_h));
    ui.painter().rect_filled(
        head,
        CornerRadius::same(0),
        mk::blend(p.bg_card, Color32::WHITE, 0.02),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(head.min.x, head.max.y), vec2(head.width(), 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    draw_text_spaced(
        ui,
        pos2(head.min.x + 10.0, head.center().y),
        Align2::LEFT_CENTER,
        "LOG",
        bold(10.0),
        p.text_dim.gamma_multiply(0.7),
        0.05 * 10.0,
    );
    if !dl.log.is_empty() {
        let cf = regular(10.5);
        let tw = text_width(ui, crate::i18n::t(lang, "clear"), cf.clone());
        let cr = Rect::from_min_size(
            pos2(head.max.x - 10.0 - tw, head.min.y),
            vec2(tw, head.height()),
        );
        let rr = ui.interact(cr, egui::Id::new("media-log-clear"), Sense::click());
        draw_text(
            ui,
            pos2(cr.min.x, cr.center().y),
            Align2::LEFT_CENTER,
            crate::i18n::t(lang, "clear"),
            cf,
            if rr.hovered() { p.text } else { p.text_dim },
        );
        if rr.clicked() {
            dl.log.clear();
        }
    }

    // `.media-log-box`: padding 8 10, max-height 150, mono 10.5/1.5.
    let body = Rect::from_min_max(
        pos2(sect.min.x + 1.0, head.max.y + 1.0),
        pos2(sect.max.x - 1.0, sect.max.y - 1.0),
    );
    if let Some(galley) = galley {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body.shrink2(vec2(10.0, 8.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_width(body.width() - 20.0);
        {
            let s = &mut child.spacing_mut().scroll;
            s.bar_width = 4.0;
            s.floating = false;
        }
        egui::ScrollArea::vertical()
            .id_salt("media-log-box")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(&mut child, |ui| {
                let (gr, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                ui.painter().galley(gr.min, galley.clone(), p.text_dim);
            });
    } else {
        draw_text(
            ui,
            pos2(body.min.x + 10.0, body.min.y + 8.0 + 8.0),
            Align2::LEFT_CENTER,
            crate::i18n::t(lang, "Waiting for output..."),
            regular(10.5),
            p.text_dim,
        );
    }
}

/// `.nd-footer`: border-top, padding 10 16 14, 8px gap, equal-width buttons.
fn modal_footer(ui: &mut Ui, dl: &DlUi, act: &mut Option<ModalAct>, lang: &str) {
    let p = mk::pal();
    let ft = row(ui, 1.0);
    ui.painter().rect_filled(
        Rect::from_min_size(ft.min, vec2(ft.width(), 1.0)),
        CornerRadius::same(0),
        p.border,
    );
    ui.add_space(10.0);
    let h = 38.0;
    let r = row(ui, h);
    let mut defs: Vec<(&str, Option<&'static str>, BtnKind, ModalAct, bool)> = vec![(
        crate::i18n::t(lang, "Minimize"),
        Some("minimize"),
        BtnKind::Secondary,
        ModalAct::Minimize,
        true,
    )];
    if matches!(
        dl.state,
        DlState::Scanning | DlState::Selecting | DlState::Downloading | DlState::Error
    ) {
        defs.push((
            crate::i18n::t(lang, "Cancel"),
            Some("close"),
            BtnKind::Danger,
            ModalAct::Cancel,
            true,
        ));
    }
    if dl.state == DlState::Selecting {
        defs.push((
            crate::i18n::t(lang, "Download"),
            Some("download"),
            BtnKind::Primary,
            ModalAct::Download,
            dl.selected_format.is_some(),
        ));
    }
    if dl.state == DlState::Error {
        defs.push((
            crate::i18n::t(lang, "Retry"),
            Some("refresh"),
            BtnKind::Primary,
            ModalAct::Retry,
            true,
        ));
    }
    let n = defs.len() as f32;
    let gap = 8.0;
    let inner_w = r.width() - 32.0;
    let w = ((inner_w - gap * (n - 1.0)) / n).max(40.0);
    let mut x = r.min.x + 16.0;
    for (i, (label, icon, kind, action, enabled)) in defs.iter().enumerate() {
        let b = Rect::from_min_size(pos2(x, r.min.y), vec2(w, h));
        if nd_button(
            ui,
            egui::Id::new(("media-modal-btn", i)),
            b,
            *icon,
            label,
            *kind,
            *enabled,
        ) {
            *act = Some(*action);
        }
        x += w + gap;
    }
    ui.add_space(14.0);
}

// ── Behaviour (Media.vue script) ────────────────────────────────────────

/// `browseFolder()` — native folder picker, seeds `C:\Downloads`.
fn browse_folder(state: &mut UiState) {
    let mut dlg = rfd::FileDialog::new().set_title("Select Save Folder");
    let seed = if state.media.dl.path.trim().is_empty() {
        "C:\\Downloads"
    } else {
        state.media.dl.path.trim()
    };
    dlg = dlg.set_directory(seed);
    if let Some(dir) = dlg.pick_folder() {
        state.media.dl.path = dir.display().to_string();
    }
}

/// `start()` — validate, open the modal in `scanning` and run the scan.
fn start(state: &mut UiState, ctx: &Ctx) {
    let url = state.media.dl.url.trim().to_string();
    if url.is_empty() {
        state.toast(false, "Please enter a URL");
        return;
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        state.toast(false, "Please enter a valid URL");
        return;
    }
    let browser = state.media.dl.browser.key().to_string();
    let scan_id = {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        format!("media-scan-{}-{}", crate::state::chrono_now_ms(), nanos)
    };
    {
        let dl = &mut state.media.dl;
        dl.task_id += 1;
        dl.show_modal = true;
        dl.state = DlState::Scanning;
        dl.error.clear();
        dl.dialed_url = url.clone();
        dl.formats.clear();
        dl.selected_format = None;
        dl.log.clear();
        dl.started = false;
        dl.is_starting = true;
        dl.active_scan_id = Some(scan_id.clone());
    }
    let sid = scan_id.clone();
    ctx.emit("media-scan", move |app| async move {
        let res = moonhunt_core::ytdlp::scan_yt_formats(
            app,
            url,
            Some(browser),
            Some(sid.clone()),
        )
        .await;
        Ok(match res {
            Ok(v) => serde_json::json!({"scanId": sid, "result": v}),
            Err(e) => serde_json::json!({"scanId": sid, "error": e}),
        })
    });
}

/// `downloadWithFormat()` — mp4 for video picks, mp3 for audio-only picks.
fn download_with_format(state: &mut UiState, ctx: &Ctx) {
    let Some(fid) = state.media.dl.selected_format.clone() else {
        state.toast(false, "Please select a quality");
        return;
    };
    let sel = state
        .media
        .dl
        .formats
        .iter()
        .find(|f| f.format_id == fid)
        .cloned();
    let fmt = match &sel {
        None => "mp4",
        Some(s) => {
            if s.has_video {
                "mp4"
            } else {
                "mp3"
            }
        }
    };
    let path = state.media.dl.path.trim().to_string();
    let browser = state.media.dl.browser.key().to_string();
    let url = state.media.dl.dialed_url.clone();
    let (has_video, has_audio, size) = match &sel {
        Some(s) => (Some(s.has_video), Some(s.has_audio), s.filesize),
        None => (None, None, None),
    };
    let label = sel
        .as_ref()
        .map(|s| s.label.clone())
        .unwrap_or_else(|| fid.clone());
    {
        let dl = &mut state.media.dl;
        dl.task_id += 1;
        dl.state = DlState::Downloading;
        dl.log.clear();
        dl.started = false;
        let mut pick = format!(
            "[pick] {} | format_id={} | container={} | video={} audio={}",
            label,
            fid,
            fmt,
            // `sel?.has_video ?? '?'`
            match has_video {
                Some(true) => "true",
                Some(false) => "false",
                None => "?",
            },
            match has_audio {
                Some(true) => "true",
                Some(false) => "false",
                None => "?",
            },
        );
        if let Some(sz) = size.filter(|s| *s > 0) {
            pick.push_str(&format!(" | ~{}", moonhunt_core::util::format_bytes(sz)));
        }
        let save_to = if path.is_empty() {
            "(default downloads folder)".to_string()
        } else {
            path.clone()
        };
        dl.log.push(pick);
        dl.log.push(format!("[pick] save to: {} | cookies: {}", save_to, browser));
    }

    let task = state.media.dl.task_id;
    let path_opt = if path.is_empty() { None } else { Some(path) };
    ctx.emit("media-dl-start", move |app| async move {
        let res = moonhunt_core::ytdlp::add_yt_download(
            app,
            url,
            fmt.to_string(),
            None,
            path_opt,
            Some(browser),
            Some(fid),
            has_video,
            has_audio,
        )
        .await;
        Ok(match res {
            Ok(gid) => serde_json::json!({"task": task, "gid": gid}),
            Err(e) => serde_json::json!({"task": task, "error": e}),
        })
    });
}

/// `handleYtStarted()` — modal closes, "Download started" toast.
fn handle_started(state: &mut UiState) {
    let dl = &mut state.media.dl;
    if dl.started {
        return;
    }
    dl.started = true;
    dl.state = DlState::Running;
    dl.show_modal = false;
    state.toast(true, "Download started");
}

/// `minimizeMediaModal` is just `show_modal = false` at the call sites.

/// `cancelMediaTask()` — drop the pending result, kill the scan, reset.
fn cancel_task(state: &mut UiState, ctx: &Ctx) {
    let scan = {
        let dl = &mut state.media.dl;
        dl.task_id += 1;
        let scan = dl.active_scan_id.take();
        dl.is_starting = false;
        dl.state = DlState::Idle;
        dl.error.clear();
        dl.show_modal = false;
        dl.formats.clear();
        dl.selected_format = None;
        dl.started = false;
        scan
    };
    if let Some(sid) = scan {
        // `invoke('cancel_yt_scan')` — killing the child blocks, so keep it
        // off the UI thread.
        let core = ctx.core.clone();
        let _ = ctx.rt.spawn_blocking(move || {
            let _ = moonhunt_core::ytdlp::cancel_yt_scan(core, sid);
        });
    }
    state.toast(true, "Media task cancelled");
}

/// `clear()`
fn clear(state: &mut UiState) {
    let dl = &mut state.media.dl;
    dl.url.clear();
    dl.path.clear();
    dl.state = DlState::Idle;
    dl.error.clear();
    dl.show_modal = false;
    dl.log.clear();
    dl.formats.clear();
    dl.selected_format = None;
    dl.started = false;
    dl.active_scan_id = None;
}

// ── Events ──────────────────────────────────────────────────────────────

/// `yt-log` / `yt-dl-started` / `yt-dl-error` / `yt-dl-complete` plus this
/// module's own async results (`media-scan`, `media-dl-start`).
pub fn apply_event(state: &mut UiState, name: &str, payload: &serde_json::Value) -> bool {
    match name {
        "yt-log" => {
            let raw = payload.as_str().unwrap_or_default();
            // `.replace(/\r?\n$/, '')`
            let line = raw.strip_suffix('\n').unwrap_or(raw);
            let line = line.strip_suffix('\r').unwrap_or(line);
            if !line.is_empty() {
                let dl = &mut state.media.dl;
                dl.log.push(line.to_string());
                if dl.log.len() > 2000 {
                    let drop = dl.log.len() - 2000;
                    dl.log.drain(0..drop);
                }
            }
            true
        }
        "yt-dl-started" => {
            if state.media.dl.state == DlState::Downloading {
                handle_started(state);
            }
            true
        }
        "yt-dl-error" => {
            let msg = payload
                .get("error")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("yt-dlp failed")
                .to_string();
            {
                let dl = &mut state.media.dl;
                dl.error = msg;
                dl.state = DlState::Error;
                // Re-open the modal so the failure detail is always visible.
                dl.show_modal = true;
                dl.is_starting = false;
            }
            state.toast(false, "Media download failed");
            true
        }
        "yt-dl-complete" => {
            {
                let dl = &mut state.media.dl;
                dl.state = DlState::Idle;
                dl.show_modal = false;
                dl.is_starting = false;
            }
            state.toast(true, "Download complete!");
            true
        }
        "media-scan" => {
            // `Ctx::emit` wraps the value as `{ok, data|error}`.
            let d = payload.get("data");
            let sid = d
                .and_then(|v| v.get("scanId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if sid.is_empty() || state.media.dl.active_scan_id.as_deref() != Some(sid) {
                return true; // stale scan (cancelled / superseded)
            }
            state.media.dl.active_scan_id = None;
            state.media.dl.is_starting = false;
            let err = d
                .and_then(|v| v.get("error"))
                .and_then(|v| v.as_str())
                .or_else(|| payload.get("error").and_then(|v| v.as_str()));
            if let Some(err) = err {
                let dl = &mut state.media.dl;
                dl.error = err.to_string();
                dl.state = DlState::Error;
                return true;
            }
            let Some(res) = d.and_then(|v| v.get("result")) else {
                return true;
            };
            let dl = &mut state.media.dl;
            dl.merger_available = res
                .get("mergerAvailable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            dl.formats = res
                .get("formats")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(YtFormat::from_value).collect())
                .unwrap_or_default();
            dl.state = DlState::Selecting;
            // Auto-select the best visible combined format, else audio-only.
            let combined = dl
                .visible_formats()
                .iter()
                .find(|f| f.has_video && f.has_audio)
                .map(|f| f.format_id.clone());
            dl.selected_format = combined.or_else(|| {
                dl.visible_formats()
                    .iter()
                    .find(|f| f.has_audio && !f.has_video)
                    .map(|f| f.format_id.clone())
            });
            let count = dl.formats.len();
            let merger = if dl.merger_available {
                "available"
            } else {
                "missing"
            };
            let auto = dl.selected_format.clone().unwrap_or_else(|| "none".to_string());
            dl.log.push(format!(
                "[scan] {} format(s) found | merger {} | auto-selected: {}",
                count, merger, auto
            ));
            true
        }
        "media-dl-start" => {
            // `Ctx::emit` wraps the value as `{ok, data|error}`.
            let d = payload.get("data");
            let task = d
                .and_then(|v| v.get("task"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if task != state.media.dl.task_id {
                return true; // stale start (cancelled while the backend ran on)
            }
            let err = d
                .and_then(|v| v.get("error"))
                .and_then(|v| v.as_str())
                .or_else(|| payload.get("error").and_then(|v| v.as_str()));
            if let Some(err) = err {
                let dl = &mut state.media.dl;
                dl.error = err.to_string();
                dl.state = DlState::Error;
                return true;
            }
            handle_started(state);
            true
        }
        _ => false,
    }
}
