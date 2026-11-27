//! Addon screen — pixel port of `AddonPage.vue` + `AddonPickerModal.vue`
//! (sidebar with addon tabs, General management list, install/enable/share/
//! remove flows, template export). The old `AddonHost.vue` renders addon code
//! through the WebView; the native port keeps the manager UI and defers the
//! embedded host.

use super::Ctx;
use crate::download_dialogs::{close_button, input_shell, text_input};
use crate::state::UiState;
use crate::ui::icons as mi;
use crate::ui::tokens;
use crate::ui::widgets as mk;
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Key, Rect, ScrollArea, Sense, Stroke,
    StrokeKind, Ui,
};

/// `.addon-sidebar { width: 180px }`.
const SIDEBAR_W: f32 = 180.0;
/// `padding: 20px 12px 16px 16px` around a 28px folder button.
const HEADER_H: f32 = 20.0 + 28.0 + 16.0;
/// `.sidebar-item`: 10+line+10, then `margin-bottom: 2px`.
const ITEM_H: f32 = 40.0;
const ITEM_GAP: f32 = 2.0;
/// `.sidebar-footer`: 1px border + `padding: 10px 12px 12px` + 30px buttons.
const FOOTER_H: f32 = 1.0 + 10.0 + 30.0 + 12.0;

#[derive(Clone, Default)]
pub struct AddonRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub icon: String,
    pub enabled: bool,
    /// `manifest.url_patterns` — hosts this addon claims (`matchUrl`).
    pub patterns: Vec<String>,
}

/// `useAddons.matchUrl` — the enabled addon whose manifest `url_patterns`
/// contains the URL's host.
impl AddonUi {
    pub fn match_url(&self, url: &str) -> Option<&AddonRow> {
        let host = crate::state::UiState::host_of(url)?.to_lowercase();
        self.addons
            .iter()
            .filter(|a| a.enabled)
            .find(|a| a.patterns.iter().any(|pt| host.contains(&pt.to_lowercase())))
    }
}

/// Addon-screen state (installed addons, active tab, picker inputs).
pub struct AddonUi {
    pub loaded: bool,
    pub loading: bool,
    /// `onActivated()` / install / enable / remove set this; `show()` re-fetches.
    pub reload: bool,
    pub error: String,
    pub addons: Vec<AddonRow>,
    /// `activeTab`: `"general"` or an enabled addon id.
    pub active: String,
    pub collapsed: bool,
    pub picker: bool,
    pub picker_url: String,
    pub picker_busy: bool,
    pub picker_error: String,
    pub picker_success: String,
    pub confirm_remove: bool,
    /// `.addon-list` wheel offset (the web list is `overflow-y: auto`).
    pub nav_scroll: f32,
    /// `open-addon` handoff: the URL the dialog wants the addon to take. The
    /// embedded addon host is still deferred, so this only records intent.
    pub pending_url: Option<String>,
}

impl Default for AddonUi {
    fn default() -> Self {
        Self {
            loaded: false,
            loading: false,
            reload: false,
            error: String::new(),
            addons: Vec::new(),
            active: "general".to_owned(),
            collapsed: false,
            picker: false,
            picker_url: String::new(),
            picker_busy: false,
            picker_error: String::new(),
            picker_success: String::new(),
            confirm_remove: false,
            nav_scroll: 0.0,
            pending_url: None,
        }
    }
}

// ── Text helpers ────────────────────────────────────────────────────────

fn prop(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Proportional)
}

fn bold(size: f32) -> FontId {
    FontId::new(size, tokens::typography::bold_family())
}

fn semi(size: f32) -> FontId {
    FontId::new(size, tokens::typography::semibold_family())
}

/// CSS `rgba(rgb, a)` on an opaque colour.
fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8)
}

fn text_w(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .rect
        .width()
}

fn ellipsize(ui: &Ui, text: &str, font: &FontId, max_w: f32) -> String {
    crate::rtl::truncate(text, |s| text_w(ui, s, font) <= max_w)
}

// ── Screen ──────────────────────────────────────────────────────────────

pub fn show(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui) {
    if !state.addon.loaded && !state.addon.loading {
        fetch_list(state, ctx);
    }
    if state.addon.reload && !state.addon.loading {
        state.addon.reload = false;
        fetch_list(state, ctx);
    }

    let area = ui.available_rect_before_wrap();
    // `.addon-sidebar { transition: width .2s }`.
    let sb_w = ui.ctx().animate_value_with_time(
        egui::Id::new("addon-sidebar-w"),
        if state.addon.collapsed { 0.0 } else { SIDEBAR_W },
        0.2,
    );
    sidebar(ui, state, ctx, area, sb_w);

    // `.addon-content { margin-left: 180px; padding: 24px 32px 32px }` inside
    // `.main-content`'s 20px side padding (the panel exposes the bare area).
    let cx = area.min.x + 20.0 + sb_w;
    let viewport = Rect::from_min_max(
        pos2(cx, area.min.y + 18.0),
        // + gutter: the pane scroller spans the panel margin so its bar is
        // flush with the window edge.
        pos2(area.max.x + super::PAGE_GUTTER, area.max.y - 24.0),
    );
    content(ui, state, ctx, viewport);

    if state.addon.picker {
        picker_modal(state, ui.ctx(), ctx);
    }
    if state.addon.confirm_remove {
        confirm_modal(state, ui.ctx(), ctx);
    }
}

fn fetch_list(state: &mut UiState, ctx: &Ctx) {
    state.addon.loading = true;
    ctx.emit("addons-list", |app| async move {
        let list = moonhunt_core::addon::list_addons(app)?;
        serde_json::to_value(list).map_err(|e| e.to_string())
    });
}

// ── Sidebar ─────────────────────────────────────────────────────────────

