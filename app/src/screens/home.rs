//! Home / Dashboard screen — pixel port of `HomePage.vue`: stats grid (6
//! chips, 32px icon wells), speed card (32/800 values + 48px animated bar
//! chart), panels (14r, 16/20 header) with the old paddings/color rules.

use super::Ctx;
use crate::state::{Status, UiState};
use crate::ui::{icons as mi, widgets as mk};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Rect, ScrollArea, Sense, Stroke,
    StrokeKind,
};
use moonhunt_core::util::format_bytes;

pub fn show(state: &mut UiState, _ctx: &Ctx, ui: &mut egui::Ui) {
    let vrect = super::page_viewport(ui);
    ui.scope_builder(egui::UiBuilder::new().max_rect(vrect), |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - super::PAGE_GUTTER);
                // CSS gaps are exact (14px between sections); egui's default item
                // spacing would be added *on top* of every `add_space`.
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.add_space(4.0);
                // `.home-page`: max 1100px centered (content is narrower on the
                // 1067px window → plain full width matches the web layout).
                let w = ui.available_width();
                stats_bar(state, ui, w);
                ui.add_space(14.0);
                speed_card(state, ui, w);
                ui.add_space(14.0);
                active_panel(state, ui, w);
                ui.add_space(14.0);
                activity_panel(state, ui, w);
                ui.add_space(20.0);
            });
    });
}

// ── Shared text painters (chip/panel typography) ──────────────────────

fn draw_text(
    ui: &egui::Ui,
    pos: egui::Pos2,
    anchor: Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
) -> egui::Rect {
    let p = ui.painter();
    let galley = p.layout_no_wrap(
        text.to_string(),
        font,
        color,
    );
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
    let at = pos + vec2(anchor_x, anchor_y);
    p.galley(at, galley, color);
    Rect::from_min_size(at, size)
}

fn bold(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::bold_family())
}

fn semibold(size: f32) -> FontId {
    FontId::new(size, crate::ui::tokens::typography::semibold_family())
}

