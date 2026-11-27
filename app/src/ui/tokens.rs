//! MoonKit design tokens — the palette, radius and spacing scale, ported
//! from the CSS variables of the original MoonHunt UI (`App.vue`, light +
//! dark), re-expressed as a typed design system.

use egui::Color32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
    /// OLED-flavoured third mode (`linear-dark` HTML class in the web app).
    LinearDark,
}

impl Mode {
    pub fn parse(s: &str) -> Self {
        match s {
            "light" | "Light" | "light-mode" => Mode::Light,
            "linear-dark" | "LinearDark" => Mode::LinearDark,
            _ => Mode::Dark,
        }
    }
}

/// Accent color presets (ported from the old app's theme system: 16 presets;
/// `--accent` = preset color, `--accent-strong` = preset color in dark modes,
/// the darkened `false_strong` value in light mode).
pub struct AccentPreset {
    pub name: &'static str,
    /// Display name used by the Appearance theme-card picker.
    #[allow(dead_code)]
    pub label: &'static str,
    pub accent: [u8; 3],
    /// Light-mode `--accent-strong` (the `gte` map in the old bundle).
    pub strong_light: [u8; 3],
}

pub const ACCENT_PRESETS: [AccentPreset; 16] = [
    AccentPreset { name: "default", label: "Default", accent: [0x4f, 0xc3, 0xf7], strong_light: [0x00, 0x6f, 0xa1] },
    AccentPreset { name: "cyan", label: "Cyan", accent: [0x1e, 0x88, 0xe5], strong_light: [0x09, 0x71, 0xcd] },
    AccentPreset { name: "purple", label: "Purple", accent: [0xb3, 0x88, 0xff], strong_light: [0x7d, 0x33, 0xa8] },
    AccentPreset { name: "green", label: "Green", accent: [0x81, 0xc7, 0x84], strong_light: [0x2d, 0x7a, 0x30] },
    AccentPreset { name: "red", label: "Red", accent: [0xef, 0x53, 0x50], strong_light: [0xd2, 0x08, 0x05] },
    AccentPreset { name: "gold", label: "Gold", accent: [0xff, 0xd5, 0x4f], strong_light: [0x8a, 0x69, 0x00] },
    AccentPreset { name: "orange", label: "Orange", accent: [0xff, 0x8a, 0x65], strong_light: [0xc6, 0x2e, 0x00] },
    AccentPreset { name: "pink", label: "Pink", accent: [0xf0, 0x62, 0x92], strong_light: [0xd1, 0x05, 0x4a] },
    AccentPreset { name: "gray", label: "Gray", accent: [0x9e, 0x9e, 0x9e], strong_light: [0x6b, 0x6b, 0x6b] },
    AccentPreset { name: "lemon", label: "Lemon", accent: [0xcd, 0xdc, 0x39], strong_light: [0x6a, 0x73, 0x0c] },
    AccentPreset { name: "aqua", label: "Aqua", accent: [0x26, 0xc6, 0xda], strong_light: [0x0d, 0x74, 0x82] },
    AccentPreset { name: "persian-blue", label: "Persian Blue", accent: [0x57, 0xc5, 0xc6], strong_light: [0x22, 0x7b, 0x7c] },
    AccentPreset { name: "teal", label: "Teal", accent: [0x26, 0xa6, 0x9a], strong_light: [0x12, 0x76, 0x6e] },
    AccentPreset { name: "lavender", label: "Lavender", accent: [0x95, 0x75, 0xcd], strong_light: [0x7a, 0x4d, 0xd2] },
    AccentPreset { name: "coral", label: "Coral", accent: [0xff, 0x70, 0x43], strong_light: [0xc6, 0x2e, 0x00] },
    AccentPreset { name: "indigo", label: "Indigo", accent: [0x5e, 0x6a, 0xd2], strong_light: [0x26, 0x34, 0xb0] },
];