fn sidebar(ui: &mut Ui, state: &mut UiState, ctx: &Ctx, area: Rect, w: f32) {
    let p = mk::pal();
    let h = area.height();
    let sb = Rect::from_min_size(area.min, vec2(SIDEBAR_W, h));
    let painted = Rect::from_min_size(area.min, vec2(w, h));

    ui.painter().rect_filled(painted, CornerRadius::same(0), p.bg_card);
    if w > 0.5 {
        ui.painter().line_segment(
            [
                pos2(painted.max.x - 0.5, painted.min.y),
                pos2(painted.max.x - 0.5, painted.max.y),
            ],
            Stroke::new(1.0, p.border),
        );
    }

    // Collapse handle rides the sidebar edge (`right: -13px`); always visible.
    let cc = pos2(sb.min.x + w, sb.min.y + 22.0 + 13.0);
    let btn = Rect::from_center_size(cc, vec2(26.0, 26.0));
    let crr = ui.interact(btn, egui::Id::new("addon-collapse"), Sense::click());
    let chover = crr.hovered();
    crate::ui::shade::soft_shadow(ui, btn, 13, Color32::BLACK, 2.0, 8.0, 40);
    ui.painter().circle_filled(
        cc,
        13.0,
        if chover { mk::blend(p.bg_card, Color32::WHITE, 0.06) } else { p.bg_card },
    );
    ui.painter().circle_stroke(
        cc,
        12.5,
        Stroke::new(1.0, if chover { p.border_strong } else { p.border }),
    );
    mi::paint_centered(
        ui.painter(),
        cc,
        if state.addon.collapsed { "chevron_right" } else { "chevron_left" },
        18.0,
        if chover { p.text } else { p.text_dim },
    );
    if crr.clicked() {
        state.addon.collapsed = !state.addon.collapsed;
    }

    if w < 24.0 {
        return;
    }
    let clip = Rect::from_min_size(area.min, vec2(w, h));
    let painter = ui.painter().with_clip_rect(clip);

    // ── Header: 16/700 title + 28px folder button ──────────────────────
    let title = "Addon";
    let title_font = bold(16.0);
    painter.text(
        pos2(sb.min.x + 16.0, sb.min.y + 34.0),
        Align2::LEFT_CENTER,
        title,
        title_font.clone(),
        p.text,
    );
    let tw = painter
        .layout_no_wrap(title.to_owned(), title_font, Color32::WHITE)
        .rect
        .width();
    let folder = Rect::from_min_size(pos2(sb.min.x + 16.0 + tw + 6.0, sb.min.y + 20.0), vec2(28.0, 28.0));
    let fhov = ui.ctx().pointer_interact_pos().is_some_and(|m| folder.contains(m));
    if fhov {
        painter.rect_filled(folder, CornerRadius::same(8), p.hover_veil);
        painter.rect_stroke(
            folder,
            CornerRadius::same(8),
            Stroke::new(1.0, p.border),
            StrokeKind::Inside,
        );
    }
    mi::paint_centered(
        &painter,
        folder.center(),
        "folder_open",
        18.0,
        if fhov { p.accent_strong } else { p.text_dim },
    );
    if ui.allocate_rect(folder, Sense::click()).clicked() {
        ctx.emit("addons-folder", |app| async move {
            moonhunt_core::addon::open_addons_folder(app).map(serde_json::Value::String)
        });
    }

    // ── Nav: General + enabled addon tabs ──────────────────────────────
    let mut items: Vec<(String, String, String)> = vec![(
        "general".to_owned(),
        state.t("General").to_owned(),
        "tune".to_owned(),
    )];
    for a in &state.addon.addons {
        if a.enabled {
            items.push((a.id.clone(), a.name.clone(), a.icon.clone()));
        }
    }
    // ── Footer: Add (accent) / Remove (bordered) ───────────────────────
    // Computed before the nav band: the list scrolls between header and footer.
    let footer = Rect::from_min_size(
        pos2(sb.min.x, sb.max.y - FOOTER_H),
        vec2(SIDEBAR_W, FOOTER_H),
    );

    let nav = Rect::from_min_max(
        pos2(sb.min.x + 12.0, sb.min.y + HEADER_H + 8.0),
        pos2(sb.max.x - 12.0, footer.min.y - 8.0),
    );
    let total = items.len() as f32 * (ITEM_H + ITEM_GAP) - ITEM_GAP;
    let max_scroll = (total - nav.height()).max(0.0);
    // `rect_contains_pointer`, not `ui.interact`: the rows are click-sensing
    // widgets created later, so a hover-sense probe on the band is occluded by
    // whichever row is under the wheel — the wheel would silently do nothing.
    // ponytail: hand-rolled offset instead of a ScrollArea because the rows are
    // absolutely painted and need the band's clip rect; swap for ScrollArea if
    // the rows ever move to layout-driven allocation.
    if ui.rect_contains_pointer(nav) {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0.0 {
            state.addon.nav_scroll = (state.addon.nav_scroll - dy).clamp(0.0, max_scroll);
            // `smooth_scroll_delta` is a per-frame slice of a queue; without a
            // repaint request the queue stalls and the list barely moves.
            ui.ctx().request_repaint();
        }
    }
    state.addon.nav_scroll = state.addon.nav_scroll.clamp(0.0, max_scroll);
    let mut y = nav.min.y - state.addon.nav_scroll;
    let mut switch: Option<String> = None;
    for (id, label, icon) in items {
        let r = Rect::from_min_size(pos2(sb.min.x + 12.0, y), vec2(SIDEBAR_W - 24.0, ITEM_H));
        if r.max.y > nav.min.y && r.min.y < nav.max.y {
            if sidebar_item(ui, r, state.addon.active == id, &label, &icon, nav).clicked() {
                switch = Some(id);
            }
        }
        y += ITEM_H + ITEM_GAP;
    }
    if let Some(id) = switch {
        state.addon.active = id;
    }
    painter.line_segment(
        [footer.min, pos2(footer.min.x + w, footer.min.y)],
        Stroke::new(1.0, p.border),
    );
    let inner = (SIDEBAR_W - 24.0 - 8.0) * 0.5;
    let add = Rect::from_min_size(pos2(sb.min.x + 12.0, footer.min.y + 11.0), vec2(inner, 30.0));
    let add_hover = ui.ctx().pointer_interact_pos().is_some_and(|m| add.contains(m));
    painter.rect_filled(
        add,
        CornerRadius::same(7),
        if add_hover { p.accent.gamma_multiply(0.9) } else { p.accent },
    );
    let dark = Color32::from_rgb(0x0a, 0x0e, 0x1a);
    let af = bold(12.0);
    let atw = painter.layout_no_wrap(state.t("Add").to_owned(), af.clone(), dark).rect.width();
    let ax = add.center().x - (14.0 + 4.0 + atw) * 0.5;
    mi::paint_centered(&painter, pos2(ax + 7.0, add.center().y), "add", 14.0, dark);
    painter.text(
        pos2(ax + 18.0, add.center().y),
        Align2::LEFT_CENTER,
        state.t("Add"),
        af,
        dark,
    );
    if ui.allocate_rect(add, Sense::click()).clicked() {
        state.addon.picker = true;
        state.addon.picker_error.clear();
        state.addon.picker_success.clear();
        state.addon.picker_url.clear();
        state.addon.reload = true;
    }

    let rm = Rect::from_min_size(
        pos2(add.max.x + 8.0, footer.min.y + 11.0),
        vec2(inner, 30.0),
    );
    let rm_hover = ui.ctx().pointer_interact_pos().is_some_and(|m| rm.contains(m));
    painter.rect_filled(rm, CornerRadius::same(7), p.hover_veil);
    painter.rect_stroke(
        rm,
        CornerRadius::same(7),
        Stroke::new(1.0, if rm_hover { p.border_strong } else { p.border }),
        StrokeKind::Inside,
    );
    let rf = prop(12.0);
    let rtw = painter.layout_no_wrap(state.t("Remove").to_owned(), rf.clone(), p.text_dim).rect.width();
    let rx = rm.center().x - (14.0 + 4.0 + rtw) * 0.5;
    let rc = if rm_hover { p.text } else { p.text_dim };
    mi::paint_centered(&painter, pos2(rx + 7.0, rm.center().y), "remove", 14.0, rc);
    painter.text(
        pos2(rx + 18.0, rm.center().y),
        Align2::LEFT_CENTER,
        state.t("Remove"),
        rf,
        rc,
    );
    if ui.allocate_rect(rm, Sense::click()).clicked() {
        if state.addon.active == "general" {
            state.toast(true, "Select an addon tab first.");
        } else {
            state.addon.confirm_remove = true;
        }
    }
}