fn regular(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

/// Panel/chip surface: bg-card + 1px border, radius r (ported `.panel`,
/// `.stat-chip`, `.speed-card`).
fn surface(ui: &egui::Ui, rect: Rect, r: u8) {
    let p = mk::pal();
    ui.painter().rect_filled(rect, CornerRadius::same(r), p.bg_card);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(r),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
}

/// Ellipsize like CSS `text-overflow: ellipsis`.
fn ellipsize(ui: &egui::Ui, text: &str, font: FontId, max_w: f32) -> String {
    let width = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_string(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    crate::rtl::truncate(text, |s| width(s) <= max_w)
}

// ── Stats bar ─────────────────────────────────────────────────────────

const CHIP_COLORS: [fn() -> Color32; 6] = [
    || Color32::from(mk::pal().accent_strong),                // play_circle (accent blue)
    || Color32::from(mk::pal().status_complete),
    || Color32::from(mk::pal().status_error),
    || Color32::from_rgb(0xff, 0xd5, 0x4f),
    || Color32::from_rgb(0xb3, 0x88, 0xff),
    || Color32::from_rgb(0x26, 0xc6, 0xda),
];

const CHIP_ICONS: [&str; 6] = [
    "play_circle",
    "check_circle",
    "error",
    "pause_circle",
    "folder_open",
    "today",
];

const CHIP_LABELS: [&str; 6] = ["Active", "Completed", "Failed", "Paused", "Total Size", "Today"];

fn stats_bar(state: &UiState, ui: &mut egui::Ui, max_w: f32) {
    let (active, completed, failed, paused, total_size, today) = state_stat_counts(state);
    let vals = [
        active.to_string(),
        completed.to_string(),
        failed.to_string(),
        paused.to_string(),
        format_bytes(total_size),
        today.to_string(),
    ];
    // grid-template-columns: repeat(6, 1fr); gap: 10px
    let gap = 10.0;
    let chip_w = (max_w - gap * 5.0) / 6.0;

    let mut x = ui.cursor().min.x;
    let y = ui.cursor().min.y;
    for i in 0..6 {
        // Chip height 65.3px: padding 14×2 + text column (19.8 val + 1 label
        // margin + 16.5 label line) rounded to 65.3; measured 65–66 in ref.
        let rect = Rect::from_min_size(pos2(x, y), vec2(chip_w, 66.0));
        stat_chip(ui, rect, i, &vals[i], state.t(CHIP_LABELS[i]));
        x = rect.max.x + gap;
    }
    ui.allocate_exact_size(vec2(max_w, 66.0), Sense::hover());
}

fn stat_chip(ui: &egui::Ui, rect: Rect, i: usize, val: &str, label: &str) {
    let p = mk::pal();
    let painter = ui.painter();
    // `.stat-chip`: padding 14 12, radius 12, hover border-strong (skip lift).
    let hover = ui.ctx().pointer_interact_pos().is_some_and(|m| rect.contains(m));
    surface(ui, rect, 12);
    let _ = hover;

    // `.chip-icon`: 32×32 r8 (rgba accent .13 well). Flex `align-items:
    // center` on a 65.3px chip puts the 32px icon at +16.6. The text column
    // (37.3px: 19.8 + 1 + 16.5) is at +14.
    let color = CHIP_COLORS[i]();
    let icon = Rect::from_min_size(rect.min + vec2(12.0, 16.6), vec2(32.0, 32.0));
    painter.rect_filled(
        icon,
        CornerRadius::same(8),
        color.gamma_multiply(0.13),
    );
    mi::paint_centered(painter, icon.center(), CHIP_ICONS[i], 18.0, color);

    // `.chip-info` text column: `gap:10`; val 18/800 line-height 1.1; label
    // 11/600 uppercase, margin-top 1.
    let tx = icon.max.x + 10.0;
    draw_text(ui, pos2(tx, rect.min.y + 14.0), Align2::LEFT_TOP, val, bold(18.0), p.text);
    draw_text(
        ui,
        pos2(tx, rect.min.y + 14.0 + 19.8 + 1.0),
        Align2::LEFT_TOP,
        &label.to_uppercase(),
        semibold(11.0),
        p.text_dim,
    );
}

fn state_stat_counts(state: &UiState) -> (usize, usize, usize, usize, u64, usize) {
    let active = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Active || d.status == Status::Waiting)
        .count();
    let completed = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Complete)
        .count();
    let failed = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Error)
        .count();
    let paused = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Paused)
        .count();
    let total_size: u64 = state.downloads.iter().map(|d| d.total).sum();
    let midnight = crate::state::local_midnight_ms();
    let today = state
        .first_seen
        .values()
        .filter(|ts| **ts >= midnight)
        .count();
    (active, completed, failed, paused, total_size, today)
}

// ── Speed card ────────────────────────────────────────────────────────