/// Resolve (accent, accent_strong) for a (mode, preset name) pair. Empty or
/// unknown names fall back to the first preset (`default`).
pub fn accent_pair(mode: Mode, preset_name: &str) -> (Color32, Color32) {
    let p = ACCENT_PRESETS
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(preset_name))
        .or(Some(&ACCENT_PRESETS[0]))
        .unwrap();
    let a = Color32::from_rgb(p.accent[0], p.accent[1], p.accent[2]);
    let s = match mode {
        Mode::Dark | Mode::LinearDark => a,
        Mode::Light => Color32::from_rgb(p.strong_light[0], p.strong_light[1], p.strong_light[2]),
    };
    (a, s)
}

/// `linear-dark` variant — near-black OLED-ish palette.
pub fn linear_dark() -> Palette {
    Palette {
        bg_primary: Color32::from_rgb(0x08, 0x09, 0x0a),
        bg_surface: Color32::from_rgb(0x0f, 0x10, 0x11),
        bg_card: Color32::from_rgb(0x19, 0x1a, 0x1b),
        bg_card_inset: Color32::from_rgb(0x11, 0x12, 0x13),
        text: Color32::from_rgb(0xf7, 0xf8, 0xf8),
        text_muted: Color32::from_rgb(0xd0, 0xd6, 0xe0),
        text_dim: Color32::from_rgb(0x62, 0x66, 0x6d),
        accent: Color32::from_rgb(0x4f, 0xc3, 0xf7),
        accent_strong: Color32::from_rgb(0x4f, 0xc3, 0xf7),
        hover_veil: Color32::from_rgba_unmultiplied(255, 255, 255, 10),
        border: Color32::from_rgba_unmultiplied(255, 255, 255, 15),
        border_strong: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
        status_active: Color32::from_rgb(0x4f, 0xc3, 0xf7),
        status_complete: Color32::from_rgb(0x81, 0xc7, 0x84),
        status_paused: Color32::from_rgb(0xff, 0xa7, 0x26),
        status_error: Color32::from_rgb(0xef, 0x53, 0x50),
        status_waiting: Color32::from_rgb(0x64, 0xb5, 0xf6),
        status_cancelled: Color32::from_rgb(0x9e, 0x9e, 0x9e),
        is_light: false,
    }
}

/// Semantic color roles. Values come from the original CSS variables so the
/// ports share one brand.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Whole-window backdrop (`--bg-primary`).
    pub bg_primary: Color32,
    /// Panel/toolbar surface (`--bg-surface`).
    pub bg_surface: Color32,
    /// Card fill (`--bg-card`).
    pub bg_card: Color32,
    /// Slightly raised inset fill (inputs, wells).
    pub bg_card_inset: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_dim: Color32,
    /// Accent (`--accent`).
    pub accent: Color32,
    /// Pressed/strong accent (`--accent-strong`).
    pub accent_strong: Color32,
    pub hover_veil: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    // ── status colors (`--status-*`)
    pub status_active: Color32,
    pub status_complete: Color32,
    pub status_paused: Color32,
    pub status_error: Color32,
    pub status_waiting: Color32,
    pub status_cancelled: Color32,
    /// Light theme flag — a few CSS variables (`--toolbar-*`, `--shadow-card`'s
    /// hairline) flip direction between themes and have no palette slot.
    pub is_light: bool,
}