fn sidebar_item(
    ui: &mut Ui,
    rect: Rect,
    active: bool,
    label: &str,
    icon: &str,
    clip: Rect,
) -> egui::Response {
    let p = mk::pal();
    let painter = ui.painter().with_clip_rect(clip);
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    if active {
        painter.rect_filled(rect, CornerRadius::same(8), with_alpha(p.accent, 0.10));
    } else if hovered {
        painter.rect_filled(rect, CornerRadius::same(8), p.hover_veil);
    }
    let color = if active {
        p.accent_strong
    } else if hovered {
        p.text
    } else {
        p.text_dim
    };
    mi::paint_centered(
        &painter,
        pos2(rect.min.x + 12.0 + 11.0, rect.center().y),
        icon,
        20.0,
        color,
    );
    let font = FontId::new(
        14.0,
        if active {
            tokens::typography::semibold_family()
        } else {
            egui::FontFamily::Proportional
        },
    );
    let max_w = rect.width() - 44.0 - 12.0;
    let text = ellipsize(ui, label, &font, max_w);
    painter.text(
        pos2(rect.min.x + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        color,
    );
    resp
}

// ── Content ─────────────────────────────────────────────────────────────

fn content(ui: &mut Ui, state: &mut UiState, ctx: &Ctx, viewport: Rect) {
    if viewport.width() < 40.0 {
        return;
    }
    if state.addon.active == "general" {
        general_section(ui, state, ctx, viewport);
    } else {
        addon_tab(ui, state, ctx, viewport);
    }
}

fn general_section(ui: &mut Ui, state: &mut UiState, ctx: &Ctx, viewport: Rect) {
    let p = mk::pal();
    if viewport.width() < 80.0 {
        return;
    }
    // `.addon-content` scrolls the pane and carries its 32px/24px padding, so the
    // wheel works over the gutters too — inside egui the padding belongs to the
    // content rect, not to a Frame around the ScrollArea (that left dead strips).
    ui.scope_builder(egui::UiBuilder::new().max_rect(viewport), |ui| {
        ScrollArea::vertical()
            .id_salt("addon-general")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut page = ui.available_rect_before_wrap();
                page.min.x += 32.0;
                page.max.x -= 32.0 + super::PAGE_GUTTER;
                page.min.y += 24.0;
                ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
                let w = ui.available_width();
                ui.set_width(w);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                // `.section-title`: 18/700, margin 0 0 4. The CSS line box is
                // 24px (18 × 1.3), so the subtitle starts 2px lower than the
                // glyph extent alone suggests.
                let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
                ui.painter().text(
                    r.left_top(),
                    Align2::LEFT_TOP,
                    state.t("General"),
                    bold(18.0),
                    p.text,
                );
                ui.add_space(4.0);
                // `.section-sub`: 13 dim, margin 0 0 20.
                let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                ui.painter().text(
                    r.left_top(),
                    Align2::LEFT_TOP,
                    state.t("Installed addons. Use Add to install more."),
                    prop(13.0),
                    p.text_dim,
                );
                ui.add_space(20.0);

                if state.addon.addons.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
                    ui.painter().text(
                        r.left_top(),
                        Align2::LEFT_TOP,
                        state.t("No addons yet."),
                        prop(13.0),
                        p.text_dim,
                    );
                    ui.add_space(20.0);
                } else {
                    for i in 0..state.addon.addons.len() {
                        if i > 0 {
                            ui.add_space(8.0);
                        }
                        addon_row(ui, state, ctx, i, w);
                    }
                }

                // `.addon-section`: padding-bottom 24, border-bottom, then the
                // next section's 32px margin.
                ui.add_space(24.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, CornerRadius::same(0), p.border);
                ui.add_space(32.0);
                });
            });
    });
}