fn speed_card(state: &UiState, ui: &mut egui::Ui, max_w: f32) {
    let p = mk::pal();
    // `.speed-card`: padding 22 28, radius 16; height = 22+~68+22 ≈ 112.
    let h = 112.0;
    let rect = Rect::from_min_size(ui.cursor().min, vec2(max_w, h));
    surface(ui, rect, 16);

    let down: u64 = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Active || d.status == Status::Waiting)
        .map(|d| d.speed)
        .sum();
    let active_n = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Active)
        .count();

    // `.speed-row` inside padding 22 28: items gap 32, divider between.
    // Measured ref row center = card.min.y + 42 (value glyph runs client
    // 184..206 → center 195, card.min 153).
    let row_cy = rect.min.y + 42.0;
    let inner = rect.shrink2(vec2(28.0, 22.0));
    let mut x = inner.min.x;
    x += speed_item(ui, x, row_cy, "arrow_downward", format_speed(down), state.t("Download"));
    x += 32.0;
    // `.speed-divider`: 1×40 bg border, centered on the row.
    ui.painter().rect_filled(
        Rect::from_min_size(pos2(x, row_cy - 20.0), vec2(1.0, 40.0)),
        CornerRadius::same(0),
        p.border,
    );
    x += 1.0 + 32.0;
    // Old upload is hard-coded 0 B/s (aria2 stats have no upload).
    speed_item(ui, x, row_cy, "arrow_upward", "0 B/s".into(), state.t("Upload"));
    // (old app hard-codes upload = 0 B/s — aria2 stats expose no upload rate)

    // `.speed-sub`: 13px dim 6px below the row; measured glyph rows rel card
    // +78..82 → line top +75.5.
    draw_text(
        ui,
        pos2(inner.min.x, rect.min.y + 75.5),
        Align2::LEFT_TOP,
        &state.t_of(&format!(
            "{active_n} {}",
            if active_n == 1 { state.raw("active download") } else { state.raw("active downloads") }
        )),
        regular(13.0),
        p.text_dim,
    );

    // `.speed-bars`: right side, height 48, 6px bars, 3px gap.
    // `background:linear-gradient(to top, rgba(accent,.3), accent)` — bright
    // top, dim bottom (ref rows: #DB6340 → #6E3B38). Rendered in 2px strips
    // lerping `accent ⇄ accent-over-card(30%)`.
    let bars = speed_history_bars(state);
    let n = bars.len() as f32;
    let bars_w = n * 6.0 + (n - 1.0) * 3.0;
    let bx = rect.max.x - 28.0 - bars_w;
    let by = rect.min.y + 32.0; // `.speed-visual` (h 48) centered in the 112px card
    let dim_color = blend(p.bg_card, p.accent, 0.3);
    for (i, hfrac) in bars.iter().enumerate() {
        let id = egui::Id::new("speedbar").with(i);
        let h_anim = ui.ctx().animate_value_with_time(id, *hfrac, 0.3);
        let px = bx + i as f32 * 9.0;
        let bar_h = (h_anim / 100.0 * 48.0).max(4.0);
        let bar = Rect::from_min_size(pos2(px, by + 48.0 - bar_h), vec2(6.0, bar_h));
        let top = CornerRadius::same(3);
        // 2px strips, t=1 at the bar top.
        let strips = (bar_h / 2.0).ceil() as usize;
        for j in 0..strips {
            let sy = bar.min.y + (j as f32) * 2.0;
            let sh = ((sy + 2.0).min(bar.max.y) - sy).max(0.0);
            if sh <= 0.0 {
                break;
            }
            let t = 1.0 - ((sy + sh / 2.0) - bar.min.y) / bar_h;
            let _ = top;
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(px, sy), vec2(6.0, sh)),
                CornerRadius { nw: if j == 0 { 3 } else { 0 }, ne: if j == 0 { 3 } else { 0 }, sw: if j == strips - 1 { 3 } else { 0 }, se: if j == strips - 1 { 3 } else { 0 } },
                lerp_color(dim_color, p.accent, t),
            );
        }
    }
    ui.allocate_exact_size(vec2(max_w, h), Sense::hover());
}

/// Vertical gradient strip lerp (top = a, bottom = b).
fn lerp_color(b: Color32, a: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(b.r(), a.r()), f(b.g(), a.g()), f(b.b(), a.b()))
}

/// CSS rgba(accent, a) over an opaque card background.
fn blend(base: Color32, top: Color32, a: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 * (1.0 - a) + y as f32 * a).round() as u8;
    Color32::from_rgb(f(base.r(), top.r()), f(base.g(), top.g()), f(base.b(), top.b()))
}

/// Returns right edge x consumed by one `.speed-item` (icon 22 + value 32/800
/// + uppercase label). Old gap between parts is 10px.
fn speed_item(ui: &egui::Ui, x: f32, cy: f32, icon: &str, value: String, label: &str) -> f32 {
    let p = mk::pal();
    let painter = ui.painter();
    let arrow_w = 22.0f32.max(0.0);
    mi::paint_centered(painter, pos2(x + arrow_w / 2.0, cy), icon, 22.0, p.accent_strong);
    let vfont = bold(32.0);
    let vsize = {
        let g = painter.layout_no_wrap(value.clone(), vfont.clone(), p.text);
        g.size()
    };
    let vx = x + arrow_w + 10.0;
    draw_text(ui, pos2(vx, cy), Align2::LEFT_CENTER, &value, vfont, p.text);
    let lx = vx + vsize.x + 10.0;
    draw_text(
        ui,
        pos2(lx, cy),
        Align2::LEFT_CENTER,
        &label.to_uppercase(),
        semibold(12.0),
        p.text_dim,
    );
    let lsize = painter
        .layout_no_wrap(label.to_uppercase(), semibold(12.0), p.text_dim)
        .size()
        .x;
    lx + lsize - x
}

