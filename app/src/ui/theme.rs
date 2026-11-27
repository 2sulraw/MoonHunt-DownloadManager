//! Theme installation: fonts + `egui::Style` for both modes.
//!
//! This is the layer that makes the app stop looking like stock egui — every
//! widget surface, corner radius, spacing and text style is driven from
//! MoonKit tokens.

use egui::{FontData, FontDefinitions, FontFamily, TextStyle, Visuals};
use crate::ui::tokens::{spacing, typography, Mode, RADIUS_CARD, RADIUS_CONTROL};

/// Install MoonKit fonts: Nunito primary (Latin), Vazirmatn fallback
/// (Persian), plus real-semibold/bold families for headings and card titles.
pub fn install_fonts(ctx: &egui::Context) {
    // `apply` runs again on every theme/accent flip; the font data must not be
    // rebuilt each time.
    let done = egui::Id::new("moonkit-fonts-installed");
    if ctx.data(|d| d.get_temp::<bool>(done).unwrap_or(false)) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
    ctx.set_fonts(font_defs());
}

/// The zh/ja system fonts are ~29 MB of resident font data, so they are not
/// part of [`install_fonts`]. Call this before painting CJK text (a ja/zh
/// locale, or the language cards); it re-installs the fonts once, which costs
/// one frame of relayout.
pub fn install_cjk_fonts(ctx: &egui::Context) {
    let done = egui::Id::new("moonkit-cjk-installed");
    if ctx.data(|d| d.get_temp::<bool>(done).unwrap_or(false)) {
        return;
    }
    // Base families first, so a ja/zh start-up cannot be overridden later.
    install_fonts(ctx);
    ctx.data_mut(|d| d.insert_temp(done, true));

    let mut defs = font_defs();
    // Neither bundled font covers CJK, so 日本語 / 中文 painted as tofu. Borrow
    // the system fonts instead of bundling ~29 MB.
    // ponytail: 29 MB of font data held for the whole session; msyh also serves
    // most hanzi for ja. Korean has no locale and gets tofu — add malgun only if
    // ko ever ships.
    let mut cjk = Vec::new();
    for (name, path) in [
        ("cjk-zh", r"C:\Windows\Fonts\msyh.ttc"),
        ("cjk-ja", r"C:\Windows\Fonts\msgothic.ttc"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            defs.font_data
                .insert(name.to_owned(), FontData::from_owned(bytes).into());
            cjk.push(name.to_owned());
        }
    }
    for fam in [
        FontFamily::Proportional,
        typography::semibold_family(),
        typography::bold_family(),
    ] {
        if let Some(list) = defs.families.get_mut(&fam) {
            list.extend(cjk.iter().cloned());
        }
    }
    ctx.set_fonts(defs);
}

/// Drop the CJK fonts again so the ~25 MB is not held after leaving the screen
/// that needed them. No-op when they were never installed.
pub fn release_cjk_fonts(ctx: &egui::Context) {
    let done = egui::Id::new("moonkit-cjk-installed");
    if !ctx.data(|d| d.get_temp::<bool>(done).unwrap_or(false)) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, false));
    ctx.set_fonts(font_defs());
}

fn font_defs() -> FontDefinitions {
    let mut defs = FontDefinitions::default();

    defs.font_data.insert(
        "nunito".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Nunito-Regular.ttf")).into(),
    );
    defs.font_data.insert(
        "vazirmatn".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Vazirmatn-Regular.ttf")).into(),
    );
    defs.font_data.insert(
        "nunito-semibold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Nunito-SemiBold.ttf")).into(),
    );
    defs.font_data.insert(
        "nunito-bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Nunito-Bold.ttf")).into(),
    );
    // Material Icons family (same font the old app loads via ligatures).
    defs.font_data.insert(
        crate::ui::icons::FAMILY.to_owned(),
        FontData::from_static(crate::ui::icons::FONT_BYTES).into(),
    );
    defs.families.insert(
        FontFamily::Name(crate::ui::icons::FAMILY.into()),
        vec![crate::ui::icons::FAMILY.to_owned()],
    );

    // Proportional: Nunito first, Vazirmatn as Persian/fallback, then defaults.
    let prop = defs.families.get_mut(&FontFamily::Proportional).unwrap();
    prop.insert(0, "vazirmatn".to_owned());
    prop.insert(0, "nunito".to_owned());

    // Dedicated families for weight control on titles/buttons.
    defs.families.insert(
        typography::semibold_family(),
        vec!["nunito-semibold".to_owned(), "vazirmatn".to_owned()],
    );
    defs.families.insert(
        typography::bold_family(),
        vec!["nunito-bold".to_owned(), "vazirmatn".to_owned()],
    );

    // Monospace keeps egui's font (box-drawing glyphs) + fallback.
    defs.families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .push("vazirmatn".to_owned());

    defs
}

