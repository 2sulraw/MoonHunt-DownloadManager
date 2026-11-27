//! Language flag textures — the exact PNGs the old `LanguageFlag.vue` bundled,
//! uploaded to egui textures and cached per-context.

use egui::TextureHandle;
use std::collections::HashMap;
use std::sync::OnceLock;

static FLAGS: OnceLock<HashMap<&'static str, &'static [u8]>> = OnceLock::new();

fn flags() -> &'static HashMap<&'static str, &'static [u8]> {
    FLAGS.get_or_init(|| {
        HashMap::from([
            ("en", include_bytes!("../assets/flags/en.png").as_slice()),
            ("fa", include_bytes!("../assets/flags/fa.png").as_slice()),
            ("fr", include_bytes!("../assets/flags/fr.png").as_slice()),
            ("es", include_bytes!("../assets/flags/es.png").as_slice()),
            ("pt", include_bytes!("../assets/flags/pt.png").as_slice()),
            ("ru", include_bytes!("../assets/flags/ru.png").as_slice()),
            ("ja", include_bytes!("../assets/flags/ja.png").as_slice()),
            ("zh", include_bytes!("../assets/flags/zh.png").as_slice()),
            ("ar", include_bytes!("../assets/flags/ar.png").as_slice()),
        ])
    })
}

/// Cached texture handle for a locale flag (28×21 CSS px in the old app).
pub fn texture(ctx: &egui::Context, code: &str) -> Option<TextureHandle> {
    let bytes = *flags().get(code)?;
    let id = egui::Id::new(("lang-flag", code));
    let cached = ctx.data_mut(|d| d.get_temp::<TextureHandle>(id));
    if let Some(t) = cached {
        return Some(t);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], img.into_raw().as_slice());
    let tex = ctx.load_texture(format!("flag-{code}"), color, egui::TextureOptions::LINEAR);
    let texture_store = tex.clone();
    ctx.data_mut(|d| d.insert_temp(id, texture_store));
    Some(tex)
}
