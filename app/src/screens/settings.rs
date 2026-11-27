//! Settings screen — pixel port of `SettingsPage.vue`: 240px `--bg-card`
//! sidebar (title, icon nav, Save/Reset footer), 24/32-padded content with
//! 18/700 section titles, uppercase 12/600 field labels, 11px hints, diamond
//! section dividers, toggle/stepper/seg/language-card controls at old metrics.

use crate::screens::Ctx;
use crate::state::{SettingsCat, UiState};
use crate::ui::{flags, icons as mi, widgets as mk};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Rect, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, TextEdit,
};
use moonhunt_core::state::ProxySettings;

pub fn show(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui) {
    // `.watch(activeCategory)`: transient proxy alerts belong to the Network
    // tab — clear them on tab switch.
    if state.settings_cat != crate::state::SettingsCat::Network {
        state.st_detect = None;
        state.st_test = None;
    }
    // The Settings screen opts out of `.main-content` padding in main.rs:
    // `.settings-sidebar` is `position: fixed` at (rail, titlebar-bottom)
    // with `height: calc(100vh - 50px)`, while `.settings-content` keeps the
    // main padding (18/20/24) and adds `margin-left: 240` + `padding: 24 32 32`.
    let area = ui.available_rect_before_wrap();
    let sidebar = Rect::from_min_size(area.min, vec2(SIDEBAR_W, area.height()));
    settings_sidebar(ui, state, ctx, sidebar);
    // Scroll viewport: main padding (left 20 + sidebar margin-left 240),
    // top 18, bottom 24; the right side sits on the panel's right margin.
    let viewport = Rect::from_min_max(
        pos2(area.min.x + 20.0 + SIDEBAR_W, area.min.y + 18.0),
        pos2(area.max.x, area.max.y - 24.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(viewport), |ui| {
        content(ui, state, ctx);
    });
}

// ── Text style helpers ────────────────────────────────────────────────

/// Points of a rotated-square ornament: an 8px square rotated 45° has a
/// half-diagonal of ≈ 5.66.
fn diamond_pts(c: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    vec![
        pos2(c.x, c.y - r),
        pos2(c.x + r, c.y),
        pos2(c.x, c.y + r),
        pos2(c.x - r, c.y),
    ]
}

/// `.section-divider`: 1px `--border` line with the centered ornament
/// (bg-primary fill, 1px accent-strong border, `0 0 6px` accent glow).
fn divider(ui: &mut egui::Ui) {
    let p = mk::pal();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(0), p.border);
    let c = pos2(rect.center().x, rect.center().y);
    // The CSS box-shadow glow, approximated with two translucent diamonds.
    ui.painter().add(egui::Shape::convex_polygon(
        diamond_pts(c, 13.0),
        p.accent.gamma_multiply(0.05),
        Stroke::NONE,
    ));
    ui.painter().add(egui::Shape::convex_polygon(
        diamond_pts(c, 9.0),
        p.accent.gamma_multiply(0.08),
        Stroke::NONE,
    ));
    ui.painter().add(egui::Shape::convex_polygon(
        diamond_pts(c, 5.66),
        p.bg_primary,
        Stroke::new(1.0, p.accent_strong),
    ));
}

/// Between-block `.section-divider` (`margin: 20px 0`; the trailing 20px is
/// the collapse against the next title's `margin-top: 20px`).
fn inline_divider(ui: &mut egui::Ui) {
    mk::gap(ui, 20.0);
    divider(ui);
    mk::gap(ui, 20.0);
}

// ── Left sidebar (fixed 240px look) ───────────────────────────────────

const SIDEBAR_W: f32 = 240.0;

const CATEGORIES: [(SettingsCat, &str, &str); 8] = [
    (SettingsCat::General, "General", "tune"),
    (SettingsCat::Aria2c, "Aria2c", "cloud_download"),
    (SettingsCat::Url, "URL Downloads", "link"),
    (SettingsCat::Torrent, "Torrent", "bolt"),
    (SettingsCat::Network, "Network", "wifi"),
    (SettingsCat::Extension, "Extension", "extension"),
    (SettingsCat::Appearance, "Appearance", "palette"),
    (SettingsCat::Performance, "Performance", "monitoring"),
];/// Footer metrics: border 1 + padding 12/16 + save 38 + gap 6 + reset 32.
const FOOTER_H: f32 = 1.0 + 12.0 + 38.0 + 6.0 + 32.0 + 16.0;

fn settings_sidebar(ui: &mut egui::Ui, state: &mut UiState, ctx: &Ctx, sb: Rect) {
    let p = mk::pal();
    ui.scope_builder(egui::UiBuilder::new().max_rect(sb), |ui| {
        // `.settings-sidebar { background: bg-card; border-right: 1px }`.
        ui.painter().rect_filled(sb, CornerRadius::same(0), p.bg_card);
        ui.painter().line_segment(
            [pos2(sb.max.x - 1.0, sb.min.y), pos2(sb.max.x - 1.0, sb.max.y)],
            Stroke::new(1.0, p.border),
        );

        // `.sidebar-header { padding: 20 16 16 }`, title 16/700.
        ui.painter().text(
            pos2(sb.min.x + 16.0, sb.min.y + 20.0),
            Align2::LEFT_TOP,
            state.t("Settings"),
            FontId::new(16.0, crate::ui::tokens::typography::bold_family()),
            p.text,
        );

        // `.sidebar-nav { padding: 8px 12px; flex: 1; overflow-y: auto }` under
        // the header block (20 top pad + ~22 title line + 16 bottom pad),
        // items 40px + 2px gap — scrolled between header and pinned footer.
        let nav = Rect::from_min_max(
            pos2(sb.min.x + 12.0, sb.min.y + 20.0 + 22.0 + 16.0 + 8.0),
            pos2(sb.max.x - 12.0, sb.max.y - FOOTER_H - 8.0),
        );
        let total = CATEGORIES.len() as f32 * 42.0 - 2.0;
        let max_scroll = (total - nav.height()).max(0.0);
        let nav_id = egui::Id::new("settings-nav-scroll");
        let mut scroll = ui.data(|d| d.get_temp::<f32>(nav_id).unwrap_or(0.0));
        if ui.rect_contains_pointer(nav) {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if dy != 0.0 {
                scroll = (scroll - dy).clamp(0.0, max_scroll);
                ui.data_mut(|d| d.insert_temp(nav_id, scroll));
                // Smoothed wheel delta is a per-frame slice; keep it draining.
                ui.ctx().request_repaint();
            }
        }
        scroll = scroll.clamp(0.0, max_scroll);
        ui.scope_builder(egui::UiBuilder::new().max_rect(nav), |ui| {
            ui.set_clip_rect(nav);
            let mut y = nav.min.y - scroll;
            for (cat, label, icon) in CATEGORIES {
                let label = state.t(label);
                let r = Rect::from_min_size(pos2(nav.min.x, y), vec2(nav.width(), 40.0));
                if r.max.y > nav.min.y && r.min.y < nav.max.y {
                    if sidebar_item(ui, r, cat == state.settings_cat, label, icon).clicked() {
                        state.settings_cat = cat;
                    }
                }
                y += 42.0;
            }
        });

        // `.sidebar-footer { margin-top: auto; padding: 12 16 16;
        // border-top: 1px; gap: 6px }`.
        let footer = Rect::from_min_size(
            pos2(sb.min.x, sb.max.y - FOOTER_H),
            vec2(SIDEBAR_W, FOOTER_H),
        );
        ui.painter().line_segment(
            [footer.min, pos2(footer.max.x, footer.min.y)],
            Stroke::new(1.0, p.border),
        );
        let inner_w = SIDEBAR_W - 32.0;

        // `.sidebar-save-btn`: accent bg, #0a0e1a text, 13/700,
        // icon 16 + 6px gap centered as a group.
        let save = Rect::from_min_size(
            pos2(footer.min.x + 16.0, footer.min.y + 13.0),
            vec2(inner_w, 38.0),
        );
        let hover = ui.ctx().pointer_interact_pos().is_some_and(|m| save.contains(m));
        let dark = Color32::from_rgb(0x0a, 0x0e, 0x1a);
        let fill = if hover { p.accent.gamma_multiply(0.9) } else { p.accent };
        ui.painter().rect_filled(save, CornerRadius::same(8), fill);
        let save_font = FontId::new(13.0, crate::ui::tokens::typography::bold_family());
        let tw = ui
            .painter()
            .layout_no_wrap(state.t("Save Changes").to_string(), save_font.clone(), dark)
            .rect
            .width();
        let gx = save.center().x - (16.0 + 6.0 + tw) / 2.0;
        mi::paint_centered(ui.painter(), pos2(gx + 8.0, save.center().y), "save", 16.0, dark);
        ui.painter().text(
            pos2(gx + 22.0, save.center().y),
            Align2::LEFT_CENTER,
            state.t("Save Changes"),
            save_font,
            dark,
        );
        if ui.allocate_rect(save, Sense::click()).clicked() && state.settings_dirty {
            save_and_apply(state, ctx);
        }

        // `.sidebar-reset-btn`: hover-bg + border, dim 12, icon 16 + gap 6.
        let reset = Rect::from_min_size(pos2(save.min.x, save.max.y + 6.0), vec2(inner_w, 32.0));
        let rhover = ui.ctx().pointer_interact_pos().is_some_and(|m| reset.contains(m));
        ui.painter().rect_filled(reset, CornerRadius::same(8), p.hover_veil);
        ui.painter().rect_stroke(
            reset,
            CornerRadius::same(8),
            Stroke::new(1.0, if rhover { p.border_strong } else { p.border }),
            StrokeKind::Inside,
        );
        let reset_font = FontId::new(12.0, egui::FontFamily::Proportional);
        let rtw = ui
            .painter()
            .layout_no_wrap(state.t("Reset to Defaults").to_string(), reset_font.clone(), p.text_dim)
            .rect
            .width();
        let rgx = reset.center().x - (16.0 + 6.0 + rtw) / 2.0;
        let rc = if rhover { p.text } else { p.text_dim };
        mi::paint_centered(
            ui.painter(),
            pos2(rgx + 8.0, reset.center().y),
            "restart_alt",
            16.0,
            rc,
        );
        ui.painter().text(
            pos2(rgx + 22.0, reset.center().y),
            Align2::LEFT_CENTER,
            state.t("Reset to Defaults"),
            reset_font,
            rc,
        );
        if ui.allocate_rect(reset, Sense::click()).clicked() {
            state.settings = ProxySettings::default();
            state.settings_dirty = true;
        }
    });
}

fn sidebar_item(
    ui: &mut egui::Ui,
    rect: Rect,
    active: bool,
    label: &str,
    icon: &str,
) -> egui::Response {
    let p = mk::pal();
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    if active {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), p.accent.gamma_multiply(0.10));
    } else if hovered {
        ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    }
    let color = if active {
        p.accent_strong
    } else if hovered {
        p.text
    } else {
        p.text_dim
    };
    // `.sidebar-item i { font-size: 20px; width: 22px; text-align: center }`
    // after 12px left padding, then a 10px gap before the label.
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 12.0 + 11.0, rect.center().y),
        icon,
        20.0,
        color,
    );
    let font = FontId::new(
        14.0,
        if active {
            crate::ui::tokens::typography::semibold_family()
        } else {
            egui::FontFamily::Proportional
        },
    );
    ui.painter().text(
        pos2(rect.min.x + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        color,
    );
    resp
}

// ── Content pane ──────────────────────────────────────────────────────

fn content(ui: &mut egui::Ui, state: &mut UiState, ctx: &Ctx) {
    // `.settings-content` scrolls the *pane* and carries `padding: 24px 32px 32px`
    // inside that scroller, so the wheel works over the padding too. In egui a
    // margin outside the ScrollArea is dead space, so the padding is applied to
    // the content rect instead and the scroller keeps the full pane width.
    let vrect = super::page_viewport(ui);
    ui.scope_builder(egui::UiBuilder::new().max_rect(vrect), |ui| {
        ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut pad = ui.available_rect_before_wrap();
            pad.min.x += 32.0;
            // 32 CSS padding + the panel gutter the scroller now spans.
            pad.max.x -= 32.0 + super::PAGE_GUTTER;
            pad.min.y += 24.0;
            pad.max.y -= 32.0;
            ui.scope_builder(egui::UiBuilder::new().max_rect(pad), |ui| {
                // The MoonKit global style adds `item_spacing` (S) after every
                // widget; the web reference relies on CSS margins only, so all
                // vertical rhythm here is the explicit CSS-metric gaps (field
                // mb 16, label mb 6, sub mb 20 …).
                ui.spacing_mut().item_spacing.y = 0.0;
                let w = ui.available_width();
                match state.settings_cat {
                    SettingsCat::General => general_section(state, ctx, ui, w),
                    SettingsCat::Aria2c => aria2c_section(state, ctx, ui, w),
                    SettingsCat::Url => url_section(state, ctx, ui, w),
                    SettingsCat::Torrent => torrent_section(state, ctx, ui, w),
                    SettingsCat::Network => network_section(state, ctx, ui, w),
                    SettingsCat::Extension => extension_section(state, ctx, ui, w),
                    SettingsCat::Appearance => appearance_section(state, ctx, ui, w),
                    SettingsCat::Performance => performance_section(state, ctx, ui, w),
                }
            });
        });
    });
}