fn addon_row(ui: &mut Ui, state: &mut UiState, ctx: &Ctx, idx: usize, w: f32) {
    let p = mk::pal();
    let row = state.addon.addons[idx].clone();
    let (r, _) = ui.allocate_exact_size(vec2(w, 58.0), Sense::hover());
    ui.painter()
        .rect_filled(r, CornerRadius::same(10), p.bg_card);
    ui.painter().rect_stroke(
        r,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    // `.addon-row-icon`: 36px, accent 12% bg, radius 10.
    let icon = Rect::from_min_size(pos2(r.min.x + 12.0, r.center().y - 18.0), vec2(36.0, 36.0));
    ui.painter()
        .rect_filled(icon, CornerRadius::same(10), with_alpha(p.accent, 0.12));
    let row_icon = if row.icon.is_empty() { "extension" } else { row.icon.as_str() };
    mi::paint_centered(
        ui.painter(),
        icon.center(),
        row_icon,
        24.0,
        p.accent_strong,
    );

    // Toggle (42×24) + share button (30×30) at the right, 10px gaps.
    let share = Rect::from_min_size(
        pos2(r.max.x - 1.0 - 12.0 - 30.0, r.center().y - 15.0),
        vec2(30.0, 30.0),
    );
    let toggle = Rect::from_min_size(
        pos2(share.min.x - 10.0 - 42.0, r.center().y - 12.0),
        vec2(42.0, 24.0),
    );

    // Name + version + description, clipped before the toggle.
    let tx = r.min.x + 58.0;
    let name_font = bold(13.0);
    let ver_font = prop(10.8);
    let desc_font = prop(12.0);
    let name_h = ui.fonts_mut(|f| f.row_height(&name_font));
    let desc_h = ui.fonts_mut(|f| f.row_height(&desc_font));
    let block_h = name_h + desc_h;
    let top = r.center().y - block_h * 0.5;
    let clip = Rect::from_min_max(pos2(tx, r.min.y + 1.0), pos2(toggle.min.x - 10.0, r.max.y - 1.0));
    let tp = ui.painter().with_clip_rect(clip);
    let version = format!("v{}", row.version);
    let name_w = text_w(ui, &row.name, &name_font);
    let ver_h = ui.fonts_mut(|f| f.row_height(&ver_font));
    tp.text(
        pos2(tx, top),
        Align2::LEFT_TOP,
        &row.name,
        name_font,
        p.text,
    );
    tp.text(
        pos2(tx + name_w + 6.0, top + (name_h - ver_h) * 0.6),
        Align2::LEFT_TOP,
        version,
        ver_font,
        p.text_dim,
    );
    let desc = if row.description.trim().is_empty() {
        row.author.clone()
    } else {
        row.description.clone()
    };
    let avail = (toggle.min.x - 10.0 - tx).max(20.0);
    tp.text(
        pos2(tx, top + name_h),
        Align2::LEFT_TOP,
        ellipsize(ui, &desc, &desc_font, avail),
        desc_font,
        p.text_dim,
    );

    // `.speed-toggle`: 42×24, 12r; knob 18 slides 18px.
    let trr = ui.interact(toggle, egui::Id::new(("addon-tgl", row.id.as_str())), Sense::click());
    let t = ui.ctx().animate_value_with_time(
        egui::Id::new(("addon-tgl-t", row.id.as_str())),
        if row.enabled { 1.0 } else { 0.0 },
        0.15,
    );
    let fill = if row.enabled {
        with_alpha(p.accent, 0.25)
    } else {
        p.hover_veil
    };
    ui.painter()
        .rect_filled(toggle, CornerRadius::same(12), fill);
    ui.painter().rect_stroke(
        toggle,
        CornerRadius::same(12),
        Stroke::new(1.0, if row.enabled { with_alpha(p.accent, 0.4) } else { p.border_strong }),
        StrokeKind::Inside,
    );
    let knob = pos2(toggle.min.x + 2.0 + 9.0 + 18.0 * t, toggle.center().y);
    ui.painter().circle_filled(
        knob,
        9.0,
        mk::blend(p.text_dim, p.accent, t),
    );

    // `.row-icon-btn`: 30×30, 8r.
    let srr = ui.interact(share, egui::Id::new(("addon-share", row.id.as_str())), Sense::click());
    let shov = srr.hovered();
    ui.painter().rect_filled(
        share,
        CornerRadius::same(8),
        if shov { with_alpha(p.accent, 0.08) } else { Color32::TRANSPARENT },
    );
    ui.painter().rect_stroke(
        share,
        CornerRadius::same(8),
        Stroke::new(1.0, if shov { with_alpha(p.accent, 0.3) } else { p.border }),
        StrokeKind::Inside,
    );
    mi::paint_centered(
        ui.painter(),
        share.center(),
        "ios_share",
        17.0,
        if shov { p.accent_strong } else { p.text_dim },
    );

    if trr.clicked() {
        let enabled = !row.enabled;
        if let Some(a) = state.addon.addons.get_mut(idx) {
            a.enabled = enabled;
        }
        if !enabled && state.addon.active == row.id {
            state.addon.active = "general".to_owned();
        }
        let id = row.id.clone();
        ctx.emit("addon-enabled", move |app| async move {
            moonhunt_core::addon::set_addon_enabled(app, id, enabled)
                .map(serde_json::Value::String)
        });
    }
    if srr.clicked() {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title(state.t("Choose folder for the exported .zip"))
            .pick_folder()
        {
            let id = row.id.clone();
            let dir = dir.to_string_lossy().to_string();
            ctx.emit("addon-export", move |app| async move {
                moonhunt_core::addon::export_addon_package(app, id, dir)
                    .map(serde_json::Value::String)
            });
        }
    }
}

fn addon_tab(ui: &mut Ui, state: &mut UiState, ctx: &Ctx, viewport: Rect) {
    let p = mk::pal();
    let Some(a) = state.addon.addons.iter().find(|a| a.id == state.addon.active) else {
        return;
    };
    let (id, icon, name) = (a.id.clone(), a.icon.clone(), a.name.clone());
    // The Spotify bridge has a native host (`screens::spotify`); every other
    // addon still needs the WebView ESM host this build does not ship.
    if id == crate::screens::spotify::ADDON_ID {
        crate::screens::spotify::show(state, ctx, ui, viewport);
        return;
    }
    // Same as `general_section`: the pane scrolls, the 720px column is content.
    ui.scope_builder(egui::UiBuilder::new().max_rect(viewport), |ui| {
        ScrollArea::vertical()
            .id_salt("addon-tab")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail = ui.available_rect_before_wrap();
                let avail = Rect::from_min_max(
                    avail.min,
                    pos2(avail.max.x - super::PAGE_GUTTER, avail.max.y),
                );
                let w = 720.0_f32.min(avail.width() - 64.0).max(120.0);
                let x = avail.min.x + 32.0 + ((avail.width() - 64.0 - w) * 0.5).max(0.0);
                let page = Rect::from_min_size(
                    pos2(x, avail.min.y + 24.0),
                    vec2(w, avail.height() - 24.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
                ui.set_width(w);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let (r, _) = ui.allocate_exact_size(vec2(w, 116.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, CornerRadius::same(14), p.bg_card);
                ui.painter().rect_stroke(
                    r,
                    CornerRadius::same(14),
                    Stroke::new(1.0, p.border),
                    StrokeKind::Inside,
                );
                let ib = Rect::from_min_size(pos2(r.min.x + 24.0, r.min.y + 24.0), vec2(44.0, 44.0));
                ui.painter()
                    .rect_filled(ib, CornerRadius::same(12), with_alpha(p.accent, 0.12));
                mi::paint_centered(
                    ui.painter(),
                    ib.center(),
                    if icon.is_empty() { "extension" } else { icon.as_str() },
                    26.0,
                    p.accent_strong,
                );
                ui.painter().text(
                    pos2(ib.max.x + 14.0, ib.min.y + 4.0),
                    Align2::LEFT_TOP,
                    &name,
                    bold(15.0),
                    p.text,
                );
                ui.painter().text(
                    pos2(ib.max.x + 14.0, ib.min.y + 26.0),
                    Align2::LEFT_TOP,
                    state.t("This addon's UI host is not available in the native build yet."),
                    prop(12.0),
                    p.text_dim,
                );
                // `AddonHost` forwards an initial URL into the addon; with no
                // host to receive it, show what would have been handed over.
                if let Some(url) = state
                    .addon
                    .pending_url
                    .as_deref()
                    .filter(|_| state.addon.active == id)
                {
                    ui.painter().text(
                        pos2(ib.max.x + 14.0, ib.min.y + 46.0),
                        Align2::LEFT_TOP,
                        url,
                        egui::FontId::monospace(11.0),
                        p.text_muted,
                    );
                }
                });
            });
    });
}