/// Push the whole MoonKit look onto `ctx`'s style for `mode`.
pub fn apply(ctx: &egui::Context, mode: Mode) {
    let p = crate::ui::widgets::pal();
    install_fonts(ctx);

    // 0.34 renamed Context::style/set_style to global_style/set_global_style
    // (0.35 removed the old names).
    let mut style = (*ctx.global_style()).clone();
    style.visuals = visuals_for(&p, mode);
    style.text_styles = text_styles().into_iter().collect();

    // Layout rhythm (4pt grid).
    style.spacing.item_spacing = egui::vec2(spacing::S, spacing::S);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.icon_width = 20.0;
    style.spacing.icon_width_inner = 16.0;
    style.spacing.icon_spacing = 4.0;
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.combo_width = 140.0;
    style.interaction.tooltip_delay = 0.25;

    // `::-webkit-scrollbar`: width 4px, thumb radius 2 in `--accent`, track
    // effectively invisible. Solid (non-floating) so the content width is
    // reduced by the 4px bar column exactly like the WebView layout.
    let sc = &mut style.spacing.scroll;
    sc.floating = false;
    sc.bar_width = 4.0;
    sc.bar_inner_margin = 0.0;
    sc.bar_outer_margin = 0.0;
    sc.dormant_handle_opacity = 1.0;
    sc.interact_handle_opacity = 1.0;

    ctx.set_global_style(style);
}

fn visuals_for(p: &crate::ui::tokens::Palette, mode: Mode) -> Visuals {
    let mut v = match mode {
        Mode::Light => Visuals::light(),
        Mode::Dark | Mode::LinearDark => Visuals::dark(),
    };

    // Surfaces
    v.panel_fill = p.bg_primary;
    v.window_fill = p.bg_card;
    // Scrollbar track is painted with `extreme_bg_color` when the bar is
    // solid — the old app's `::-webkit-scrollbar-track` is transparent.
    v.extreme_bg_color = egui::Color32::TRANSPARENT;
    v.faint_bg_color = p.hover_veil;
    v.code_bg_color = p.bg_card_inset;
    v.window_corner_radius = egui::CornerRadius::same(RADIUS_CARD);

    // Scrollbar thumb: `::-webkit-scrollbar-thumb {background: var(--scrollbar-thumb)}`
    // — the accent in dark, `rgba(20,24,34,.25)` in light.
    // egui picks the handle color from `widgets.{active,hovered,inactive}
    // .fg_stroke` (foreground_color=true); no native widgets are used, so
    // repointing them is safe and keeps the thumb accent-colored on hover.
    let thumb = if p.is_light {
        egui::Color32::from_rgba_unmultiplied(20, 24, 34, 64)
    } else {
        p.accent
    };
    v.widgets.noninteractive.bg_fill = p.bg_card;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, thumb);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.inactive.bg_fill = p.bg_card_inset;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, thumb);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_fill = p.hover_veil;
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, thumb);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.border_strong);

    // Controls: soft corners everywhere, accent on interaction.
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = egui::CornerRadius::same(RADIUS_CONTROL);
    }
    v.widgets.active.bg_fill = p.hover_veil;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, thumb);

    v.selection.bg_fill = p.accent.linear_multiply(0.35);
    v.selection.stroke = egui::Stroke::new(1.0, p.accent);
    v.hyperlink_color = p.accent;

    v
}

/// Text style table — Heading/card titles use the real-bold Nunito family.
pub fn text_styles() -> [(TextStyle, egui::FontId); 5] {
    [
        (TextStyle::Heading, typography::heading()),
        (TextStyle::Body, typography::body()),
        (TextStyle::Button, typography::body()),
        (TextStyle::Small, typography::small()),
        (TextStyle::Monospace, typography::mono()),
    ]
}

pub const RAIL_W: f32 = spacing::RAIL_W;

#[cfg(test)]
mod tests {
    use super::*;

    fn cjk_width(ctx: &egui::Context) -> f32 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear(); // epaint panics if a delta is dropped unapplied
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                "中文".to_owned(),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            )
            .size()
            .x
        })
    }

    /// The system CJK fonts are the app's single largest allocation; they must
    /// cost nothing until something actually paints CJK.
    #[test]
    fn cjk_fonts_cost_nothing_until_asked() {
        if !std::path::Path::new(r"C:\Windows\Fonts\msyh.ttc").exists() {
            return; // stripped Windows image: nothing to load
        }
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let tofu = cjk_width(&ctx);
        install_cjk_fonts(&ctx);
        let real = cjk_width(&ctx);
        assert!(real > tofu + 10.0, "CJK glyphs did not arrive: {tofu} -> {real}");
        install_cjk_fonts(&ctx); // one-shot: no second re-install
        assert_eq!(cjk_width(&ctx), real);
        // Leaving the screen that needed them must give the memory back.
        release_cjk_fonts(&ctx);
        assert!(
            (cjk_width(&ctx) - tofu).abs() < 0.5,
            "CJK fonts were not released"
        );
        release_cjk_fonts(&ctx); // no-op when already released
        assert!((cjk_width(&ctx) - tofu).abs() < 0.5);
    }
}