// ── Section scaffolding (flow) ────────────────────────────────────────

/// `.section-title` 18/700 (`margin-bottom: 4px`) + `.section-sub` 13 dim
/// (`margin-bottom: 20px`). An empty sub renders an inline h2 only.
fn section_head(ui: &mut egui::Ui, title: &str, sub: &str) {
    let p = mk::pal();
    ui.label(
        RichText::new(title)
            .font(FontId::new(18.0, crate::ui::tokens::typography::bold_family()))
            .color(p.text),
    );
    mk::gap(ui, 4.0);
    if !sub.is_empty() {
        ui.label(RichText::new(sub).size(13.0).color(p.text_dim));
        mk::gap(ui, 20.0);
    }
}

/// `.field label`: uppercase, 12/600, muted.
fn field_label(ui: &mut egui::Ui, text: &str) {
    let p = mk::pal();
    ui.label(
        RichText::new(text.to_uppercase())
            .font(FontId::new(12.0, crate::ui::tokens::typography::semibold_family()))
            .color(p.text_muted),
    );
    mk::gap(ui, 6.0);
}

/// `.field-hint`: 11px dim. The caller inserts the leading gap (`margin-top:
/// 6px` collapses against the preceding element's margin) — the CSS bottom
/// margin is absorbed by the next element's own gap.
fn hint(ui: &mut egui::Ui, text: &str) {
    let p = mk::pal();
    ui.label(RichText::new(text).size(11.0).color(p.text_dim));
}

/// End of `.settings-section`: `padding-bottom: 24px` → border-bottom line →
/// `margin-bottom: 32px`.
fn end_section(ui: &mut egui::Ui) {
    mk::gap(ui, 24.0);
    divider(ui);
    mk::gap(ui, 32.0);
}

// ── General ───────────────────────────────────────────────────────────

/// Language endonyms live outside the i18n table, so shape/reorder them here.
fn label_visual(label: &str) -> String {
    crate::rtl::visual(label)
}

fn general_section(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(ui, state.t("General"), state.t("General application preferences."));

    // ── Language: `.language-cards` grid (minmax(150px,1fr), 10px gap).
    // The cards paint their own names (日本語, 中文); `App::sync_fonts` has the
    // system CJK fonts installed by the time this screen draws.
    field_label(ui, state.t("Language"));
    let codes = [
        ("en", "English"),
        ("fa", "فارسی"),
        ("fr", "Français"),
        ("es", "Español"),
        ("pt", "Português"),
        ("ru", "Русский"),
        ("ja", "日本語"),
        ("zh", "中文"),
        ("ar", "العربية"),
    ];
    let gap = 10.0;
    let per_row = ((w + gap) / (150.0 + gap)).floor().max(1.0) as usize;
    // `minmax(150px, 1fr)`: the tracks stretch, so every card is the row width
    // split evenly (fixed 150px cards left the last column ~20px short).
    let card_w = ((w - gap * (per_row.saturating_sub(1)) as f32) / per_row as f32).max(120.0);
    let mut col = 0usize;
    while col < codes.len() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for (code, label) in codes.iter().skip(col).take(per_row) {
                let r = Rect::from_min_size(ui.cursor().min, vec2(card_w, 56.0));
                ui.painter().rect_filled(r, CornerRadius::same(12), p.bg_card);
                let active = state.settings.language == *code;
                let hov = ui.ctx().pointer_interact_pos().is_some_and(|m| r.contains(m));
                let anim = ui.ctx().animate_value_with_time(
                    egui::Id::new("lang-card").with(*code),
                    if active { 1.0 } else if hov { 0.35 } else { 0.0 },
                    0.12,
                );
                let bc = Color32::from(blend_rgb(p.border, p.accent_strong, anim.clamp(0.0, 1.0)));
                ui.painter().rect_stroke(
                    r,
                    CornerRadius::same(12),
                    Stroke::new(2.0, bc),
                    StrokeKind::Inside,
                );
                if let Some(tex) = flags::texture(ui.ctx(), code) {
                    let fr =
                        Rect::from_min_size(r.min + vec2(14.0, (56.0 - 21.0) / 2.0), vec2(28.0, 21.0));
                    let sized = egui::load::SizedTexture::new(tex.id(), vec2(28.0, 21.0));
                    ui.put(fr, egui::Image::new(sized));
                }
                ui.painter().text(
                    pos2(r.min.x + 52.0, r.center().y),
                    Align2::LEFT_CENTER,
                    &label_visual(*label),
                    FontId::new(13.0, crate::ui::tokens::typography::semibold_family()),
                    p.text,
                );
                if active {
                    mi::paint_centered(
                        ui.painter(),
                        pos2(r.max.x - 17.0, r.min.y + 17.0),
                        "check_circle",
                        18.0,
                        p.accent_strong,
                    );
                }
                if ui.allocate_rect(r, Sense::click()).clicked() {
                    state.settings.language = code.to_string();
                    state.settings_dirty = true;
                }
            }
        });
        col += per_row;
        if col < codes.len() {
            mk::gap(ui, gap);
        }
    }
    field_gap(ui);
    inline_divider(ui);

    // ── Global Download Speed Limit (`.field.row-field.global-speed-limit`).
    section_head(ui, state.t("Global Download Speed Limit"), "");
    let g_en = state.settings.global_speed_limit_enabled.unwrap_or(false);
    row_field(ui, state.t("Max Download Speed"), |ui| {
        if mk::switch(ui, g_en, "g-speed-sw").clicked() {
            state.settings.global_speed_limit_enabled = Some(!g_en);
            state.settings_dirty = true;
        }
    });
    if g_en {
        // `.global-speed-limit { margin-bottom: 2 }` before `.speed-control-row`.
        mk::gap(ui, 2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            let mut v = state.settings.max_download_speed as i32;
            if stepper(ui, "g-speed", &mut v, 0, 1_000_000) {
                state.settings.max_download_speed = v.max(0) as u32;
                state.settings_dirty = true;
            }
            unit_switch(ui, "g-speed-unit", &mut state.st_speed_unit);
        });
    }
    mk::gap(ui, 6.0);
    hint(ui, state.t("This limit applies to all downloads at once."));
    inline_divider(ui);

    // ── Search
    section_head(ui, state.t("Search"), "");
    let sp = state.settings.search_preview_enabled;
    row_field(ui, state.t("Search Preview"), |ui| {
        if mk::switch(ui, sp, "st-search-preview").clicked() {
            state.settings.search_preview_enabled = !sp;
            state.settings_dirty = true;
        }
    });
    row_hint(ui, state.t("Show matching downloads in a dropdown while typing in the titlebar search."));
    inline_divider(ui);

    // ── Security
    section_head(ui, state.t("Security"), "");
    let vs = state.settings.verify_ssl;
    row_field(ui, state.t("Verify Certificates"), |ui| {
        if mk::switch(ui, vs, "st-verify").clicked() {
            state.settings.verify_ssl = !vs;
            state.settings_dirty = true;
        }
    });
    row_hint(ui, state.t("When enabled, certificates are verified during downloads. Disable only if you encounter certificate errors."));
    inline_divider(ui);

    // ── Download Engine
    section_head(ui, state.t("Download Engine"), "");
    let is_exp = state.settings.progress_engine.as_deref() == Some("experimental");
    let (seg_legacy, seg_exp) = (state.t("Legacy"), state.t("Experimental"));
    row_field(ui, state.t("Progress Engine"), |ui| {
        if mk::seg_switch(ui, "st-engine", seg_legacy, seg_exp, is_exp) != is_exp {
            state.settings.progress_engine =
                Some(if !is_exp { "experimental" } else { "legacy" }.into());
            state.settings_dirty = true;
        }
    });
    row_hint(ui, state.t("Legacy parses yt-dlp percent text. Experimental counts bytes with a fixed total and updates more often."));

    let dg = state.settings.show_download_diagnostics;
    row_field(ui, state.t("Show Download Diagnostics"), |ui| {
        if mk::switch(ui, dg, "st-diag").clicked() {
            state.settings.show_download_diagnostics = !dg;
            state.settings_dirty = true;
        }
    });
    row_hint(ui, state.t("Adds a raw diagnostics section (engine, phase, bytes, backend health) to the download Details modal."));
    end_section(ui);
    let _ = ctx;
}

fn blend_rgb(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

// ── Shared controls (reference CSS metrics) ───────────────────────────

/// `.field { margin-bottom: 16px }` after a completed stacked field.
fn field_gap(ui: &mut egui::Ui) {
    mk::gap(ui, 16.0);
}

/// `.field-hint` under a `.row-field`: the field's `margin-bottom: 16`
/// collapses against the hint's `margin-top: 6` → 16px.
fn row_hint(ui: &mut egui::Ui, text: &str) {
    mk::gap(ui, 16.0);
    hint(ui, text);
}

/// `.row-field { display:flex; justify-content: space-between }` — uppercase
/// `.field label` left, control right (vertically centered).
fn row_field(ui: &mut egui::Ui, label: &str, right: impl FnOnce(&mut egui::Ui)) {
    let p = mk::pal();
    row_field_custom(
        ui,
        |ui| {
            ui.label(
                RichText::new(label.to_uppercase())
                    .font(FontId::new(
                        12.0,
                        crate::ui::tokens::typography::semibold_family(),
                    ))
                    .color(p.text_muted),
            );
        },
        right,
    );
}

/// `.row-field` with arbitrary left content (label + hint, optional lamp).
fn row_field_custom(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        left(ui);
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
            right(ui);
        });
    });
}

/// `.nd-input`: 36px box, faint white inset fill, MoonKit border (accent when
/// focused) — the text row is vertically centered with the CSS side padding
/// (`0 10`, `34` with a lead button, `36`/`72` reserved for trailing ones).
/// `trail` buttons sit at `right:10` then `right:42` (26px, 7r, icon 18).
/// Returns the clicked action icon.
fn nd_field(
    ui: &mut egui::Ui,
    salt: &str,
    width: f32,
    value: &mut String,
    placeholder: &str,
    lead: Option<&'static str>,
    trail: &[&'static str],
    secret: bool,
    editable: bool,
) -> Option<&'static str> {
    let p = mk::pal();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    let fid = egui::Id::new(("ndf", salt));
    let focused = editable && ui.ctx().memory(|m| m.has_focus(fid));
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
    let pad_l: i8 = if lead.is_some() { 34 } else { 10 };
    let pad_r: i8 = match trail.len() {
        0 => 10,
        1 => 36,
        _ => 72,
    };
    let line_h =
        ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    if editable {
        let text_y = rect.min.y + ((rect.height() - line_h) * 0.5).max(0.0);
        let text_rect =
            Rect::from_min_size(pos2(rect.min.x, text_y), vec2(rect.width(), line_h));
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(text_rect).scope_id(fid),
            |ui| {
                ui.add(
                    TextEdit::singleline(value)
                        .desired_width(text_rect.width())
                        .font(font.clone())
                        // egui defaults to the accent fg; `.nd-input` uses
                        // plain `color: var(--text)`.
                        .text_color(p.text)
                        .hint_text(
                            RichText::new(placeholder)
                                .size(12.0)
                                .color(p.text_dim.gamma_multiply(0.55)),
                        )
                        .password(secret)
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                            left: pad_l,
                            right: pad_r,
                            top: 0,
                            bottom: 0,
                        })),
                );
            },
        );
    } else if !value.is_empty() {
        // Read-only display (aria2 path, disabled torrent location).
        let clip = Rect::from_min_max(
            pos2(rect.min.x + pad_l as f32, rect.min.y),
            pos2(rect.max.x - pad_r as f32, rect.max.y),
        );
        ui.painter().with_clip_rect(clip).text(
            pos2(clip.min.x, rect.center().y),
            Align2::LEFT_CENTER,
            value.clone(),
            font,
            if editable { p.text } else { p.text_dim },
        );
    } else if !placeholder.is_empty() {
        ui.painter().text(
            pos2(rect.min.x + pad_l as f32, rect.center().y),
            Align2::LEFT_CENTER,
            placeholder,
            font,
            p.text_dim.gamma_multiply(0.55),
        );
    }
    // The TextEdit scope above moves the cursor back to the text's own
    // (shorter) row; advance to the box bottom so the next section keeps the
    // CSS rhythm (same trick as `modals::nd_input`).
    ui.advance_cursor_after_rect(rect);

    if let Some(icon) = lead {
        let c = Rect::from_min_size(
            pos2(rect.min.x + 10.0, rect.center().y - 13.0),
            vec2(26.0, 26.0),
        );
        let rr = ui.interact(c, fid.with("lead"), Sense::click());
        if rr.hovered() {
            ui.painter()
                .rect_filled(c, CornerRadius::same(7), p.accent.gamma_multiply(0.10));
        }
        mi::paint_centered(
            ui.painter(),
            c.center(),
            icon,
            18.0,
            if rr.hovered() { p.accent_strong } else { p.text_muted },
        );
        if rr.clicked() {
            return Some(icon);
        }
    }
    let mut out = None;
    for (i, icon) in trail.iter().enumerate() {
        let from_right = 10.0 + 32.0 * (trail.len() - 1 - i) as f32;
        let c = Rect::from_min_size(
            pos2(rect.max.x - from_right - 26.0, rect.center().y - 13.0),
            vec2(26.0, 26.0),
        );
        let rr = ui.interact(c, fid.with(("trail", i)), Sense::click());
        if rr.hovered() {
            ui.painter()
                .rect_filled(c, CornerRadius::same(7), p.accent.gamma_multiply(0.10));
        }
        mi::paint_centered(
            ui.painter(),
            c.center(),
            icon,
            18.0,
            if rr.hovered() { p.accent_strong } else { p.text_muted },
        );
        if rr.clicked() {
            out = Some(*icon);
        }
    }
    out
}