fn speed_history_bars(state: &UiState) -> Vec<f32> {
    let mut samples: Vec<u64> = Vec::new();
    for (gid, hist) in state.speed_history.iter() {
        if state
            .downloads
            .iter()
            .any(|d| d.gid.as_str() == gid.as_str() && d.status == Status::Active)
        {
            samples.extend(hist.iter().copied());
        }
    }
    if samples.is_empty() {
        return vec![4.0; 16];
    }
    let tail: Vec<u64> = samples.split_off(samples.len().saturating_sub(16));
    let max = tail.iter().copied().max().unwrap_or(1).max(1);
    tail.iter()
        .map(|s| {
            let pct = (*s as f32 / max as f32) * 100.0;
            pct.max(4.0)
        })
        .collect()
}

fn format_speed(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB/s", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB/s", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B/s")
    }
}

// ── Panel scaffolding ─────────────────────────────────────────────────

const PANEL_HEADER_H: f32 = 52.0; // padding 16�2 + title line 19.6
const ITEM_PAD_X: f32 = 20.0;

// ── Active downloads panel ────────────────────────────────────────────

fn active_panel(state: &mut UiState, ui: &mut egui::Ui, max_w: f32) {
    let p = mk::pal();
    let mut rows: Vec<&crate::state::DownloadVm> = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Active || d.status == Status::Waiting)
        .collect();
    rows.sort_by(|a, b| {
        b.progress
            .partial_cmp(&a.progress)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Panel: header + content height.
    let content_h = if rows.is_empty() { 133.0 } else { (rows.len().min(5) as f32) * 58.0 };
    let total_h = PANEL_HEADER_H + content_h;
    let rect = Rect::from_min_size(ui.cursor().min, vec2(max_w, total_h));
    surface(ui, rect, 14);

    // Header (draw + link click).
    let header = Rect::from_min_size(rect.min, vec2(max_w, PANEL_HEADER_H));
    let ty = header.min.y + 25.0;
    let painter = ui.painter();
    mi::paint_centered(
        painter,
        pos2(header.min.x + ITEM_PAD_X + 9.0, ty),
        "download",
        18.0,
        p.accent_strong,
    );
    draw_text(
        ui,
        pos2(header.min.x + ITEM_PAD_X + 18.0 + 8.0, ty),
        Align2::LEFT_CENTER,
        state.t("Active Downloads"),
        bold(14.0),
        p.text,
    );
    // View All link.
    let lfont = semibold(12.0);
    let lig = painter.layout_no_wrap(state.t("View All").to_string(), lfont.clone(), p.accent_strong);
    let arrow_w = 16.0;
    let total = lig.size().x + 4.0 + arrow_w;
    let lx = header.max.x - ITEM_PAD_X - total;
    draw_text(ui, pos2(lx, ty), Align2::LEFT_CENTER, state.t("View All"), lfont, p.accent_strong);
    mi::paint_centered(
        painter,
        pos2(lx + lig.size().x + 4.0 + arrow_w / 2.0, ty),
        "arrow_forward",
        16.0,
        p.accent_strong,
    );
    let link_hit = Rect::from_min_size(pos2(lx - 4.0, ty - 15.0), vec2(total + 8.0, 30.0));
    let clicked = ui.input(|i| i.pointer.primary_clicked() && i.pointer.latest_pos().is_some_and(|m| link_hit.contains(m)));
    if clicked {
        state.screen = crate::state::Screen::Downloads;
    }
    painter.line_segment(
        [pos2(header.min.x, header.max.y), pos2(header.max.x, header.max.y)],
        Stroke::new(1.0, p.border),
    );

    // Items.
    if rows.is_empty() {
        empty_state(
            ui,
            Rect::from_min_size(pos2(rect.min.x, header.max.y), vec2(max_w, content_h)),
            "cloud_download",
            state.t("No active downloads"),
            state.t("Start one from the Downloads page."),
        );
        ui.allocate_exact_size(vec2(max_w, total_h), Sense::hover());
        return;
    }
    let mut y = header.max.y;
    for (idx, d) in rows.iter().take(5).enumerate() {
        let item = Rect::from_min_size(pos2(rect.min.x, y), vec2(max_w, 58.0));
        // hover bg
        let hov = ui.ctx().pointer_interact_pos().is_some_and(|m| item.contains(m));
        if hov {
            ui.painter().rect_filled(item, CornerRadius::same(0), p.hover_veil.gamma_multiply(0.6));
        }
        if idx < 4 {
            ui.painter().line_segment(
                [pos2(item.min.x, item.max.y), pos2(item.max.x, item.max.y)],
                Stroke::new(1.0, p.border),
            );
        }
        active_item(ui, item, d);
        y = item.max.y;
    }
    ui.allocate_exact_size(vec2(max_w, total_h), Sense::hover());
}

/// `.active-item`: info left, progress (120px track) + pct right.
fn active_item(ui: &egui::Ui, item: Rect, d: &crate::state::DownloadVm) {
    let p = mk::pal();
    let painter = ui.painter();
    let name_x = item.min.x + ITEM_PAD_X;
    let name = ellipsize(ui, &d.filename, semibold(13.0), item.width() - 260.0);
    draw_text(ui, pos2(name_x, item.min.y + 14.0 + 6.0), Align2::LEFT_CENTER, &name, semibold(13.0), p.text);
    let mut meta = format!("{} · {}", format_speed(d.speed), format_bytes(d.total));
    if let Some(s) = d.remaining_seconds {
        meta.push_str(&format!(" · {}", format_remaining(s)));
    }
    draw_text(ui, pos2(name_x, item.min.y + 31.0), Align2::LEFT_CENTER, &meta, regular(11.0), p.text_dim);

    // progress-track 120×6 r3 + fill; pct 36px right 12/700 dim.
    let track = Rect::from_min_size(
        pos2(item.max.x - ITEM_PAD_X - 36.0 - 10.0 - 120.0, item.center().y - 3.0),
        vec2(120.0, 6.0),
    );
    painter.rect_filled(track, CornerRadius::same(3), p.bg_card_inset);
    draw_text(
        ui,
        pos2(item.max.x - ITEM_PAD_X - 18.0, item.center().y),
        Align2::CENTER_CENTER,
        &format!("{:.0}%", d.progress),
        bold(12.0),
        p.text_dim,
    );
}

fn format_remaining(s: u64) -> String {
    if s >= 3600 {
        format!("{:.1}h", s as f64 / 3600.0)
    } else if s >= 60 {
        format!("{:.0}m", s as f64 / 60.0)
    } else {
        format!("{s}s")
    }
}

// ── Recent activity panel ─────────────────────────────────────────────

fn activity_panel(state: &UiState, ui: &mut egui::Ui, max_w: f32) {
    let p = mk::pal();
    let mut rows: Vec<&crate::state::DownloadVm> = state
        .downloads
        .iter()
        .filter(|d| d.status == Status::Complete || d.status == Status::Error)
        .collect();
    rows.sort_by_cached_key(|d| {
        std::cmp::Reverse(state.first_seen.get(d.gid.as_str()).copied().unwrap_or(0))
    });

    let content_h = if rows.is_empty() { 133.0 } else { (rows.len().min(5) as f32) * 56.0 };
    let total_h = PANEL_HEADER_H + content_h;
    let rect = Rect::from_min_size(ui.cursor().min, vec2(max_w, total_h));
    surface(ui, rect, 14);

    let header = Rect::from_min_size(rect.min, vec2(max_w, PANEL_HEADER_H));
    let ty = header.min.y + 25.0;
    let painter = ui.painter();
    mi::paint_centered(
        painter,
        pos2(header.min.x + ITEM_PAD_X + 9.0, ty),
        "history",
        18.0,
        p.accent_strong,
    );
    draw_text(
        ui,
        pos2(header.min.x + ITEM_PAD_X + 18.0 + 8.0, ty),
        Align2::LEFT_CENTER,
        state.t("Recent Activity"),
        bold(14.0),
        p.text,
    );
    painter.line_segment(
        [pos2(header.min.x, header.max.y), pos2(header.max.x, header.max.y)],
        Stroke::new(1.0, p.border),
    );

    if rows.is_empty() {
        empty_state(
            ui,
            Rect::from_min_size(pos2(rect.min.x, header.max.y), vec2(max_w, content_h)),
            "hourglass_empty",
            state.t("No recent activity"),
            state.t("Completed and failed downloads appear here."),
        );
        ui.allocate_exact_size(vec2(max_w, total_h), Sense::hover());
        return;
    }
    let mut y = header.max.y;
    for (idx, d) in rows.iter().take(5).enumerate() {
        let item = Rect::from_min_size(pos2(rect.min.x, y), vec2(max_w, 56.0));
        let hov = ui.ctx().pointer_interact_pos().is_some_and(|m| item.contains(m));
        if hov {
            ui.painter().rect_filled(item, CornerRadius::same(0), p.hover_veil.gamma_multiply(0.6));
        }
        if idx < 4 {
            ui.painter().line_segment(
                [pos2(item.min.x, item.max.y), pos2(item.max.x, item.max.y)],
                Stroke::new(1.0, p.border),
            );
        }
        // `.activity-icon`: 28px circle, 12% tint.
        let color = if d.status == Status::Error { p.status_error } else { p.status_complete };
        let icon_r = Rect::from_min_size(
            pos2(item.min.x + ITEM_PAD_X, item.center().y - 14.0),
            vec2(28.0, 28.0),
        );
        painter.rect_filled(icon_r, CornerRadius::same(14), color.gamma_multiply(0.13));
        mi::paint_centered(
            painter,
            icon_r.center(),
            if d.status == Status::Error { "error" } else { "check_circle" },
            16.0,
            color,
        );
        let name_x = icon_r.max.x + 12.0;
        let name = ellipsize(ui, &d.filename, semibold(13.0), item.width() - 220.0);
        draw_text(ui, pos2(name_x, item.min.y + 18.0), Align2::LEFT_CENTER, &name, semibold(13.0), p.text);
        // `.activity-meta`: `formatSize(d.size)` — a persisted row's stored
        // string, else the bytes we know about.
        let size_s = if d.size_text.is_empty() {
            format_bytes(d.completed)
        } else {
            d.size_text.clone()
        };
        draw_text(
            ui,
            pos2(name_x, item.min.y + 36.0),
            Align2::LEFT_CENTER,
            &format!("{} · aria2", size_s),
            regular(11.0),
            p.text_dim,
        );
        draw_text(
            ui,
            pos2(item.max.x - ITEM_PAD_X, item.center().y),
            Align2::RIGHT_CENTER,
            &ago_text(state, state.first_seen.get(d.gid.as_str()).copied()),
            regular(11.0),
            p.text_dim,
        );
        y = item.max.y;
    }
    ui.allocate_exact_size(vec2(max_w, total_h), Sense::hover());
}

/// `.empty-state`: centered, padding 28/20, 32px icon @ 40% alpha + 8px gap,
/// 13/600 muted title, 2px gap, 12px dim sub.
fn empty_state(ui: &egui::Ui, rect: Rect, icon: &str, title: &str, sub: &str) {
    let p = mk::pal();
    let painter = ui.painter();
    mi::paint_centered(
        painter,
        pos2(rect.center().x, rect.min.y + 28.0 + 16.0),
        icon,
        32.0,
        p.text_dim.gamma_multiply(0.4),
    );
    draw_text(
        ui,
        pos2(rect.center().x, rect.min.y + 28.0 + 32.0 + 8.0),
        Align2::CENTER_TOP,
        title,
        semibold(13.0),
        p.text_muted,
    );
    draw_text(
        ui,
        pos2(rect.center().x, rect.min.y + 28.0 + 32.0 + 8.0 + 18.2 + 2.0),
        Align2::CENTER_TOP,
        sub,
        regular(12.0),
        p.text_dim,
    );
}

fn ago_text(state: &crate::state::UiState, ts: Option<i64>) -> String {
    let Some(ts) = ts else {
        return String::new();
    };
    let diff_min = (crate::state::chrono_now_ms() - ts) / 60_000;
    // Same frame `HomePage.vue#formatTime` uses: translate the sentence *with*
    // its `{n}` placeholder, splice the number in, then order it once.
    let logical = if diff_min < 1 {
        state.raw("Just now").to_owned()
    } else if diff_min < 60 {
        state.raw("m ago").replace("{n}", &diff_min.to_string())
    } else if diff_min < 60 * 24 {
        state.raw("h ago").replace("{n}", &(diff_min / 60).to_string())
    } else {
        state.raw("d ago")
            .replace("{n}", &(diff_min / (60 * 24)).to_string())
    };
    state.t_of(&logical)
}