pub fn dark() -> Palette {
    Palette {
    bg_primary: Color32::from_rgb(0x0a, 0x0e, 0x1a),
    bg_surface: Color32::from_rgb(0x16, 0x16, 0x2a),
    bg_card: Color32::from_rgb(0x1e, 0x1e, 0x32),
    bg_card_inset: Color32::from_rgb(0x14, 0x17, 0x28),
    text: Color32::from_rgb(0xff, 0xff, 0xff),
    text_muted: Color32::from_rgb(0x8c, 0x95, 0xb4),
    text_dim: Color32::from_rgb(0x6b, 0x7a, 0x9a),
    accent: Color32::from_rgb(0x4f, 0xc3, 0xf7),
    accent_strong: Color32::from_rgb(0x00, 0x78, 0xad),
    hover_veil: Color32::from_rgba_unmultiplied(255, 255, 255, 15),
    border: Color32::from_rgba_unmultiplied(255, 255, 255, 15),
    border_strong: Color32::from_rgba_unmultiplied(255, 255, 255, 20),
    status_active: Color32::from_rgb(0x4f, 0xc3, 0xf7),
    status_complete: Color32::from_rgb(0x81, 0xc7, 0x84),
    status_paused: Color32::from_rgb(0xff, 0xa7, 0x26),
    status_error: Color32::from_rgb(0xef, 0x53, 0x50),
    status_waiting: Color32::from_rgb(0x64, 0xb5, 0xf6),
        status_cancelled: Color32::from_rgb(0x9e, 0x9e, 0x9e),
        is_light: false,
    }
}

pub fn light() -> Palette {
    Palette {
    bg_primary: Color32::from_rgb(0xf4, 0xf5, 0xf7),
    bg_surface: Color32::from_rgb(0xee, 0xf0, 0xf3),
    bg_card: Color32::from_rgb(0xff, 0xff, 0xff),
    bg_card_inset: Color32::from_rgb(0xf2, 0xf4, 0xf7),
    text: Color32::from_rgb(0x17, 0x19, 0x23),
    text_muted: Color32::from_rgb(0x56, 0x5d, 0x6d),
    text_dim: Color32::from_rgb(0x83, 0x8b, 0x9b),
    accent: Color32::from_rgb(0x00, 0x78, 0xb4),
    accent_strong: Color32::from_rgb(0x00, 0x60, 0x90),
    hover_veil: Color32::from_rgba_unmultiplied(20, 24, 34, 13),
    border: Color32::from_rgba_unmultiplied(20, 24, 34, 23),
    border_strong: Color32::from_rgba_unmultiplied(20, 24, 34, 36),
    status_active: Color32::from_rgb(0x00, 0x78, 0xb4),
    status_complete: Color32::from_rgb(0x23, 0x7a, 0x2a),
    status_paused: Color32::from_rgb(0xa0, 0x60, 0x00),
    status_error: Color32::from_rgb(0xd3, 0x2f, 0x2b),
    status_waiting: Color32::from_rgb(0x1c, 0x72, 0xcc),
    status_cancelled: Color32::from_rgb(0x6b, 0x72, 0x80),
    is_light: true,
    }
}

pub fn palette(mode: Mode) -> Palette {
    match mode {
        Mode::Dark => dark(),
        Mode::LinearDark => linear_dark(),
        Mode::Light => light(),
    }
}

/// Corner radius scale (px) — cards get roomier corners, controls 8, pills 999.
pub const RADIUS_CARD: u8 = 14;
pub const RADIUS_CONTROL: u8 = 8;
pub const RADIUS_PILL: u8 = 99;

/// Spacing scale (px) — 4pt grid.
pub mod spacing {
    pub const S: f32 = 8.0;

    /// Control height for buttons and inputs.
    pub const CONTROL_H: f32 = 30.0;
    /// Left navigation rail width (`--sidebar-width`).
    pub const RAIL_W: f32 = 72.0;
}

pub mod typography {
    use egui::{FontFamily, FontId};

    /// Custom (non-default) font families installed by `theme::font_defs`.
    pub fn bold_family() -> FontFamily {
        FontFamily::Name("nunito-bold".into())
    }
    pub fn semibold_family() -> FontFamily {
        FontFamily::Name("nunito-semibold".into())
    }

    pub fn body() -> FontId {
        FontId::new(13.5, FontFamily::Proportional)
    }
    pub fn small() -> FontId {
        FontId::new(11.5, FontFamily::Proportional)
    }
    pub fn heading() -> FontId {
        FontId::new(19.0, bold_family())
    }
    pub fn mono() -> FontId {
        FontId::monospace(11.5)
    }
}