/// `.stepper`: 36/56/36 — decrement, editable 600/center number, increment;
/// outer 8r border, buttons `--hover-bg`, value cell `--bg-surface`.
fn stepper(ui: &mut egui::Ui, salt: &str, value: &mut i32, min: i32, max: i32) -> bool {
    let p = mk::pal();
    // `.stepper` sets no height of its own: 1px border + 38px content = 40
    // outer (128 content + 2 border = 130 wide).
    let (rect, _) = ui.allocate_exact_size(vec2(130.0, 40.0), Sense::hover());
    let inner = Rect::from_min_max(rect.min + vec2(1.0, 1.0), rect.max - vec2(1.0, 1.0));
    ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    let cell = Rect::from_min_size(pos2(inner.min.x + 36.0, inner.min.y), vec2(56.0, 38.0));
    ui.painter()
        .rect_filled(cell, CornerRadius::same(0), p.bg_surface);
    ui.painter().line_segment(
        [pos2(cell.min.x, inner.min.y), pos2(cell.min.x, inner.max.y)],
        Stroke::new(1.0, p.border_strong),
    );
    ui.painter().line_segment(
        [pos2(cell.max.x, inner.min.y), pos2(cell.max.x, inner.max.y)],
        Stroke::new(1.0, p.border_strong),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
    let mut changed = false;
    for (i, (delta, icon)) in [(-1i32, "remove"), (1i32, "add")].into_iter().enumerate() {
        let b = if i == 0 {
            Rect::from_min_size(inner.min, vec2(36.0, 38.0))
        } else {
            Rect::from_min_size(pos2(inner.max.x - 36.0, inner.min.y), vec2(36.0, 38.0))
        };
        let at_limit = if delta < 0 { *value <= min } else { *value >= max };
        let rr = ui.interact(
            b,
            egui::Id::new(("stp", salt, delta)),
            Sense::click(),
        );
        if rr.hovered() && !at_limit {
            ui.painter()
                .rect_filled(b, CornerRadius::same(0), p.accent.gamma_multiply(0.10));
        }
        mi::paint_centered(
            ui.painter(),
            b.center(),
            icon,
            18.0,
            if at_limit {
                p.text_dim.gamma_multiply(0.35)
            } else {
                p.text_dim
            },
        );
        if rr.clicked() && !at_limit {
            *value = (*value + delta).clamp(min, max);
            changed = true;
        }
    }
    // Editable center value (`.stepper input`: 56px, centered, 600).
    let font = FontId::new(
        13.0,
        crate::ui::tokens::typography::semibold_family(),
    );
    let line_h =
        ui.fonts_mut(|f| f.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let text_y = cell.min.y + ((cell.height() - line_h) * 0.5).max(0.0);
    let text_rect = Rect::from_min_size(pos2(cell.min.x, text_y), vec2(56.0, line_h));
    let mut text = value.to_string();
    let vid = egui::Id::new(("stp-val", salt));
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(text_rect).scope_id(vid),
        |ui| {
            ui.add(
                TextEdit::singleline(&mut text)
                    .desired_width(56.0)
                    .font(font.clone())
                    .text_color(p.text)
                    .horizontal_align(Align::Center)
                    .frame(egui::Frame::NONE),
            );
        },
    );
    if let Ok(v) = text.parse::<i32>() {
        if v != *value {
            *value = v.clamp(min, max);
            changed = true;
        }
    }
    // Value scope moved the cursor to the text row — restore the box bottom.
    ui.advance_cursor_after_rect(rect);
    changed
}

/// `.speed-unit-switch`: 38px segmented KB/MB/GB (44px cells, 11/700).
fn unit_switch(ui: &mut egui::Ui, salt: &str, sel: &mut usize) {
    let p = mk::pal();
    let units = ["KB", "MB", "GB"];
    let (rect, _) = ui.allocate_exact_size(vec2(132.0, 38.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    for (i, u) in units.iter().enumerate() {
        let cell = Rect::from_min_size(
            pos2(rect.min.x + 44.0 * i as f32, rect.min.y),
            vec2(44.0, 38.0),
        );
        let rr = ui.interact(cell, egui::Id::new(("unit", salt, i)), Sense::click());
        let active = *sel == i;
        if active {
            ui.painter()
                .rect_filled(cell, CornerRadius::same(0), p.accent.gamma_multiply(0.12));
        } else if rr.hovered() {
            ui.painter()
                .rect_filled(cell, CornerRadius::same(0), p.accent.gamma_multiply(0.10));
        }
        if i > 0 {
            ui.painter().line_segment(
                [pos2(cell.min.x, rect.min.y), pos2(cell.min.x, rect.max.y)],
                Stroke::new(1.0, p.border_strong),
            );
        }
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            *u,
            FontId::new(11.0, crate::ui::tokens::typography::bold_family()),
            if active { p.accent_strong } else { p.text_dim },
        );
        if rr.clicked() {
            *sel = i;
        }
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
}

/// `.btn-check`: 8/16 padding, 8r, `--bg-card` + `--border-strong`, icon 18
/// + 6px gap + 13/600 label; hover → accent-strong.
fn btn_check(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool) -> egui::Response {
    let p = mk::pal();
    let font = FontId::new(13.0, crate::ui::tokens::typography::semibold_family());
    let tw = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), p.text)
        .rect
        .width();
    let (rect, rr) = ui.allocate_exact_size(vec2(16.0 + 18.0 + 6.0 + tw + 16.0, 34.0), Sense::click());
    let hover = rr.hovered() && enabled;
    let (border, tc) = if !enabled {
        (p.border_strong, p.text_dim)
    } else if hover {
        (p.accent_strong, p.accent_strong)
    } else {
        (p.border_strong, p.text)
    };
    let dim = if enabled { 1.0 } else { 0.5 };
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(8),
        p.bg_card.gamma_multiply(dim),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, border.gamma_multiply(dim)),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 16.0 + 9.0, rect.center().y),
        icon,
        18.0,
        tc,
    );
    ui.painter().text(
        pos2(rect.min.x + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        tc,
    );
    rr
}

/// `.btn-detect`: 5/10 padding, 6r, `--card` + `--border`, 11/600 dim with
/// a 16px icon (Auto Detect).
fn btn_detect(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool) -> egui::Response {
    let p = mk::pal();
    let font = FontId::new(11.0, crate::ui::tokens::typography::semibold_family());
    let tw = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), p.text_dim)
        .rect
        .width();
    let (rect, rr) = ui.allocate_exact_size(vec2(10.0 + 16.0 + 4.0 + tw + 10.0, 26.0), Sense::click());
    let hover = rr.hovered() && enabled;
    let border = if hover {
        p.accent.gamma_multiply(0.3)
    } else {
        p.border
    };
    let tc = if hover { p.accent_strong } else { p.text_dim };
    let dim = if enabled { 1.0 } else { 0.5 };
    ui.painter().rect_filled(rect, CornerRadius::same(6), p.bg_card.gamma_multiply(dim));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, border.gamma_multiply(dim)),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 10.0 + 8.0, rect.center().y),
        icon,
        16.0,
        tc,
    );
    ui.painter().text(
        pos2(rect.min.x + 10.0 + 16.0 + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        tc,
    );
    rr
}

/// `.proxy-test-result` / `.detect-result`: 8/12 rounded status row.
fn status_row(ui: &mut egui::Ui, ok: bool, fail_is_dim: bool, icon: &str, text: &str, radius: u8) {
    let p = mk::pal();
    let (bg, fg) = if ok {
        (
            Color32::from_rgba_unmultiplied(
                p.status_complete.r(),
                p.status_complete.g(),
                p.status_complete.b(),
                20,
            ),
            p.status_complete,
        )
    } else if fail_is_dim {
        (
            Color32::from_rgba_unmultiplied(
                p.text_dim.r(),
                p.text_dim.g(),
                p.text_dim.b(),
                20,
            ),
            p.text_dim,
        )
    } else {
        (
            Color32::from_rgba_unmultiplied(
                p.status_error.r(),
                p.status_error.g(),
                p.status_error.b(),
                20,
            ),
            p.status_error,
        )
    };
    let font = FontId::new(12.0, egui::FontFamily::Proportional);
    let tw = ui
        .painter()
        .layout_no_wrap(text.to_string(), font.clone(), fg)
        .rect
        .width();
    let (rect, _) = ui.allocate_exact_size(vec2((tw + 12.0 + 6.0 + 16.0 + 24.0).min(ui.available_width()), 32.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(radius), bg);
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 12.0 + 8.0, rect.center().y),
        icon,
        16.0,
        fg,
    );
    ui.painter().text(
        pos2(rect.min.x + 12.0 + 16.0 + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        fg,
    );
}

// ── Aria2c ────────────────────────────────────────────────────────────

/// Paint a Material glyph rotated about its center (the spinning refresh icon).
fn rotated_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    name: &str,
    size: f32,
    color: Color32,
    angle: f32,
) {
    let galley = painter.layout_no_wrap(mi::glyph(name).to_string(), mi::font(size), color);
    let half = galley.size() * 0.5;
    let (s, c) = angle.sin_cos();
    let off = vec2(half.x * c - half.y * s, half.x * s + half.y * c);
    let mut shape = egui::epaint::TextShape::new(center - off, galley, color);
    shape.angle = angle;
    painter.add(shape);
}

/// Start a status probe now (used by both the 5s poll and the refresh button).
fn aria2_probe(state: &mut UiState, ctx: &Ctx) {
    state.aria2_fetching = true;
    state.aria2_next_poll = std::time::Instant::now() + std::time::Duration::from_secs(5);
    ctx.egui_ctx.request_repaint();
    ctx.emit("settings-aria2", |_app| async move {
        moonhunt_core::aria2::get_aria2_version().await
    });
}