// ── AddonPickerModal ────────────────────────────────────────────────────

fn picker_modal(state: &mut UiState, ectx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ectx.content_rect();
    // `.addon-picker` asks for 520px, but `.nd-modal`'s `max-width: 480px`
    // still wins (different property), so the rendered card is 480 wide.
    let w = 480.0_f32.min((screen.width() - 48.0).max(320.0));
    let busy = state.addon.picker_busy;

    let mut act_file = false;
    let mut act_folder = false;
    let mut act_url = false;
    let mut act_template = false;
    let mut close = false;

    let mut overlay_click = false;
    egui::Area::new(egui::Id::new("addon-picker-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ectx, |ui| {
            ui.painter().rect_filled(
                screen,
                CornerRadius::same(0),
                Color32::from_black_alpha(140),
            );
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                overlay_click = true;
            }
        });
    if overlay_click || ectx.input(|i| i.key_pressed(Key::Escape)) {
        close = true;
    }

    let body_max = (screen.height() * 0.85 - 61.0 - 63.0).max(120.0);

    egui::Area::new(egui::Id::new("addon-picker"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ectx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.border_strong))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    // ── Header (`.nd-header`: padding 14 16 10) ──────────
                    let (h, _) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
                    let avatar = Rect::from_min_size(
                        pos2(h.min.x + 16.0, h.min.y + 14.0),
                        vec2(36.0, 36.0),
                    );
                    crate::ui::shade::grad135(
                        ui,
                        avatar,
                        10,
                        mk::blend(p.bg_card, p.accent, 0.12),
                        mk::blend(p.bg_card, p.accent, 0.04),
                    );
                    ui.painter().rect_stroke(
                        avatar,
                        CornerRadius::same(10),
                        Stroke::new(1.0, with_alpha(p.accent, 0.12)),
                        StrokeKind::Inside,
                    );
                    mi::paint_centered(
                        ui.painter(),
                        avatar.center(),
                        "extension",
                        18.0,
                        p.accent_strong,
                    );
                    let tx = h.min.x + 16.0 + 36.0 + 10.0;
                    let title_font = bold(14.0);
                    ui.painter().text(
                        pos2(tx, h.min.y + 14.0),
                        Align2::LEFT_TOP,
                        state.t("Add Addon"),
                        title_font,
                        p.text,
                    );
                    let sub = "Official, file, folder, or URL \u{2014} community-made works too.";
                    let sub_w = (h.max.x - 16.0 - 28.0 - 12.0 - tx).max(80.0);
                    let sub_galley =
                        ui.painter()
                            .layout(sub.to_owned(), prop(11.0), p.text_muted, sub_w);
                    ui.painter().galley(
                        pos2(tx, h.min.y + 14.0 + 16.8 + 2.0),
                        sub_galley,
                        p.text_muted,
                    );
                    let cr = Rect::from_min_size(
                        pos2(h.max.x - 16.0 - 28.0, h.min.y + 14.0),
                        vec2(28.0, 28.0),
                    );
                    if close_button(ui, &p, cr, "addon-picker-close", true) {
                        close = true;
                    }

                    // `.nd-divider` at the header bottom, inset 16px.
                    let (d, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
                    ui.painter().line_segment(
                        [
                            pos2(d.min.x + 16.0, d.center().y),
                            pos2(d.max.x - 16.0, d.center().y),
                        ],
                        Stroke::new(1.0, p.border_strong),
                    );

                    // ── Body (`.nd-body`: 12/16/8, gap 12) ───────────────
                    ScrollArea::vertical()
                        .id_salt("addon-picker-body")
                        .max_height(body_max)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.set_width(w);
                            ui.spacing_mut().item_spacing = vec2(0.0, 12.0);

                            // Trust note (icon 18 + 8 + text, padding 10 12).
                            let trust = state.t("Only install addons you trust. Addons run inside MoonHunt with full app privileges.");
                            let wrap = w - 32.0 - 24.0 - 18.0 - 8.0;
                            let galley = ui.painter().layout(
                                trust.to_owned(),
                                prop(12.0),
                                p.text,
                                wrap.max(80.0),
                            );
                            let th = galley.size().y.max(18.0);
                            let r = modal_row(ui, w, th + 20.0);
                            ui.painter()
                                .rect_filled(r, CornerRadius::same(8), with_alpha(p.accent, 0.08));
                            mi::paint_centered(
                                ui.painter(),
                                pos2(r.min.x + 12.0 + 9.0, r.min.y + 10.0 + 9.0),
                                "warning",
                                18.0,
                                p.accent_strong,
                            );
                            ui.painter().galley(
                                pos2(r.min.x + 12.0 + 18.0 + 8.0, r.min.y + 10.0),
                                galley,
                                p.text,
                            );
                            // `.addon-trust { margin-bottom: 8px }`.
                            ui.add_space(8.0);

                            modal_section(ui, &p, w, state.t("Community addons"));
                            modal_hint(
                                ui,
                                &p,
                                w,
                                state.t("Install a .zip package from a file, a folder, or a link. A playlist-bridge example is preinstalled."),
                            );

                            let r = modal_row(ui, w, 38.0);
                            let bw = (r.width() - 8.0) * 0.5;
                            if crate::modals::nd_button(
                                ui,
                                &p,
                                "addon-pick-file",
                                Rect::from_min_size(r.min, vec2(bw, 38.0)),
                                state.t("Choose .zip file"),
                                Some("folder_open"),
                                false,
                                !busy,
                            ) {
                                act_file = true;
                            }
                            if crate::modals::nd_button(
                                ui,
                                &p,
                                "addon-pick-folder",
                                Rect::from_min_size(
                                    pos2(r.min.x + bw + 8.0, r.min.y),
                                    vec2(bw, 38.0),
                                ),
                                state.t("Install from folder"),
                                Some("drive_folder_upload"),
                                false,
                                !busy,
                            ) {
                                act_folder = true;
                            }
                            // `.addon-actions { margin-bottom: 8px }`.
                            ui.add_space(8.0);

                            let r = modal_row(ui, w, 36.0);
                            let uid = egui::Id::new("addon-picker-url");
                            let focused = ui.ctx().memory(|m| m.has_focus(uid));
                            let hovered = ui.rect_contains_pointer(r);
                            input_shell(ui, &p, r, focused, hovered, 8);
                            text_input(
                                ui,
                                uid,
                                r,
                                &mut state.addon.picker_url,
                                "https://example.com/my-addon.zip",
                                prop(12.0),
                                10,
                                0.55,
                            );
                            if focused
                                && ui.ctx().input(|i| i.key_pressed(Key::Enter))
                                && !state.addon.picker_url.trim().is_empty()
                                && !busy
                            {
                                act_url = true;
                            }
                            // `.addon-url-wrap { margin-bottom: 8px }`.
                            ui.add_space(8.0);

                            let r = modal_row(ui, w, 38.0);
                            let can_url = !state.addon.picker_url.trim().is_empty() && !busy;
                            if crate::modals::nd_button(
                                ui,
                                &p,
                                "addon-pick-url",
                                r,
                                state.t("Install from URL"),
                                Some("download"),
                                false,
                                can_url,
                            ) && can_url
                            {
                                act_url = true;
                            }
                            // `.addon-url-btn { margin-bottom: 8px }`.
                            ui.add_space(8.0);

                            modal_section(ui, &p, w, state.t("Make your own"));
                            modal_hint(
                                ui,
                                &p,
                                w,
                                state.t("Get the starter template (manifest.json + addon.js + guide), edit it, test via Install from folder, share as .zip."),
                            );
                            let r = modal_row(ui, w, 38.0);
                            if crate::modals::nd_button(
                                ui,
                                &p,
                                "addon-template",
                                r,
                                state.t("Download starter template"),
                                Some("code"),
                                false,
                                !busy,
                            ) {
                                act_template = true;
                            }

                            if busy {
                                status_box(
                                    ui,
                                    &p,
                                    w,
                                    StatusKind::Busy,
                                    state.t("Installing..."),
                                );
                            }
                            if !state.addon.picker_error.is_empty() {
                                let e = state.addon.picker_error.clone();
                                status_box(ui, &p, w, StatusKind::Error, &e);
                            }
                            if !state.addon.picker_success.is_empty() {
                                let s = state.addon.picker_success.clone();
                                status_box(ui, &p, w, StatusKind::Ok, &s);
                            }
                            ui.add_space(8.0);
                        });

                    // ── Footer (`.nd-footer`: 10/16/14 + 1px rule) ───────
                    let (f, _) = ui.allocate_exact_size(vec2(w, 63.0), Sense::hover());
                    ui.painter().line_segment(
                        [f.min, pos2(f.max.x, f.min.y)],
                        Stroke::new(1.0, p.border),
                    );
                    let btn = Rect::from_min_size(
                        pos2(f.min.x + 16.0, f.min.y + 11.0),
                        vec2(w - 32.0, 38.0),
                    );
                    if crate::modals::nd_button(
                        ui,
                        &p,
                        "addon-picker-done",
                        btn,
                        state.t("Close"),
                        None,
                        false,
                        true,
                    ) {
                        close = true;
                    }
                });
        });

    if close {
        state.addon.picker = false;
        state.addon.picker_url.clear();
    }
    if ectx.input(|i| i.key_pressed(Key::Escape)) {
        state.addon.picker = false;
        state.addon.picker_url.clear();
    }

    // Native pickers run after the modal closure so state borrows are clear.
    if act_file {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Addon package", &["zip"])
            .set_title(state.t("Choose addon file"))
            .pick_file()
        {
            let path = path.to_string_lossy().to_string();
            begin_install(state, cctx, Install::Path(path));
        }
    }
    if act_folder {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title(state.t("Choose addon folder"))
            .pick_folder()
        {
            let dir = dir.to_string_lossy().to_string();
            begin_install(state, cctx, Install::Dir(dir));
        }
    }
    if act_url {
        let url = state.addon.picker_url.trim().to_owned();
        if !url.is_empty() && !state.addon.picker_busy {
            state.addon.picker_url.clear();
            begin_install(state, cctx, Install::Url(url));
        }
    }
    if act_template {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title(state.t("Choose empty folder for the template"))
            .pick_folder()
        {
            let dir = dir.to_string_lossy().to_string();
            state.addon.picker_busy = true;
            state.addon.picker_error.clear();
            state.addon.picker_success.clear();
            cctx.emit("addon-template", move |_app| async move {
                moonhunt_core::addon::export_addon_template(dir)
                    .map(serde_json::Value::String)
            });
        }
    }
    if state.addon.picker_busy {
        ectx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

enum Install {
    Path(String),
    Dir(String),
    Url(String),
}

fn begin_install(state: &mut UiState, ctx: &Ctx, kind: Install) {
    state.addon.picker_busy = true;
    state.addon.picker_error.clear();
    state.addon.picker_success.clear();
    ctx.emit("addon-installed", move |app| async move {
        let a = match kind {
            Install::Path(p) => moonhunt_core::addon::install_addon_from_path(app, p).await?,
            Install::Dir(p) => moonhunt_core::addon::install_addon_from_dir(app, p)?,
            Install::Url(u) => moonhunt_core::addon::install_addon_from_url(app, u).await?,
        };
        serde_json::to_value(a).map_err(|e| e.to_string())
    });
}

/// `.nd-body` child row: full width, inset 16px on both sides.
fn modal_row(ui: &mut Ui, w: f32, h: f32) -> Rect {
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    Rect::from_min_size(pos2(r.min.x + 16.0, r.min.y), vec2(w - 32.0, h))
}

/// `.nd-section-title`: 10/700 uppercase 70%, `margin: 8px 0 0`.
fn modal_section(ui: &mut Ui, p: &crate::ui::tokens::Palette, w: f32, title: &str) {
    let r = modal_row(ui, w, 8.0 + 14.0);
    ui.painter().text(
        pos2(r.min.x, r.max.y),
        Align2::LEFT_BOTTOM,
        title.to_uppercase(),
        semi(10.0),
        with_alpha(p.text_dim, 0.7),
    );
}

/// `.nd-field-hint`: 11px dim, `margin: 2px 0 0`, line-height 1.45.
fn modal_hint(ui: &mut Ui, p: &crate::ui::tokens::Palette, w: f32, text: &str) {
    let galley = ui
        .painter()
        .layout(text.to_owned(), prop(11.0), p.text_dim, (w - 32.0).max(80.0));
    let r = modal_row(ui, w, 2.0 + galley.size().y);
    ui.painter()
        .galley(pos2(r.min.x, r.min.y + 2.0), galley, p.text_dim);
}

#[derive(Clone, Copy, PartialEq)]
enum StatusKind {
    Busy,
    Error,
    Ok,
}

/// `.spm-box` (10/12, radius 8, 18px icon, gap 8) for install feedback.
fn status_box(ui: &mut Ui, p: &crate::ui::tokens::Palette, w: f32, kind: StatusKind, text: &str) {
    let icon = match kind {
        StatusKind::Busy => None,
        StatusKind::Error => Some("error"),
        StatusKind::Ok => Some("check_circle"),
    };
    let lead = if kind == StatusKind::Busy { 14.0 } else { 18.0 };
    let wrap = (w - 32.0 - 24.0 - lead - 8.0).max(80.0);
    let galley = ui
        .painter()
        .layout(text.to_owned(), prop(12.0), p.text, wrap);
    let h = galley.size().y.max(lead) + 20.0;
    let r = modal_row(ui, w, h);
    let bg = match kind {
        StatusKind::Error => Color32::from_rgba_unmultiplied(255, 80, 80, 20),
        _ => with_alpha(p.accent, 0.08),
    };
    ui.painter().rect_filled(r, CornerRadius::same(8), bg);
    let ix = r.min.x + 12.0;
    match icon {
        None => spinner(ui, p, pos2(ix + 7.0, r.min.y + 10.0 + lead * 0.5), 6.0),
        Some(name) => mi::paint_centered(
            ui.painter(),
            pos2(ix + 9.0, r.min.y + 10.0 + lead * 0.5),
            name,
            18.0,
            if kind == StatusKind::Error {
                Color32::from_rgb(0xe5, 0x48, 0x4d)
            } else {
                p.accent_strong
            },
        ),
    }
    ui.painter().galley(
        pos2(ix + lead + 8.0, r.min.y + 10.0),
        galley,
        p.text,
    );
}

/// `.media-spinner`: 14px ring, accent 15% with an accent-strong arc.
fn spinner(ui: &Ui, p: &crate::ui::tokens::Palette, center: egui::Pos2, r: f32) {
    ui.painter()
        .circle_stroke(center, r, Stroke::new(2.0, with_alpha(p.accent, 0.15)));
    let t = ui.ctx().input(|i| i.time) as f32;
    let start = t * std::f32::consts::TAU * 1.25;
    let mut pts = Vec::with_capacity(21);
    for i in 0..=20 {
        let a = start + (i as f32 / 20.0) * std::f32::consts::TAU * 0.75;
        pts.push(pos2(center.x + r * a.cos(), center.y + r * a.sin()));
    }
    ui.painter()
        .add(egui::Shape::line(pts, Stroke::new(2.0, p.accent_strong)));
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(16));
}

