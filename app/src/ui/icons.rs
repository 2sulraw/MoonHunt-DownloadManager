//! Material Icons integration — the old app renders every icon through the
//! Material Icons ligature font; we bundle the exact same TTF and map icon
//! names to codepoints so glyph shapes are identical.

use egui::{Color32, FontFamily, FontId, Pos2};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const FAMILY: &str = "mi";
// The codepoints file is the *Regular* font's map, so the Regular TTF must be
// loaded with it — the Old TTF has different codepoints for several names
// (e.g. `inbox` rendered as a box glyph).
pub const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/MaterialIcons-Classic.ttf");
pub const CODEPOINTS_BYTES: &[u8] =
    include_bytes!("../assets/fonts/MaterialIcons-Regular.codepoints");

/// The MoonHunt mark (`src-tauri/icons/128x128@2x.png`, cropped to the glyph,
/// brightened, 64 px). The taskbar only honours a window icon, so the window
/// gets this one; the exe keeps the multi-size `.ico` for Explorer.
pub const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

/// Window icon, decoded once and handed to `ViewportBuilder::with_icon`.
pub fn logo_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(LOGO_BYTES).ok()?.to_rgba8();
    let (width, height) = (img.width(), img.height());
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

/// Name → codepoint (lazily parsed from the bundled codepoints file once).
fn map() -> &'static HashMap<&'static str, char> {
    static MAP: OnceLock<HashMap<&'static str, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let text = std::str::from_utf8(CODEPOINTS_BYTES).unwrap_or_default();
        text.lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let name = parts.next()?;
                let hex = parts.next()?;
                u32::from_str_radix(hex.trim_start_matches("0x"), 16)
                    .ok()
                    .and_then(|cp| char::from_u32(cp))
                    .map(|c| (name, c))
            })
            .collect()
    })
}

/// Codepoint char for an icon name; unknown names fall back to crop_square.
pub fn glyph(name: &str) -> char {
    // `monitoring` only exists in Material Symbols; closest classic match.
    let name = if name == "monitoring" { "query_stats" } else { name };
    map().get(name).copied().unwrap_or('\u{e3c6}')
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY.into()))
}

/// Paint one Material icon centered at `pos`.
pub fn paint_centered(p: &egui::Painter, pos: Pos2, name: &str, size: f32, color: Color32) {
    p.text(
        pos,
        egui::Align2::CENTER_CENTER,
        glyph(name).to_string(),
        font(size),
        color,
    );
}

#[cfg(test)]
mod tests {
    /// The icon family must resolve the Material codepoints through *itself*
    /// (1em advance), not by falling back to a text font.
    #[test]
    fn material_family_resolves_glyphs() {
        let ctx = egui::Context::default();
        crate::ui::theme::install_fonts(&ctx);
        // epaint 0.37 panics when a font atlas delta is dropped unapplied.
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();

        let mi = egui::FontId::new(20.0, egui::FontFamily::Name(super::FAMILY.into()));
        let cp = super::glyph("cloud_download");
        ctx.fonts_mut(|f| {
            assert!(f.has_glyph(&mi.clone(), cp), "icon font lacks U+{:04X}", cp as u32);
            let w = f
                .layout_no_wrap(cp.to_string(), mi.clone(), egui::Color32::WHITE)
                .size()
                .x;
            assert!((w - 20.0).abs() < 0.5, "icon advance should be 1em, got {w}");
        });

        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    }

}