/// `.aria2-live-pill`: liquid-glass status pill (dot + state icon + label +
/// version chip + refresh). The dot breathes while the engine runs, the border
/// and refresh glyph pulse/spin while a probe is in flight, and a click
/// anywhere on the pill (or the refresh glyph) re-checks immediately.
/// Returns `true` when a re-check was requested.
fn aria2_pill(state: &mut UiState, ui: &mut egui::Ui) -> bool {
    let p = mk::pal();
    let (kind, label, icon) = match &state.aria2_status {
        Some(Ok(_)) => (0, state.t("Aria2c Running"), "check_circle"),
        Some(Err(_)) => (1, state.t("Aria2c is not running"), "error"),
        None => (2, state.t("Checking Aria2c…"), "hourglass_empty"),
    };
    let version = state.aria2_version.clone();
    let checking = state.aria2_fetching;
    let time = ui.input(|i| i.time) as f32;
    // 0..1 breath for the dot / border.
    let pulse = match kind {
        0 => (time * 2.2).sin() * 0.5 + 0.5,
        2 => (time * 3.4).sin() * 0.5 + 0.5,
        _ => 0.0,
    };
    let (fill, mut border, fg) = match kind {
        0 => (
            p.accent.gamma_multiply(0.15),
            p.accent.gamma_multiply(0.45),
            p.accent_strong,
        ),
        1 => (
            p.status_error.gamma_multiply(0.15),
            p.status_error.gamma_multiply(0.5),
            p.status_error,
        ),
        _ => (
            mk::blend(p.bg_card, Color32::from_rgb(142, 158, 178), 0.10),
            p.border_strong,
            p.text_dim,
        ),
    };
    if checking {
        border = border.gamma_multiply(0.55 + 0.45 * pulse);
    }
    let font = FontId::new(13.0, crate::ui::tokens::typography::bold_family());
    let label_w = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), fg)
        .rect
        .width();
    let ver_w = if version.is_empty() {
        0.0
    } else {
        let vw = ui
            .painter()
            .layout_no_wrap(version.clone(), FontId::new(11.0, crate::ui::tokens::typography::semibold_family()), fg)
            .rect
            .width();
        vw + 16.0 + 8.0
    };
    // padding 16 + dot 8 + gap 8 + icon 18 + gap 8 + label + [chip] + gap 8 + refresh 15 + padding 16
    let w = 16.0 + 8.0 + 8.0 + 18.0 + 8.0 + label_w + ver_w + 8.0 + 15.0 + 16.0;
    let (rect, rr) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    if rr.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().rect_filled(rect, CornerRadius::same(17), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(17),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let cy = rect.center().y;
    let mut x = rect.min.x + 16.0;
    // Breathing status dot: a soft halo expands/fades while running or probing.
    let dot = pos2(x + 4.0, cy);
    if kind != 1 {
        let halo = 4.0 + 5.0 * pulse;
        let a = (0.38 * (1.0 - pulse) * 255.0) as u8;
        ui.painter()
            .circle_filled(dot, halo, Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), a));
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    ui.painter().circle_filled(dot, 4.0, fg);
    x += 8.0 + 8.0;
    mi::paint_centered(ui.painter(), pos2(x + 9.0, cy), icon, 18.0, fg);
    x += 18.0 + 8.0;
    ui.painter()
        .text(pos2(x, cy), Align2::LEFT_CENTER, label, font.clone(), fg);
    x += label_w;
    if !version.is_empty() {
        let chip = Rect::from_min_size(pos2(x, cy - 11.0), vec2(ui.painter().layout_no_wrap(version.clone(), FontId::new(11.0, crate::ui::tokens::typography::semibold_family()), fg).rect.width() + 16.0, 22.0));
        ui.painter()
            .rect_filled(chip, CornerRadius::same(11), Color32::from_black_alpha(51));
        ui.painter().text(
            pos2(chip.center().x, cy),
            Align2::CENTER_CENTER,
            version,
            FontId::new(11.0, crate::ui::tokens::typography::semibold_family()),
            fg.gamma_multiply(0.75),
        );
        x += ver_w;
    }
    x += 8.0;
    // Refresh glyph: its own hit region so it hovers/clicks reliably.
    let refresh_center = pos2(x + 7.5, cy);
    let refresh_rect = Rect::from_center_size(refresh_center, vec2(20.0, 20.0));
    let refresh = ui.interact(
        refresh_rect,
        ui.make_persistent_id("aria2-refresh"),
        Sense::click(),
    );
    let hot = refresh.hovered();
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter()
            .circle_filled(refresh_center, 11.0, Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), 28));
    }
    // Spin fast while checking, slowly on hover, still otherwise.
    let angle = if checking {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        time * std::f32::consts::TAU * 0.9
    } else if hot {
        time * std::f32::consts::TAU * 0.15
    } else {
        0.0
    };
    let refresh_fg = fg.gamma_multiply(if hot || checking { 1.0 } else { 0.6 });
    rotated_icon(ui.painter(), refresh_center, "refresh", 15.0, refresh_fg, angle);

    (rr.clicked() || refresh.clicked()) && !checking
}

fn aria2c_section(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    if state.aria2_bin_path.is_empty() {
        if let Ok(pt) = moonhunt_core::aria2::get_aria2_bin_path() {
            state.aria2_bin_path = pt;
        }
    }
    // `.watch(activeCategory === 'aria2c')` → live status polling (5s).
    if !state.aria2_fetching && std::time::Instant::now() >= state.aria2_next_poll {
        aria2_probe(state, ctx);
    }

    section_head(ui, state.t("Aria2c"), state.t("Advanced aria2 download engine settings."));

    field_label(ui, state.t("Engine Status"));
    if aria2_pill(state, ui) {
        aria2_probe(state, ctx);
    }
    if let Some(Err(e)) = &state.aria2_status {
        mk::gap(ui, 6.0);
        ui.label(RichText::new(e).size(11.0).color(p.status_error));
    }
    field_gap(ui);

    // Installed location: readonly path + folder/copy actions inside.
    field_label(ui, state.t("Installed Location"));
    let mut path = state.aria2_bin_path.clone();
    let act = nd_field(
        ui,
        "aria2-path",
        w,
        &mut path,
        state.t("Locating aria2c…"),
        None,
        &["folder_open", "content_copy"],
        false,
        false,
    );
    match act {
        Some("folder_open") => {
            ctx.run(move |_| async move { moonhunt_core::aria2::open_aria2_location() });
        }
        Some("content_copy") => {
            let pth = state.aria2_bin_path.clone();
            ui.ctx().copy_text(pth);
            state.toast(true, "Path copied");
        }
        _ => {}
    }
    field_gap(ui);

    // Engine controls (`.aria2-controls`): uniform btn-check row.
    field_label(ui, state.t("Engine Controls"));
    let running = matches!(state.aria2_status, Some(Ok(_)));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        if btn_check(ui, "play_arrow", state.t("Start"), !running).clicked() {
            ctx.run(move |app| async move {
                moonhunt_core::aria2::start_aria2(app)
                    .await
                    .map(|m| format!("Started: {m}"))
            });
        }
        if btn_check(ui, "restart_alt", state.t("Restart"), true).clicked() {
            ctx.run(move |app| async move { moonhunt_core::aria2::restart_aria2(app).await });
        }
        if btn_check(ui, "power_settings_new", state.t("Shutdown"), running).clicked() {
            ctx.run(move |_| async move { moonhunt_core::aria2::shutdown_aria2().await });
        }
        if btn_check(ui, "update", state.t("Check for Update"), true).clicked() {
            ctx.run(move |_| async move {
                moonhunt_core::aria2::check_aria2_update().await.map(|v| {
                    let latest = v
                        .get("latestVersion")
                        .and_then(|x| x.as_str())
                        .unwrap_or("?");
                    format!("aria2 update check → latest {latest}")
                })
            });
        }
    });
    field_gap(ui);

    // RPC configuration sub-heading (`.section-divider` + title + sub).
    inline_divider(ui);
    section_head(ui, state.t("RPC Configuration"), state.t("aria2 RPC server settings."));
    field_label(ui, state.t("RPC Port"));
    let old = state.settings.rpc_port.to_string();
    let mut port_s = old.clone();
    let act = nd_field(
        ui,
        "rpc-port",
        w,
        &mut port_s,
        "",
        None,
        &["content_copy"],
        false,
        true,
    );
    if port_s != old {
        if let Ok(v) = port_s.parse::<u32>() {
            if (1..=65_535).contains(&v) {
                state.settings.rpc_port = v;
                state.settings_dirty = true;
            }
        }
    }
    if act == Some("content_copy") {
        let c = state.settings.rpc_port.to_string();
        ui.ctx().copy_text(c);
        state.toast(true, "RPC port copied");
    }
    end_section(ui);
}

// ── URL ───────────────────────────────────────────────────────────────

fn url_section(state: &mut UiState, _ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(
        ui,
        state.t("URL Downloads"),
        state.t("Where URL-based files land and how they download."),
    );

    // Download location: `.nd-input-wrap` + folder action.
    field_label(ui, state.t("Download Location"));
    let old = state.settings.download_location.clone();
    let mut loc = old.clone();
    if nd_field(
        ui,
        "url-loc",
        w,
        &mut loc,
        state.t("Default download location"),
        None,
        &["folder_open"],
        false,
        true,
    ) == Some("folder_open")
        || (loc != old && loc.is_empty())
    {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            state.settings.download_location = dir.display().to_string();
            state.settings_dirty = true;
        }
    } else if loc != old {
        state.settings.download_location = loc;
        state.settings_dirty = true;
    }
    field_gap(ui);

    // Max concurrent downloads (reference range 1..10).
    field_label(ui, state.t("Max Concurrent Downloads"));
    let mut mc = state.settings.max_concurrent as i32;
    if stepper(ui, "url-mc", &mut mc, 1, 10) {
        state.settings.max_concurrent = mc.clamp(1, 10) as u32;
        state.settings_dirty = true;
    }
    field_gap(ui);

    // Retry settings group heading + hint inside `.field`.
    field_label(ui, state.t("Retry Settings"));
    hint(ui, state.t("How aria2 retries on unstable connections."));
    mk::gap(ui, 16.0);

    // `.retry-row`: 3 equal columns, Title-case labels (not `.field label`).
    let mut retry_changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(16.0, 0.0);
        let col_w = (ui.available_width() - 32.0) / 3.0;
        // Translated before the `&mut state.settings` borrows below.
        const RETRY_KEYS: [&str; 3] = ["Max Tries", "Retry per URI", "Retry Wait (sec)"];
        let retry_names = [
            state.t(RETRY_KEYS[0]),
            state.t(RETRY_KEYS[1]),
            state.t(RETRY_KEYS[2]),
        ];
        for (i, (val, lo, hi)) in [
            (&mut state.settings.max_retries, 1i32, 999i32),
            (&mut state.settings.retry_per_uri, 1, 999),
            (&mut state.settings.retry_wait, 1, 120),
        ]
        .into_iter()
        .enumerate()
        {
            let label = retry_names[i];
            let _ = i;
            ui.vertical(|ui| {
                ui.set_width(col_w);
                // `.retry-row .retry-field label`: 12px muted, mb 4.
                ui.label(
                    RichText::new(label)
                        .size(12.0)
                        .color(p.text_muted),
                );
                mk::gap(ui, 4.0);
                let mut v = *val as i32;
                if stepper(ui, label, &mut v, lo, hi) {
                    *val = v.clamp(lo, hi) as u32;
                    retry_changed = true;
                }
            });
        }
    });
    if retry_changed {
        state.settings_dirty = true;
    }
    mk::gap(ui, 10.0);

    // Enable Download limit (`.toggle-header-row`).
    let dl_en = state.settings.download_limit_enabled;
    row_field(ui, state.t("Enable Download Limit"), |ui| {
        if mk::switch(ui, dl_en, "st-dl-limit").clicked() {
            let en = !dl_en;
            state.settings.download_limit_enabled = en;
            // Old `toggleDlLimit`: enabling starts the counter fresh so a
            // stale used-value can't fire the alert on the spot, and an
            // unset limit defaults to 1 MB (what the field already shows).
            if en {
                if state.settings.download_limit_bytes == 0 {
                    state.settings.download_limit_bytes = 1024 * 1024;
                }
                state.settings.download_limit_used_bytes = 0;
                state.limit_reached = false;
            }
            state.settings_dirty = true;
        }
    });
    mk::gap(ui, 2.0);

    if dl_en {
        // `.field` LIMIT: number input + unit dropdown + "Equivalent to" hint.
        field_label(ui, state.t("Limit"));
        let unit = ["KB", "MB", "GB"][state.st_dl_unit.min(2)];
        let mult: u64 = match unit {
            "KB" => 1024,
            "MB" => 1024 * 1024,
            _ => 1024 * 1024 * 1024,
        };
        let mut amount = (state.settings.download_limit_bytes / mult).max(1);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            let in_w = ui.available_width() - 72.0 - 8.0;
            let old = amount.to_string();
            let mut s = old.clone();
            nd_field(ui, "dl-limit", in_w, &mut s, "e.g. 1", None, &[], false, true);
            if s != old {
                if let Ok(v) = s.parse::<u64>() {
                    if v >= 1 {
                        amount = v;
                        state.settings.download_limit_bytes = amount * mult;
                        state.settings_dirty = true;
                    }
                }
            }
            unit_dropdown(ui, "dl-unit", &mut state.st_dl_unit);
        });
        mk::gap(ui, 6.0);
        let equiv = fmt_limit(state.settings.download_limit_bytes);
        ui.label(
            RichText::new(state.t_of(&format!("{} {equiv}", state.raw("Equivalent to"))))
                .size(11.0)
                .color(p.text_dim),
        );
        mk::gap(ui, 16.0);

        // Usage bar (`.limit-usage-*`).
        field_label(ui, state.t("Usage"));
        let bytes = state.settings.download_limit_bytes.max(1);
        let frac = (state.settings.download_limit_used_bytes as f64 / bytes as f64) as f32;
        let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
        ui.painter().rect_filled(track, CornerRadius::same(3), Color32::from_white_alpha(15));
        ui.painter().rect_filled(
            Rect::from_min_size(track.min, vec2(track.width() * frac.clamp(0.0, 1.0), 6.0)),
            CornerRadius::same(3),
            p.accent,
        );
        mk::gap(ui, 6.0);
        ui.label(
            RichText::new(format!(
                "{} / {}",
                fmt_limit(state.settings.download_limit_used_bytes),
                equiv
            ))
            .size(12.0)
            .color(p.text_dim),
        );
    }
    end_section(ui);
}