// ── Remove confirmation ────────────────────────────────────────────────

fn confirm_modal(state: &mut UiState, ectx: &egui::Context, cctx: &Ctx) {
    let p = mk::pal();
    let screen = ectx.content_rect();
    let w = 380.0_f32.min((screen.width() - 48.0).max(280.0));

    let mut overlay_click = false;
    egui::Area::new(egui::Id::new("addon-confirm-overlay"))
        .order(egui::Order::Middle)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ectx, |ui| {
            ui.painter().rect_filled(
                screen,
                CornerRadius::same(0),
                Color32::from_black_alpha(140),
            );
            if ui.allocate_rect(screen, Sense::click()).clicked() {
                overlay_click = true;
            }
        });

    let mut remove = false;
    let mut close = overlay_click || ectx.input(|i| i.key_pressed(Key::Escape));

    egui::Area::new(egui::Id::new("addon-confirm"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ectx, |ui| {
            egui::Frame::new()
                .fill(p.bg_card)
                .corner_radius(CornerRadius::same(14))
                .stroke(Stroke::new(1.0, p.border_strong))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

                    let (b, _) = ui.allocate_exact_size(vec2(w, 84.0), Sense::hover());
                    let ib = Rect::from_min_size(
                        pos2(b.min.x + 16.0, b.min.y + 16.0),
                        vec2(36.0, 36.0),
                    );
                    ui.painter().rect_filled(
                        ib,
                        CornerRadius::same(10),
                        Color32::from_rgba_unmultiplied(0xef, 0x53, 0x50, 26),
                    );
                    mi::paint_centered(
                        ui.painter(),
                        ib.center(),
                        "delete",
                        18.0,
                        p.status_error,
                    );
                    ui.painter().text(
                        pos2(ib.max.x + 12.0, ib.min.y + 3.0),
                        Align2::LEFT_TOP,
                        state.t("Remove addon"),
                        bold(14.0),
                        p.text,
                    );
                    let msg = state.t("Remove this addon and its files?").to_owned();
                    let galley = ui.painter().layout(
                        msg,
                        prop(12.0),
                        p.text_muted,
                        w - 32.0 - 48.0,
                    );
                    ui.painter()
                        .galley(pos2(ib.max.x + 12.0, ib.min.y + 23.0), galley, p.text_muted);

                    let (f, _) = ui.allocate_exact_size(vec2(w, 63.0), Sense::hover());
                    ui.painter().line_segment(
                        [f.min, pos2(f.max.x, f.min.y)],
                        Stroke::new(1.0, p.border),
                    );
                    let bw = (w - 32.0 - 8.0) * 0.5;
                    if crate::modals::nd_button(
                        ui,
                        &p,
                        "addon-confirm-cancel",
                        Rect::from_min_size(
                            pos2(f.min.x + 16.0, f.min.y + 11.0),
                            vec2(bw, 38.0),
                        ),
                        state.t("Cancel"),
                        None,
                        false,
                        true,
                    ) {
                        close = true;
                    }
                    if crate::modals::nd_button(
                        ui,
                        &p,
                        "addon-confirm-remove",
                        Rect::from_min_size(
                            pos2(f.min.x + 16.0 + bw + 8.0, f.min.y + 11.0),
                            vec2(bw, 38.0),
                        ),
                        state.t("Remove"),
                        None,
                        true,
                        true,
                    ) {
                        remove = true;
                    }
                });
        });

    if remove {
        let id = state.addon.active.clone();
        if id != "general" {
            cctx.emit("addon-removed", move |app| async move {
                moonhunt_core::addon::uninstall_addon(app, id).map(serde_json::Value::String)
            });
        }
        state.addon.confirm_remove = false;
    } else if close {
        state.addon.confirm_remove = false;
    }
}