/// `.dl-limit-unit-custom`: 72×38 trigger + popup list (KB/MB/GB).
fn unit_dropdown(ui: &mut egui::Ui, salt: &str, sel: &mut usize) {
    let p = mk::pal();
    let units = ["KB", "MB", "GB"];
    let (rect, _) = ui.allocate_exact_size(vec2(72.0, 36.0), Sense::hover());
    let open = ui.data(|d| d.get_temp::<bool>(egui::Id::new(("udd", salt)))).unwrap_or(false);
    ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );
    ui.painter().text(
        pos2(rect.min.x + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        units[(*sel).min(2)],
        FontId::new(13.0, crate::ui::tokens::typography::semibold_family()),
        p.text,
    );
    mi::paint_centered(
        ui.painter(),
        pos2(rect.max.x - 14.0, rect.center().y),
        if open { "expand_less" } else { "expand_more" },
        18.0,
        p.text_dim,
    );
    let rr = ui.interact(rect, egui::Id::new(("udd-trg", salt)), Sense::click());
    if rr.clicked() {
        ui.data_mut(|d| d.insert_temp(egui::Id::new(("udd", salt)), !open));
    }
    if open {
        let mut picked: Option<usize> = None;
        egui::Area::new(egui::Id::new(("udd-pop", salt)))
            .order(egui::Order::Foreground)
            .fixed_pos([rect.min.x, rect.max.y + 4.0])
            .show(ui.ctx(), |ui| {
                ui.set_width(rect.width());
                egui::Frame::new()
                    .fill(p.bg_card)
                    .stroke(Stroke::new(1.0, p.border_strong))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        for (i, u) in units.iter().enumerate() {
                            let (r, _) =
                                ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
                            let ir = ui.interact(
                                r,
                                egui::Id::new(("udd-opt", salt, i)),
                                Sense::click(),
                            );
                            let active = *sel == i;
                            if ir.hovered() || active {
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
                                pos2(r.min.x + 8.0, r.center().y),
                                Align2::LEFT_CENTER,
                                *u,
                                FontId::new(13.0, egui::FontFamily::Proportional),
                                if active { p.accent_strong } else { p.text },
                            );
                            if ir.clicked() {
                                picked = Some(i);
                            }
                        }
                    });
            });
        if let Some(i) = picked {
            *sel = i;
            ui.data_mut(|d| d.insert_temp(egui::Id::new(("udd", salt)), false));
        } else if ui.input(|i| i.pointer.any_pressed()) {
            // Outside click closes (CustomDropdown `handleClickOutside`); the
            // popup occupies a deterministic rect below the trigger.
            let pop = Rect::from_min_size(
                pos2(rect.min.x, rect.max.y + 4.0),
                vec2(72.0, 98.0),
            );
            let over_other = ui.input(|i| i.pointer.latest_pos()).is_some_and(|pt| {
                !pop.contains(pt) && !rect.contains(pt)
            });
            if over_other {
                ui.data_mut(|d| d.insert_temp(egui::Id::new(("udd", salt)), false));
            }
        }
    }
}

/// `formatLimitDisplay` — largest whole-ish unit (GB > MB), "0 B" fallback.
fn fmt_limit(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    if bytes >= 1024 * 1024 * 1024 {
        let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        return if gb.fract() == 0.0 {
            format!("{} GB", gb as u64)
        } else {
            format!("{gb:.2} GB")
        };
    }
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb.fract() == 0.0 {
        format!("{} MB", mb as u64)
    } else {
        format!("{mb:.0} MB")
    }
}

// ── Torrent ───────────────────────────────────────────────────────────

fn torrent_section(state: &mut UiState, _ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(ui, state.t("Torrent Downloads"), state.t("Settings for BitTorrent downloads."));

    // Location row: input (+folder) + follow/custom toggle fitted right.
    field_label(ui, state.t("Torrent Download Location"));
    let custom = state.settings.torrent_download_location.is_some();
    let url_loc = state.settings.download_location.clone();
    let ph = if custom {
        state.t("Default torrent download location")
    } else if !url_loc.is_empty() {
        url_loc.as_str()
    } else {
        state.t("Follows URL Downloads location")
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
        let in_w = ui.available_width() - 42.0 - 10.0;
        let mut loc = state
            .settings
            .torrent_download_location
            .clone()
            .unwrap_or_default();
        let act = nd_field(
            ui,
            "torrent-loc",
            in_w,
            &mut loc,
            ph,
            None,
            &["folder_open"],
            false,
            custom,
        );
        if act == Some("folder_open") && custom {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                state.settings.torrent_download_location = Some(dir.display().to_string());
                state.settings_dirty = true;
            }
        } else if custom && loc != state.settings.torrent_download_location.clone().unwrap_or_default() {
            state.settings.torrent_download_location = Some(loc);
            state.settings_dirty = true;
        }
        if mk::switch(ui, custom, "st-torrent-loc").clicked() {
            state.settings.torrent_download_location = if custom {
                None
            } else {
                Some(state.settings.download_location.clone())
            };
            state.settings_dirty = true;
        }
    });
    if !custom {
        mk::gap(ui, 6.0);
        hint(ui, state.t("Off — torrents follow the URL Downloads location."));
    }
    field_gap(ui);

    // Max connections per torrent (reference range 1..128).
    field_label(ui, state.t("Max Connections Per Torrent"));
    let mut mc = state
        .settings
        .torrent_max_connections
        .unwrap_or(55) as i32;
    if stepper(ui, "torrent-mc", &mut mc, 1, 128) {
        state.settings.torrent_max_connections = Some(mc.clamp(1, 128) as u32);
        state.settings_dirty = true;
    }
    field_gap(ui);

    let dht = state.settings.torrent_dht_enabled.unwrap_or(true);
    row_field(ui, state.t("Enable DHT"), |ui| {
        if mk::switch(ui, dht, "st-dht").clicked() {
            state.settings.torrent_dht_enabled = Some(!dht);
            state.settings_dirty = true;
        }
    });
    field_gap(ui);

    let auto = state.settings.torrent_auto_start.unwrap_or(true);
    row_field(ui, state.t("Auto-Start Torrents"), |ui| {
        if mk::switch(ui, auto, "st-autostart").clicked() {
            state.settings.torrent_auto_start = Some(!auto);
            state.settings_dirty = true;
        }
    });
    field_gap(ui);

    // Max upload speed (`.global-speed-limit` row, mb 2).
    let up_en = state.settings.torrent_upload_limit_enabled.unwrap_or(false);
    row_field(ui, state.t("Max Upload Speed"), |ui| {
        if mk::switch(ui, up_en, "st-up-limit").clicked() {
            state.settings.torrent_upload_limit_enabled = Some(!up_en);
            state.settings_dirty = true;
        }
    });
    mk::gap(ui, 2.0);
    if up_en {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
            let mut v = state.settings.torrent_upload_limit.unwrap_or(50) as i32;
            if stepper(ui, "torrent-up", &mut v, 1, 1_000_000) {
                state.settings.torrent_upload_limit = Some(v.max(1) as u32);
                state.settings_dirty = true;
            }
            unit_switch(ui, "torrent-up-unit", &mut state.st_upload_unit);
        });
    }
    mk::gap(ui, 6.0);
    hint(ui, state.t("This limit applies to all torrent uploads at once."));
    end_section(ui);
    let _ = p;
}

// ── Network (proxy) ───────────────────────────────────────────────────

fn network_section(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(
        ui,
        state.t("Proxy Configuration"),
        state.t("Route traffic through a proxy server to bypass ISP restrictions."),
    );

    let enabled = state.settings.proxy_type != "none";
    let detecting = state.st_detecting;
    row_field(
        ui,
        state.t("Enable Proxy"),
        |ui| {
            // Right group: `[Auto Detect][toggle]` with an 8px gap — the
            // right-to-left layout places the first widget rightmost.
            ui.spacing_mut().item_spacing.x = 8.0;
            if mk::switch(ui, enabled, "st-proxy-sw").clicked() {
                state.settings.proxy_type = if enabled {
                    "none".into()
                } else {
                    "http".into()
                };
                state.settings_dirty = true;
            }
            let label = if detecting { state.t("Detecting…") } else { state.t("Auto Detect") };
            if btn_detect(ui, "wifi_find", label, !detecting).clicked() && !detecting {
                state.st_detecting = true;
                state.st_detect = None;
                ctx.emit("settings-detect", |_app| async move {
                    moonhunt_core::aria2::detect_system_proxy().await
                });
            }
        },
    );

    if let Some((ok, msg)) = state.st_detect.clone() {
        mk::gap(ui, 4.0);
        let msg = state.t_of(&msg);
        status_row(
            ui,
            ok,
            false,
            if ok { "check_circle" } else { "info_outline" },
            &msg,
            6,
        );
        mk::gap(ui, 12.0);
    }

    if enabled {
        // Type: HTTP or SOCKS5. `build_proxy_url`/aria2 already understand
        // `socks5`, but nothing in the UI could set it, so a SOCKS tunnel
        // (hysteria, ssh -D) was unreachable from Settings. Laid out like the
        // Host/Port fields (label above) so the row is not a short one-liner
        // wedged under the Enable Proxy switch.
        mk::gap(ui, 28.0);
        field_label(ui, state.t("Type"));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for (value, label) in [("http", "HTTP"), ("socks5", "SOCKS5")] {
                let on = state.settings.proxy_type == value;
                let icon = if on { "check_circle" } else { "radio_button_unchecked" };
                if btn_check(ui, icon, label, true).clicked() && !on {
                    state.settings.proxy_type = value.to_string();
                    state.settings_dirty = true;
                }
            }
        });
        mk::gap(ui, 24.0);

        // `.field-row`: Host + Port side by side (flex 1, gap 12).
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
            let half = (ui.available_width() - 12.0) / 2.0;
            ui.vertical(|ui| {
                field_label(ui, state.t("Host"));
                let old = state.settings.proxy_host.clone();
                let mut h = old.clone();
                nd_field(
                    ui,
                    "px-host",
                    half,
                    &mut h,
                    state.t("proxy.example.com"),
                    None,
                    &[],
                    false,
                    true,
                );
                if h != old {
                    state.settings.proxy_host = h;
                    state.settings_dirty = true;
                }
            });
            ui.vertical(|ui| {
                field_label(ui, state.t("Port"));
                let old = state.settings.proxy_port.to_string();
                let mut pt = old.clone();
                nd_field(ui, "px-port", half, &mut pt, "1080", None, &[], false, true);
                if pt != old {
                    if let Ok(v) = pt.parse::<u32>() {
                        state.settings.proxy_port = v.min(65_535);
                        state.settings_dirty = true;
                    }
                }
            });
        });
        mk::gap(ui, 16.0);

        // Username + Password (`.optional` suffix stays dim inside the label).
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
            let half = (ui.available_width() - 12.0) / 2.0;
            ui.vertical(|ui| {
                opt_field_label(ui, state.t("Username"));
                let old = state.settings.proxy_user.clone();
                let mut v = old.clone();
                nd_field(ui, "px-user", half, &mut v, state.t("Leave blank if none"), None, &[], false, true);
                if v != old {
                    state.settings.proxy_user = v;
                    state.settings_dirty = true;
                }
            });
            ui.vertical(|ui| {
                opt_field_label(ui, state.t("Password"));
                let old = state.settings.proxy_pass.clone();
                let mut v = old.clone();
                nd_field(ui, "px-pass", half, &mut v, state.t("Leave blank if none"), None, &[], true, true);
                if v != old {
                    state.settings.proxy_pass = v;
                    state.settings_dirty = true;
                }
            });
        });

        // `.proxy-checker`: test button + result row.
        mk::gap(ui, 16.0);
        let testing = state.st_testing;
        let last_ok = state.st_test.as_ref().map(|(ok, _)| *ok);
        let label = match (testing, last_ok) {
            (true, _) => state.t("Testing…"),
            (false, Some(true)) => state.t("Re-test Connection"),
            (false, Some(false)) => state.t("Retry Connection"),
            (false, None) => state.t("Test Connection"),
        };
        let testing_yt = state.st_testing_yt;
        let last_yt = state.st_test_yt.as_ref().map(|(ok, _)| *ok);
        let label_yt = match (testing_yt, last_yt) {
            (true, _) => state.t("Testing…"),
            (false, Some(true)) => state.t("YouTube OK"),
            (false, Some(false)) => state.t("Retry YouTube"),
            (false, None) => state.t("Test YouTube"),
        };
        let clicked = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let a = btn_check(
                    ui,
                    match last_ok {
                        Some(true) => "check_circle",
                        Some(false) => "error",
                        None => "wifi_find",
                    },
                    label,
                    !testing,
                )
                .clicked()
                    && !testing;
                let b = btn_check(
                    ui,
                    match last_yt {
                        Some(true) => "check_circle",
                        Some(false) => "error",
                        None => "public",
                    },
                    label_yt,
                    !testing_yt,
                )
                .clicked()
                    && !testing_yt;
                (a, b)
            })
            .inner;
        if clicked.1 {
            state.st_testing_yt = true;
            state.st_test_yt = None;
            ctx.emit("settings-test-yt", |app| async move {
                moonhunt_core::aria2::test_proxy_url(
                    app,
                    "https://www.youtube.com/".to_string(),
                    None,
                )
                .await
            });
        }
        if clicked.0 {
            state.st_testing = true;
            state.st_test = None;
            let (ptype, host, port, user, pass) = (
                state.settings.proxy_type.clone(),
                state.settings.proxy_host.clone(),
                state.settings.proxy_port,
                state.settings.proxy_user.clone(),
                state.settings.proxy_pass.clone(),
            );
            ctx.emit("settings-test", move |_app| async move {
                moonhunt_core::aria2::test_proxy(
                    ptype,
                    host,
                    port,
                    Some(user),
                    Some(pass),
                )
                .await
            });
        }
        if let Some((ok, msg)) = state.st_test.clone() {
            mk::gap(ui, 8.0);
            let msg = state.t_of(&msg);
            status_row(ui, ok, false, if ok { "check_circle" } else { "error" }, &msg, 8);
        }
        if let Some((ok, msg)) = state.st_test_yt.clone() {
            mk::gap(ui, 8.0);
            let msg = state.t_of(&msg);
            status_row(ui, ok, false, if ok { "check_circle" } else { "error" }, &msg, 8);
        }
    } else {
        // `.proxy-disabled-hint`: info strip.
        mk::gap(ui, 4.0);
        let text = state.t("Proxy is disabled. Enable it if your ISP blocks certain sites.");
        let font = FontId::new(12.0, egui::FontFamily::Proportional);
        let tw = ui
            .painter()
            .layout_no_wrap(text.to_string(), font.clone(), p.text_dim)
            .rect
            .width();
        let (rect, _) = ui.allocate_exact_size(
            vec2((tw + 14.0 + 8.0 + 16.0 + 28.0).min(ui.available_width()), 42.0),
            Sense::hover(),
        );
        ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover_veil);
        mi::paint_centered(
            ui.painter(),
            pos2(rect.min.x + 14.0 + 8.0, rect.center().y),
            "info_outline",
            16.0,
            p.text_dim,
        );
        ui.painter().text(
            pos2(rect.min.x + 14.0 + 16.0 + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            font,
            p.text_dim,
        );
    }
    end_section(ui);
}

/// `.field label` + `.optional` "(…)" suffix (11px, weight 400, dim).
fn opt_field_label(ui: &mut egui::Ui, text: &str) {
    let p = mk::pal();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
        ui.label(
            RichText::new(text.to_uppercase())
                .font(FontId::new(
                    12.0,
                    crate::ui::tokens::typography::semibold_family(),
                ))
                .color(p.text_muted),
        );
        ui.label(
            RichText::new("(OPTIONAL)")
                .size(11.0)
                .font(FontId::new(12.0, egui::FontFamily::Proportional))
                .color(p.text_dim),
        );
    });
    mk::gap(ui, 6.0);
}

// ── Browser extension ─────────────────────────────────────────────────

fn extension_section(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(ui, state.t("Browser Extension"), state.t("Connect MoonHunt to your browser."));

    // Extension API port: number input + copy action.
    field_label(ui, state.t("Extension API Port"));
    let old = state.settings.extension_port.to_string();
    let mut ep = old.clone();
    let act = nd_field(
        ui,
        "ext-port",
        w,
        &mut ep,
        "",
        None,
        &["content_copy"],
        false,
        true,
    );
    if ep != old {
        if let Ok(v) = ep.parse::<u32>() {
            if (1..=65_535).contains(&v) {
                state.settings.extension_port = v;
                state.settings_dirty = true;
            }
        }
    }
    if act == Some("content_copy") {
        let c = state.settings.extension_port.to_string();
        ui.ctx().copy_text(c);
        state.toast(true, "Port copied");
    }
    field_gap(ui);

    // Auth token row: label + lamp tooltip left, toggle right.
    let auth_en = state.settings.extension_auth_enabled;
    let optional = if auth_en {
        state.t("(REQUIRED WHEN ENABLED)")
    } else {
        state.t("(OPTIONAL)")
    };
    // Hoisted: the left closure must not capture `state`, the right one needs it.
    let auth_label = state.t("AUTH TOKEN ");
    row_field_custom(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 0.0);
                ui.label(
                    RichText::new(auth_label)
                        .font(FontId::new(
                            12.0,
                            crate::ui::tokens::typography::semibold_family(),
                        ))
                        .color(p.text_muted),
                );
                ui.label(
                    RichText::new(optional)
                        .size(11.0)
                        .font(FontId::new(12.0, egui::FontFamily::Proportional))
                        .color(p.text_dim),
                );
                let (lr, lrr) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                mi::paint_centered(
                    ui.painter(),
                    lr.center(),
                    "lightbulb",
                    16.0,
                    if lrr.hovered() { p.accent_strong } else { p.text_dim },
                );
                lrr.on_hover_text(
                    "Require a shared token so only your extension can connect. \
                     When enabled, requests without a matching token are rejected.",
                );
            });
        },
        |ui| {
            if mk::switch(ui, auth_en, "st-ext-auth").clicked() {
                state.settings.extension_auth_enabled = !auth_en;
                state.settings_dirty = true;
            }
        },
    );
    field_gap(ui);

    // Token input: lead generate + secret text + eye + copy.
    let mut tok = state.settings.extension_auth_token.clone();
    let secret = !state.st_token_visible;
    let act = nd_field(
        ui,
        "ext-token",
        w,
        &mut tok,
        state.t("Enter a secret token"),
        Some("auto_fix_high"),
        &["visibility", "content_copy"],
        secret,
        true,
    );
    match act {
        Some("auto_fix_high") => {
            let new = uuid_like_token();
            ui.ctx().copy_text(new.clone());
            state.settings.extension_auth_token = new;
            state.settings_dirty = true;
            state.toast(true, "New token generated + copied");
        }
        Some("visibility") => {
            state.st_token_visible = !state.st_token_visible;
        }
        Some("content_copy") => {
            let t = state.settings.extension_auth_token.clone();
            ui.ctx().copy_text(t);
            state.toast(true, "Token copied");
        }
        _ => {}
    }
    if tok != state.settings.extension_auth_token {
        state.settings.extension_auth_token = tok;
        state.settings_dirty = true;
    }
    field_gap(ui);

    // `.info-box` setup card with the two tutorial pills.
    info_box(
        ui,
        state.t("Browser Extension Setup"),
        &[
            (state.t("Chrome Tutorial"), "chrome_tutorial"),
            (state.t("Firefox Tutorial"), "firefox_tutorial"),
        ],
        ctx,
    );
    mk::gap(ui, 14.0);

    // Open MoonHunt from the browser (label + hint left, toggle right).
    let proto = state.settings.open_in_browser_protocol_enabled;
    // Hoisted: the left closure would otherwise capture `state` and the right
    // one needs unique access to `state.settings`.
    let proto_hint = state.t("Lets the extension wake MoonHunt when it is closed (moonhunt://).");
    let browser_label = state.t("OPEN MOONHUNT FROM THE BROWSER");
    row_field_custom(
        ui,
        |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(browser_label)
                        .font(FontId::new(
                            12.0,
                            crate::ui::tokens::typography::semibold_family(),
                        ))
                        .color(p.text_muted),
                );
                mk::gap(ui, 6.0);
                ui.label(
                    RichText::new(proto_hint)
                        .size(11.0)
                        .color(p.text_dim),
                );
            });
        },
        |ui| {
            if mk::switch(ui, proto, "st-protocol").clicked() {
                state.settings.open_in_browser_protocol_enabled = !proto;
                state.settings_dirty = true;
            }
        },
    );
    end_section(ui);
}

/// `.info-box`: accent-tinted strip with an icon title and pill actions.
fn info_box(ui: &mut egui::Ui, title: &str, buttons: &[(&str, &str)], ctx: &Ctx) {
    let p = mk::pal();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
    // Height: padding 12*2 + title ~17 + gap 8 + button ~30.
    let rect = Rect::from_min_size(rect.min, vec2(rect.width(), 12.0 + 17.0 + 8.0 + 30.0 + 12.0));
    // `.info-box` is rounded only on the right: paint an 8r fill, then square
    // off the left edge with a 0r strip.
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(8),
        p.accent.gamma_multiply(0.05),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, vec2(8.0, rect.height())),
        CornerRadius::same(0),
        p.accent.gamma_multiply(0.05),
    );
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
        CornerRadius::same(0),
        p.accent.gamma_multiply(0.3),
    );
    let cy_title = rect.min.y + 12.0 + 8.0;
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 14.0 + 7.5, cy_title),
        "info",
        15.0,
        p.accent_strong,
    );
    ui.painter().text(
        pos2(rect.min.x + 14.0 + 15.0 + 6.0, cy_title),
        Align2::LEFT_CENTER,
        title,
        FontId::new(12.0, crate::ui::tokens::typography::bold_family()),
        p.accent_strong,
    );
    // Tutorial pill buttons (`.btn-tutorial`).
    let by = rect.min.y + 12.0 + 17.0 + 8.0;
    let mut bx = rect.min.x + 14.0;
    for (label, action) in buttons {
        let font = FontId::new(12.0, crate::ui::tokens::typography::bold_family());
        let tw = ui
            .painter()
            .layout_no_wrap(label.to_string(), font.clone(), p.accent_strong)
            .rect
            .width();
        let bw = 14.0 + 14.0 + 4.0 + tw + 14.0;
        let br = Rect::from_min_size(pos2(bx, by), vec2(bw, 30.0));
        let rr = ui.interact(br, egui::Id::new(("tut", label)), Sense::click());
        ui.painter().rect_filled(
            br,
            CornerRadius::same(15),
            if rr.hovered() {
                p.accent.gamma_multiply(0.2)
            } else {
                p.accent.gamma_multiply(0.1)
            },
        );
        ui.painter().rect_stroke(
            br,
            CornerRadius::same(15),
            Stroke::new(1.0, p.accent.gamma_multiply(0.25)),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(br.min.x + 14.0 + 7.0, br.center().y),
            "lightbulb",
            14.0,
            p.accent_strong,
        );
        ui.painter().text(
            pos2(br.min.x + 14.0 + 14.0 + 4.0, br.center().y),
            Align2::LEFT_CENTER,
            *label,
            font,
            p.accent_strong,
        );
        if rr.clicked() {
            let url = if *action == "chrome_tutorial" {
                "https://github.com/2sulraw/MoonHunt-DownloadManager"
            } else {
                "https://addons.mozilla.org/en-US/firefox/addon/moonhunt-download-manager/"
            }
            .to_string();
            ctx.run(move |app| async move {
                moonhunt_core::fs_ops::open_external(app, url).map(|()| String::new())
            });
        }
        bx += bw + 8.0;
    }
    // Advance the outer cursor past the painted card.
    ui.allocate_rect(rect, Sense::hover());
}

// ── Appearance ────────────────────────────────────────────────────────