// ── Events ─────────────────────────────────────────────────────────────

/// Addon-related backend events. Returns `true` when consumed.
pub fn apply_event(state: &mut UiState, name: &str, payload: &serde_json::Value) -> bool {
    let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let err = || {
        payload
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Addon operation failed")
            .to_owned()
    };
    let data = payload.get("data");
    match name {
        "addons-list" => {
            state.addon.loading = false;
            if ok {
                state.addon.loaded = true;
                state.addon.error.clear();
                state.addon.addons = data
                    .and_then(|v| serde_json::from_value::<Vec<moonhunt_core::addon::InstalledAddon>>(v.clone()).ok())
                    .map(|list| {
                        list.into_iter()
                            .map(|a| AddonRow {
                                id: a.manifest.id,
                                name: a.manifest.name,
                                version: a.manifest.version,
                                author: a.manifest.author,
                                description: a.manifest.description,
                                icon: a.manifest.icon,
                                enabled: a.enabled,
                                patterns: a.manifest.url_patterns,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // `MOONHUNT_FAKE_ADDONS=N`: the seeded list survives the core
                // fetch so the sidebar's scroll band can be exercised.
                if let Ok(n) = std::env::var("MOONHUNT_FAKE_ADDONS") {
                    for i in 0..n.parse::<usize>().unwrap_or(0) {
                        state.addon.addons.push(AddonRow {
                            id: format!("a-fake-{i}"),
                            name: format!("Fake addon {i}"),
                            version: "1.0.0".to_owned(),
                            icon: "extension".to_owned(),
                            enabled: true,
                            ..Default::default()
                        });
                    }
                }
                if state.addon.active != "general"
                    && !state.addon.addons.iter().any(|a| a.id == state.addon.active && a.enabled)
                {
                    state.addon.active = "general".to_owned();
                }
            } else {
                // Outside Tauri the backend is unavailable: keep the list empty.
                state.addon.error = err();
                state.addon.addons.clear();
                state.addon.active = "general".to_owned();
            }
        }
        "addon-installed" => {
            state.addon.picker_busy = false;
            if ok {
                if let Some(a) = data {
                    if let (Some(id), Some(man)) =
                        (a.get("manifest").and_then(|m| m.get("id")), a.get("manifest"))
                    {
                        if let Some(id) = id.as_str() {
                            state.addon.active = id.to_owned();
                            let name = man.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let ver = man.get("version").and_then(|v| v.as_str()).unwrap_or("");
                            state.addon.picker_success = format!("Addon added: {name} v{ver}");
                            state.addon.picker = false;
                        }
                    }
                }
                state.addon.picker_url.clear();
                state.addon.reload = true;
            } else {
                state.addon.picker_error = err();
            }
        }
        "addon-enabled" => {
            state.addon.reload = true;
            if !ok {
                state
                    .toast(false, format!("Failed to update addon: {}", err()));
            }
        }
        "addon-removed" => {
            if ok {
                let msg = data
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| "Addon removed".to_owned());
                state.addon.active = "general".to_owned();
                state.toast(true, msg);
            } else {
                state
                    .toast(false, format!("Failed to remove addon: {}", err()));
            }
            state.addon.reload = true;
        }
        "addon-export" => {
            if ok {
                let out = data
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or_default();
                state.toast(true, format!("Addon exported: {out}"));
            } else {
                state
                    .toast(false, format!("Failed to export addon: {}", err()));
            }
        }
        "addon-template" => {
            state.addon.picker_busy = false;
            if ok {
                state.addon.picker_success = data
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| "Template exported".to_owned());
            } else {
                state.addon.picker_error = err();
            }
        }
        "addons-folder" => {
            if !ok {
                state
                    .toast(false, format!("Failed to open addons folder: {}", err()));
            }
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod match_url_tests {
    use super::AddonUi;

    fn ui_with(id: &str, pattern: &str, enabled: bool) -> AddonUi {
        let mut ui = AddonUi::default();
        ui.addons.push(super::AddonRow {
            id: id.into(),
            patterns: vec![pattern.into()],
            enabled,
            ..Default::default()
        });
        ui
    }

    #[test]
    fn matches_host_not_path_and_skips_disabled() {
        let off = ui_with("a", "youtube.com", false);
        assert!(off.match_url("https://www.youtube.com/watch?v=1").is_none());
        let on = ui_with("a", "youtube.com", true);
        assert_eq!(
            on.match_url("https://www.youtube.com/watch?v=1").map(|a| a.id.as_str()),
            Some("a")
        );
        // Vue uses `host.includes(p)`, so a longer host still matches.
        assert!(on.match_url("https://notyoutube.com/x").is_some());
        assert!(on.match_url("not a url").is_none());
        assert!(on.match_url("").is_none());
    }
}