fn appearance_section(state: &mut UiState, _ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(
        ui,
        state.t("App Theme"),
        state.t("Choose the overall visual style Application."),
    );

    // `.theme-cards`: 3 preview cards (flex 1, min 140, max 200).
    let cards: [(&str, &str, &str, [u8; 3], [u8; 3], [u8; 3]); 3] = [
        (
            "dark",
            state.t("Dark"),
            state.t("Deep navy backgrounds"),
            [0x0a, 0x0e, 0x1a],
            [0x1e, 0x1e, 0x32],
            [0x16, 0x16, 0x2a],
        ),
        (
            "light",
            state.t("Light"),
            state.t("Warm cream surfaces"),
            [0xf4, 0xf5, 0xf7],
            [0xff, 0xff, 0xff],
            [0xff, 0xff, 0xff],
        ),
        (
            "linear-dark",
            state.t("Linear Dark"),
            state.t("Near-black, minimal surfaces"),
            [0x08, 0x09, 0x0a],
            [0x19, 0x1a, 0x1b],
            [0x0f, 0x10, 0x11],
        ),
    ];
    let cw = ((w - 24.0) / 3.0).clamp(140.0, 200.0);
    let mode = state.settings.theme_mode.clone();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
        for (key, name, desc, bg, bar, line) in cards {
            let active = mode == key;
            let hov = ui
                .ctx()
                .pointer_interact_pos()
                .is_some_and(|m| m.x >= ui.cursor().min.x
                    && m.x <= ui.cursor().min.x + cw
                    && m.y >= ui.cursor().min.y
                    && m.y <= ui.cursor().min.y + 138.0);
            let r = Rect::from_min_size(ui.cursor().min, vec2(cw, 138.0));
            let border_c = if active {
                p.accent_strong
            } else if hov {
                blend_rgb(p.border, p.accent_strong, 0.3)
            } else {
                p.border
            };
            ui.painter().rect_filled(r, CornerRadius::same(12), p.bg_card);
            ui.painter().rect_stroke(
                r,
                CornerRadius::same(12),
                Stroke::new(2.0, border_c),
                StrokeKind::Inside,
            );
            // Preview (80px): bar + two lines.
            let pv = Rect::from_min_size(r.min + vec2(2.0, 2.0), vec2(cw - 4.0, 80.0));
            ui.painter().rect_filled(
                pv,
                CornerRadius {
                    nw: 10,
                    ne: 10,
                    se: 0,
                    sw: 0,
                },
                Color32::from_rgb(bg[0], bg[1], bg[2]),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(pv.min + vec2(12.0, 12.0), vec2((cw - 4.0) * 0.4, 8.0)),
                CornerRadius::same(4),
                Color32::from_rgb(bar[0], bar[1], bar[2]),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(pv.min + vec2(12.0, 26.0), vec2(cw - 4.0 - 24.0, 6.0)),
                CornerRadius::same(3),
                Color32::from_rgb(line[0], line[1], line[2]),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(pv.min + vec2(12.0, 36.0), vec2((cw - 4.0 - 24.0) * 0.6, 6.0)),
                CornerRadius::same(3),
                Color32::from_rgb(line[0], line[1], line[2]),
            );
            // Info block (10/12 padding): name 13/600 + desc 11 dim.
            let iy = r.min.y + 2.0 + 80.0 + 10.0;
            ui.painter().text(
                pos2(r.min.x + 12.0, iy),
                Align2::LEFT_TOP,
                name,
                FontId::new(13.0, crate::ui::tokens::typography::semibold_family()),
                p.text,
            );
            ui.painter().text(
                pos2(r.min.x + 12.0, iy + 19.0),
                Align2::LEFT_TOP,
                desc,
                FontId::new(11.0, egui::FontFamily::Proportional),
                p.text_dim,
            );
            if active {
                mi::paint_centered(
                    ui.painter(),
                    pos2(r.max.x - 8.0 - 10.0, r.min.y + 8.0 + 10.0),
                    "check_circle",
                    20.0,
                    p.accent_strong,
                );
            }
            if ui.allocate_rect(r, Sense::click()).clicked() {
                state.settings.theme_mode = key.to_string();
                state.settings_dirty = true;
            }
        }
    });
    mk::gap(ui, 24.0);

    // Accent swatches: 32px circles (`.color-swatches`).
    field_label(ui, state.t("Accent Color"));
    let cur = state.settings.theme.clone();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for preset in crate::ui::tokens::ACCENT_PRESETS.iter() {
            let active = cur.eq_ignore_ascii_case(preset.name);
            let c = Color32::from_rgb(preset.accent[0], preset.accent[1], preset.accent[2]);
            let (rect, rr) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
            if rr.hovered() {
                let s = Rect::from_center_size(rect.center(), vec2(36.0, 36.0));
                ui.painter()
                    .circle_filled(s.center(), 18.0, p.accent.gamma_multiply(0.12));
            }
            ui.painter()
                .circle_filled(rect.center(), 15.0, c);
            if active {
                ui.painter().circle_stroke(
                    rect.center(),
                    15.0,
                    Stroke::new(2.0, p.text),
                );
                ui.painter().circle_stroke(
                    rect.center(),
                    17.5,
                    Stroke::new(3.0, p.bg_primary),
                );
                mi::paint_centered(
                    ui.painter(),
                    rect.center(),
                    "check",
                    14.0,
                    Color32::WHITE,
                );
            }
            if rr.clicked() {
                state.settings.theme = preset.name.to_string();
                state.settings_dirty = true;
            }
        }
    });
    end_section(ui);
}

// ── Performance ───────────────────────────────────────────────────────

/// `.perf-mini-btn`: 24×24 bordered icon button.
fn mini_btn(ui: &mut egui::Ui, icon: &str, size: f32) -> egui::Response {
    let p = mk::pal();
    let (rect, rr) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    if rr.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent.gamma_multiply(0.06));
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(
            1.0,
            if rr.hovered() {
                p.accent.gamma_multiply(0.35)
            } else {
                p.border
            },
        ),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        rect.center(),
        icon,
        14.0,
        if rr.hovered() {
            p.accent_strong
        } else {
            p.text_dim
        },
    );
    rr
}

fn jget<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn jstr(v: &serde_json::Value, path: &str, d: &str) -> String {
    jget(v, path)
        .and_then(|x| x.as_str())
        .unwrap_or(d)
        .to_string()
}

fn jf64(v: &serde_json::Value, path: &str, d: f64) -> f64 {
    jget(v, path)
        .and_then(|x| x.as_f64())
        .unwrap_or(d)
}

/// `formatBytes` from the reference (B/KB/MB/GB, 1 decimal above B).
fn fmt_bytes(b: u64) -> String {
    if b == 0 {
        return "—".to_string();
    }
    let units = ["B", "KB", "MB", "GB"];
    let mut size = b as f64;
    let mut i = 0;
    while size >= 1024.0 && i < units.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", size.round() as i64, units[i])
    } else {
        format!("{size:.1} {}", units[i])
    }
}

fn performance_section(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    section_head(ui, state.t("Performance"), state.t("App resource usage and storage."));

    // Live stats (1.5s) + one-shot DB listing — `.watch(activeCategory)`.
    let now = std::time::Instant::now();
    if !state.perf_fetching && now >= state.perf_next_poll {
        state.perf_fetching = true;
        state.perf_next_poll = now + std::time::Duration::from_millis(1500);
        ctx.emit("settings-stats", |app| async move {
            moonhunt_core::fs_ops::get_system_stats(app).await
        });
    }
    if !state.db_fetching && state.db_folder.is_empty() && now >= state.db_next_fetch {
        state.db_fetching = true;
        state.db_next_fetch = now + std::time::Duration::from_secs(5);
        ctx.emit("settings-db", |app| async move {
            moonhunt_core::db::list_app_db_folder(app)
        });
    }

    if let Some(err) = &state.perf_error {
        let font = FontId::new(13.0, egui::FontFamily::Proportional);
        let tw = ui.painter().layout_no_wrap(err.clone(), font.clone(), Color32::WHITE).rect.width();
        let (rect, _) = ui.allocate_exact_size(
            vec2((tw + 16.0 + 8.0 + 32.0).min(w), 46.0),
            Sense::hover(),
        );
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(10),
            Color32::from_rgba_unmultiplied(0xf4, 0x43, 0x36, 20),
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xf4, 0x43, 0x36, 51)),
            StrokeKind::Inside,
        );
        mi::paint_centered(
            ui.painter(),
            pos2(rect.min.x + 16.0 + 8.0, rect.center().y),
            "error_outline",
            16.0,
            Color32::from_rgb(0xef, 0x9a, 0x9a),
        );
        ui.painter().text(
            pos2(rect.min.x + 16.0 + 16.0 + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            err.clone(),
            font,
            Color32::from_rgb(0xef, 0x9a, 0x9a),
        );
        end_section(ui);
        return;
    }

    // `.perf-grid`: 2-column stat cards.
    if let Some(stats) = state.perf_stats.clone() {
        let cell_w = (w - 12.0) / 2.0;
        let mut cells: Vec<(String, &str, String, String, f32, Option<[(&str, String); 3]>)> =
            Vec::new();
        // Memory
        cells.push((
            "memory".into(),
            state.t("Memory"),
            jstr(&stats, "memory.used_human", "—"),
            state.t_of(&format!(
                "{:.1}% {} {}",
                jf64(&stats, "memory.percent", 0.0),
                state.raw("of"),
                jstr(&stats, "memory.total_human", "—")
            )),
            (jf64(&stats, "memory.percent", 0.0) as f32 / 100.0).clamp(0.0, 1.0),
            Some([
                (state.t("System:"), jstr(&stats, "memory.system_used_human", "—")),
                (state.t("Total:"), jstr(&stats, "memory.total_human", "—")),
                ("", String::new()),
            ]),
        ));
        // CPU
        cells.push((
            "cpu".into(),
            state.t("CPU"),
            format!("{:.1}%", jf64(&stats, "cpu.usage_percent", 0.0)),
            state.t_of(&format!(
                "{:.1}% {} · {} {}",
                jf64(&stats, "cpu.process_percent", 0.0),
                state.raw("app"),
                jget(&stats, "cpu.cores").and_then(|c| c.as_u64()).unwrap_or(0),
                state.raw("cores")
            )),
            (jf64(&stats, "cpu.usage_percent", 0.0) as f32 / 100.0).clamp(0.0, 1.0),
            None,
        ));
        // Disk
        cells.push((
            "disk".into(),
            state.t("Disk Usage"),
            jstr(&stats, "disk.total_human", "—"),
            state.t_of(&format!(
                "{:.1}% {}",
                jf64(&stats, "disk.usage_percent", 0.0),
                state.raw("disk activity")
            )),
            (jf64(&stats, "disk.usage_percent", 0.0) as f32 / 100.0).clamp(0.0, 1.0),
            Some([
                (state.t("Read:"), jstr(&stats, "disk.read_human", "—")),
                (state.t("Write:"), jstr(&stats, "disk.write_human", "—")),
                (state.t("Total:"), jstr(&stats, "disk.total_human", "—")),
            ]),
        ));
        // Cache (no bar; folder/help actions).
        cells.push((
            "cache".into(),
            state.t("Cache"),
            jstr(&stats, "cache.cache_dir_human", "0 B"),
            String::new(),
            -1.0,
            Some([(state.t("Temp:"), jstr(&stats, "cache.temp_dir_human", "0 B")), ("", String::new()), ("", String::new())]),
        ));

        for row in cells.chunks(2) {
            // `.perf-grid` stretches every card to the row height, so a pair
            // with different content ends flush instead of one hanging short.
            let min_h = row
                .iter()
                .map(|c| perf_card_h(c.3.as_str(), c.4, c.5.is_some()))
                .fold(0.0_f32, f32::max);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
                for (i, (key, label, value, sub, frac, breakdown)) in row.iter().enumerate() {
                    // Half a pixel of rounding lands on the last card, so both
                    // edges stay flush instead of one card running 1px short.
                    let cw = if i + 1 == row.len() {
                        w - 12.0 - cell_w
                    } else {
                        cell_w
                    };
                    perf_card(
                        ui,
                        key,
                        label,
                        value,
                        sub,
                        *frac,
                        breakdown,
                        cw,
                        min_h,
                        state,
                        ctx,
                    );
                }
            });
            mk::gap(ui, 12.0);
        }
    }

    // Hardware acceleration row + hint.
    inline_divider(ui);
    let hw = !state.settings.disable_hardware_acceleration;
    row_field(ui, state.t("Hardware Acceleration"), |ui| {
        if mk::switch(ui, hw, "st-hwaccel").clicked() {
            let new = !hw;
            state.settings.disable_hardware_acceleration = !new;
            state.settings_dirty = true;
            ctx.run(move |app| async move {
                moonhunt_core::fs_ops::set_hardware_acceleration(app, new).await
            });
        }
    });
    mk::gap(ui, 6.0);
    hint(
        ui,
        state.t("Disable to reduce GPU memory usage (30-50 MB savings). UI animations may feel sluggish. Restart required for changes to take effect."),
    );

    // Debug logging row + hint (+ log folder action while enabled).
    inline_divider(ui);
    let dbg = state.settings.debug_logging;
    row_field(ui, state.t("Debug Logging"), |ui| {
        if mk::switch(ui, dbg, "st-debuglog").clicked() {
            let new = !dbg;
            state.settings.debug_logging = new;
            state.settings_dirty = true;
            ctx.run(move |app| async move {
                moonhunt_core::logfile::set_debug_logging(new, app)
            });
        }
        if dbg && mini_btn(ui, "folder_open", 28.0).clicked() {
            ctx.run(move |app| async move { moonhunt_core::logfile::open_log_folder(app) });
        }
    });
    mk::gap(ui, 6.0);
    hint(
        ui,
        state.t("When enabled, the app writes timestamped activity (downloads, polls, errors) to moonhunt.log in the app data folder."),
    );

    // App Database explorer.
    if !state.db_files.is_empty() {
        inline_divider(ui);
        db_explorer(state, ctx, ui, w);
    }
    end_section(ui);
    let _ = p;
}

/// One `.perf-stat` card (header, 22/800 value, sub, 4px bar, breakdown).
#[allow(clippy::too_many_arguments)]
/// Intrinsic height of a stat card — the same numbers `perf_card` lays out
/// with, so a row can be stretched to its tallest member.
fn perf_card_h(sub: &str, frac: f32, has_breakdown: bool) -> f32 {
    let sub_h = if sub.is_empty() { 0.0 } else { 10.0 + 17.0 };
    let bar_h = if frac >= 0.0 { 10.0 + 4.0 } else { 0.0 };
    let breakdown_h = if has_breakdown { 8.0 + 15.0 } else { 0.0 };
    14.0 + 17.0 + 8.0 + 28.0 + 2.0 + sub_h + bar_h + breakdown_h + 14.0
}

fn perf_card(
    ui: &mut egui::Ui,
    key: &str,
    label: &str,
    value: &str,
    sub: &str,
    frac: f32,
    breakdown: &Option<[(&str, String); 3]>,
    w: f32,
    min_h: f32,
    state: &mut UiState,
    ctx: &Ctx,
) {
    let p = mk::pal();
    let has_bar = frac >= 0.0;
    let breakdown_h = if breakdown.is_some() { 8.0 + 15.0 } else { 0.0 };
    let bar_h = if has_bar { 10.0 + 4.0 } else { 0.0 };
    let sub_h = if sub.is_empty() { 0.0 } else { 10.0 + 17.0 };
    // header 17 + 8 + value 28 + 2 + sub + bar + breakdown + padding 28
    let h = (14.0_f32 + 17.0 + 8.0 + 28.0 + 2.0 + sub_h + bar_h + breakdown_h + 14.0).max(min_h);
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(10), p.hover_veil);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let mut y = rect.min.y + 14.0;
    // Header: icon + uppercase label (+ actions for the cache card).
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 16.0 + 8.0, y + 8.5),
        match key {
            "memory" => "memory",
            "cpu" => "developer_board",
            "disk" => "storage",
            _ => "cached",
        },
        16.0,
        p.accent_strong,
    );
    ui.painter().text(
        pos2(rect.min.x + 16.0 + 16.0 + 8.0, y + 8.5),
        Align2::LEFT_CENTER,
        label.to_uppercase(),
        FontId::new(12.0, crate::ui::tokens::typography::semibold_family()),
        p.text_muted,
    );
    if key == "cache" {
        let mut bx = rect.max.x - 16.0 - 24.0;
        if mini_btn_in(ui, "help_outline", Rect::from_min_size(pos2(bx, y - 3.0), vec2(24.0, 24.0))) {
            state.st_cache_help = !state.st_cache_help;
        }
        bx -= 28.0;
        if mini_btn_in(ui, "folder_open", Rect::from_min_size(pos2(bx, y - 3.0), vec2(24.0, 24.0))) {
            ctx.run(move |app| async move { moonhunt_core::fs_ops::open_cache_folder(app).await });
        }
    }
    y += 17.0 + 8.0;
    // Value 22/800.
    ui.painter().text(
        pos2(rect.min.x + 16.0, y + 14.0),
        Align2::LEFT_CENTER,
        value,
        FontId::new(22.0, crate::ui::tokens::typography::bold_family()),
        p.text,
    );
    y += 28.0 + 2.0;
    if !sub.is_empty() {
        ui.painter().text(
            pos2(rect.min.x + 16.0, y + 8.0),
            Align2::LEFT_CENTER,
            sub,
            FontId::new(12.0, egui::FontFamily::Proportional),
            p.text_dim,
        );
        y += 17.0 + 10.0;
    }
    if has_bar {
        let track = Rect::from_min_size(
            pos2(rect.min.x + 16.0, y),
            vec2(rect.width() - 32.0, 4.0),
        );
        ui.painter()
            .rect_filled(track, CornerRadius::same(2), Color32::from_white_alpha(15));
        let fill_c = match key {
            "cpu" => p.accent,
            "disk" => Color32::from_rgb(0x42, 0xa5, 0xf5),
            _ => p.accent,
        };
        ui.painter().rect_filled(
            Rect::from_min_size(track.min, vec2(track.width() * frac, 4.0)),
            CornerRadius::same(2),
            fill_c,
        );
        y += 4.0 + 10.0;
    }
    if let Some(bd) = breakdown {
        let mut x = rect.min.x + 16.0;
        for (k, v) in bd.iter() {
            if k.is_empty() {
                continue;
            }
            let text = format!("{k} {v}");
            ui.painter().text(
                pos2(x, y + 7.0),
                Align2::LEFT_CENTER,
                text.clone(),
                FontId::new(11.0, egui::FontFamily::Proportional),
                p.text_dim,
            );
            x += ui
                .painter()
                .layout_no_wrap(text, FontId::new(11.0, egui::FontFamily::Proportional), p.text_dim)
                .rect
                .width()
                + 16.0;
        }
    }
    if key == "cache" && state.st_cache_help {
        // `.cache-help` note under the values.
        let help = state.t("Cache stores temporary yt-dlp / EJS media files and aria2 session data. Open the folder to inspect or clean it.");
        let galley = ui.fonts_mut(|f| {
            f.layout(
                help.to_string(),
                FontId::new(11.0, egui::FontFamily::Proportional),
                p.text_muted,
                rect.width() - 32.0 - 20.0,
            )
        });
        let hh = galley.size().y + 16.0;
        let hr = Rect::from_min_size(
            pos2(rect.min.x + 16.0, rect.max.y - 14.0 - hh),
            vec2(rect.width() - 32.0, hh),
        );
        ui.painter().rect_filled(
            hr,
            CornerRadius::same(8),
            p.accent.gamma_multiply(0.06),
        );
        ui.painter().rect_stroke(
            hr,
            CornerRadius::same(8),
            Stroke::new(1.0, p.accent.gamma_multiply(0.18)),
            StrokeKind::Inside,
        );
        ui.painter().galley(
            pos2(hr.min.x + 10.0, hr.min.y + 8.0),
            galley,
            p.text_muted,
        );
    }
}

/// `mini_btn` painted into an explicit rect (cards use absolute layout).
fn mini_btn_in(ui: &mut egui::Ui, icon: &str, rect: Rect) -> bool {
    let p = mk::pal();
    let rr = ui.interact(
        rect,
        egui::Id::new(("mini", icon, rect.min.x.to_bits(), rect.min.y.to_bits())),
        Sense::click(),
    );
    if rr.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), p.accent.gamma_multiply(0.06));
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(
            1.0,
            if rr.hovered() {
                p.accent.gamma_multiply(0.35)
            } else {
                p.border
            },
        ),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        rect.center(),
        icon,
        14.0,
        if rr.hovered() {
            p.accent_strong
        } else {
            p.text_dim
        },
    );
    rr.clicked()
}

/// `.db-explorer`: bordered card listing the app database folder.
fn db_explorer(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui, w: f32) {
    let p = mk::pal();
    let files = state.db_files.clone();
    let folder = state.db_folder.clone();
    // Row heights: header ~40 + path 22 + rows*(5+18+5+2) + footer 32 + padding.
    let rows = files.len().min(10) as f32;
    let h = 16.0 + 40.0 + 10.0 + 22.0 + 6.0 + rows * 24.0 + 8.0 + 24.0 + 16.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(12), p.bg_card);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );
    let mut y = rect.min.y + 16.0;
    // Header: title + desc (left), open-folder mini btn (right).
    ui.painter().text(
        pos2(rect.min.x + 16.0, y + 9.0),
        Align2::LEFT_CENTER,
        state.t("App Database"),
        FontId::new(18.0, crate::ui::tokens::typography::bold_family()),
        p.text,
    );
    ui.painter().text(
        pos2(rect.min.x + 16.0, y + 30.0),
        Align2::LEFT_CENTER,
        state.t("Settings, download history, categories, and schedules."),
        FontId::new(11.0, egui::FontFamily::Proportional),
        p.text_dim,
    );
    if mini_btn_in(
        ui,
        "folder_open",
        Rect::from_min_size(pos2(rect.max.x - 16.0 - 24.0, y - 2.0), vec2(24.0, 24.0)),
    ) {
        ctx.run(move |app| async move { moonhunt_core::db::open_app_db_folder(app) });
    }
    y += 40.0 + 10.0;
    // Path row (folder icon + monospace path).
    mi::paint_centered(
        ui.painter(),
        pos2(rect.min.x + 16.0 + 7.5, y + 11.0),
        "folder",
        15.0,
        p.text_dim,
    );
    let path_clip = Rect::from_min_max(
        pos2(rect.min.x + 16.0 + 15.0 + 7.0, y),
        pos2(rect.max.x - 16.0, y + 22.0),
    );
    ui.painter().with_clip_rect(path_clip).text(
        pos2(path_clip.min.x, y + 11.0),
        Align2::LEFT_CENTER,
        folder.clone(),
        FontId::monospace(11.0),
        p.text_dim,
    );
    y += 22.0 + 6.0;
    // File rows.
    for (name, size) in files.iter().take(10) {
        let rr = Rect::from_min_size(pos2(rect.min.x + 8.0, y), vec2(rect.width() - 16.0, 24.0));
        if ui.rect_contains_pointer(rr) {
            ui.painter()
                .rect_filled(rr, CornerRadius::same(6), p.hover_veil);
        }
        mi::paint_centered(
            ui.painter(),
            pos2(rr.min.x + 10.0, rr.center().y),
            "description",
            18.0,
            p.text_dim,
        );
        let name_clip = Rect::from_min_max(
            pos2(rr.min.x + 28.0, rr.min.y),
            pos2(rr.max.x - 60.0, rr.max.y),
        );
        ui.painter().with_clip_rect(name_clip).text(
            pos2(name_clip.min.x, rr.center().y),
            Align2::LEFT_CENTER,
            name.clone(),
            FontId::monospace(12.0),
            p.text,
        );
        let size_s = fmt_bytes(*size);
        let sw = ui
            .painter()
            .layout_no_wrap(size_s.clone(), FontId::new(11.0, egui::FontFamily::Proportional), p.text_dim)
            .rect
            .width();
        ui.painter().text(
            pos2(rr.max.x - 24.0 - 8.0 - sw, rr.center().y),
            Align2::LEFT_CENTER,
            size_s,
            FontId::new(11.0, egui::FontFamily::Proportional),
            p.text_dim,
        );
        if mini_btn_in(ui, "open_in_new", Rect::from_min_size(pos2(rr.max.x - 24.0, rr.min.y), vec2(24.0, 24.0))) {
            ctx.run(move |app| async move { moonhunt_core::db::open_app_db_folder(app) });
        }
        y += 24.0;
    }
    // Footer: refresh.
    let fr = Rect::from_min_size(pos2(rect.max.x - 16.0 - 24.0, rect.max.y - 16.0 - 24.0), vec2(24.0, 24.0));
    if mini_btn_in(ui, "refresh", fr) {
        state.db_next_fetch = std::time::Instant::now();
        state.db_folder.clear();
    }
}


// ── Save/apply + token util ───────────────────────────────────────────

fn save_and_apply(state: &mut UiState, ctx: &Ctx) {
    let settings = state.settings.clone();
    let c = ctx.clone();
    // Old `saveSettings` flow: persist first and report that on its own
    // (`All settings saved successfully!` / `Error saving settings: …`), then
    // do the slower live aria2/extension work, which only ever speaks up when
    // it fails (`Settings saved, but live download settings could not be applied.`).
    let msg_saved = state.t("All settings saved successfully!");
    let msg_partial = state.t("Settings saved, but live download settings could not be applied.");
    ctx.rt.spawn(async move {
        if let Err(e) = moonhunt_core::db::save_all_settings(settings.clone(), c.core.clone()).await {
            c.toast(false, format!("Error saving settings: {e}"));
            return;
        }
        c.toast(true, msg_saved);
        let live: Result<String, String> = async {
            moonhunt_core::aria2::set_proxy(settings.clone(), c.core.clone()).await?;
            moonhunt_core::aria2::apply_global_speed_limit(settings).await
        }
        .await;
        if let Err(e) = live {
            eprintln!("[!] Failed to apply runtime settings: {e}");
            c.toast(false, msg_partial);
        }
    });
    state.settings_dirty = false;
}

fn uuid_like_token() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let h = |mix: u128| {
        let mut x = now ^ mix ^ (std::process::id() as u128);
        for _ in 0..4 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
        }
        format!("{x:016x}")
    };
    format!("{}-{}-{}-{}", h(0x9e3779b9), h(0x1b873593), h(0x85ebca6b), h(0xc2b2ae35))
}
