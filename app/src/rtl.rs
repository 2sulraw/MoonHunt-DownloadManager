//! RTL text seam for egui.
//!
//! epaint 0.37 *shapes* a run with `harfrust` (a HarfBuzz port): it picks the
//! joined forms, mirrors brackets and emits an RTL run's glyphs right to left.
//! What it has no notion of is bidi — runs are emitted in *logical* order — and
//! a sentence arrives as one run per word, because runs break wherever the font
//! changes and the stack resolves every space, digit and Latin letter to Nunito
//! while a Persian letter falls through to Vazirmatn. Handed logical text, each
//! word shapes perfectly and the words read left to right:
//! `هنوز دانلودی وجود ندارد` paints as `ندارد وجود دانلود هنوز`.
//!
//! So this module supplies the ordering epaint lacks, one level above the
//! shaper: split the string into space-separated units, emit the units back to
//! front, and leave every unit's letters untouched for the shaper to reverse.
//! `visual()` also strips the bidi controls (invisible format characters the
//! Vue strings carry; Vazirmatn has no glyph for them, so they paint as tofu).
//!
//! ponytail: a unit reversal, not UAX #9. A token that mixes scripts without a
//! space (`URLهای`) stays one unit and takes the Latin direction; two strings
//! in the fa table do that, which is cheaper than a bidi pass here.

/// Locales the old Vue app loaded as RTL (`src/i18n/index.ts`).
pub fn is_rtl(lang: &str) -> bool {
    matches!(lang, "fa" | "ar" | "he" | "ur")
}

/// True when `s` holds a character from an Arabic-script (cursive, right-to-left)
/// block — text that only shapes correctly when it is laid out as one run.
pub fn has_rtl(s: &str) -> bool {
    s.chars().any(|c| is_arabic_block(c as u32))
}

/// True when `s` holds an Arabic-script *letter* — the thing that decides which
/// way a word reads. The block also holds punctuation (`،` `؟`), which must not
/// drag a Latin word into RTL: `Chrome،` is still a Latin word.
fn has_rtl_letters(s: &str) -> bool {
    s.chars().any(|c| c.is_alphabetic() && is_arabic_block(c as u32))
}

fn is_arabic_block(c: u32) -> bool {
    matches!(
        c,
        0x0590..=0x05FF | 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF
            | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
    )
}

/// Text as the painter should receive it: no bidi controls, units in display
/// order, letterforms left to the shaper.
pub fn visual(s: &str) -> String {
    let clean = strip_controls(s);
    if has_rtl_letters(&clean) {
        units_back_to_front(&clean)
    } else {
        clean
    }
}

/// Invisible format characters — no glyph in Vazirmatn, so they paint as tofu.
/// ZWNJ (U+200C) and ZWJ (U+200D) are *not* controls: they decide how letters
/// join and the shaper needs them.
fn strip_controls(s: &str) -> String {
    fn is_control(c: char) -> bool {
        matches!(c as u32, 0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
    }
    if !s.chars().any(is_control) {
        return s.to_owned();
    }
    s.chars().filter(|c| !is_control(*c)).collect()
}

/// Emit the space-separated units back to front.
///
/// Each word is a unit; consecutive Latin/digit tokens stay in one unit
/// (`Load unpacked` is read left to right even inside a Persian sentence), and
/// punctuation written against a word travels with it. Separators keep their
/// slots, so no double or swallowed spaces appear.
fn units_back_to_front(s: &str) -> String {
    let parts: Vec<&str> = s.split(' ').collect();
    let mut units: Vec<Vec<&str>> = Vec::new();
    for part in parts.iter().copied().filter(|p| !p.is_empty()) {
        match units.last_mut() {
            Some(unit) if !has_rtl_letters(part) && !has_rtl_letters(unit[0]) => unit.push(part),
            _ => units.push(vec![part]),
        }
    }
    let mut words = units.iter().rev().flat_map(|u| u.iter());
    let mut out = String::with_capacity(s.len());
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if !part.is_empty() {
            out.push_str(words.next().copied().unwrap_or_default());
        }
    }
    out
}

/// Shorten `text` until `fits` accepts it, marking the cut.
///
/// `text` is display order (see `visual`), so a Persian sentence's *end* sits at
/// the start of the string: that is the end CSS `text-overflow: ellipsis` clips
/// in an RTL layout. Latin text still loses its tail.
pub fn truncate(text: &str, fits: impl Fn(&str) -> bool) -> String {
    truncate_with(text, "\u{2026}", fits)
}

/// `truncate` with a caller-chosen ellipsis.
pub fn truncate_with(text: &str, ell: &str, fits: impl Fn(&str) -> bool) -> String {
    if fits(text) {
        return text.to_owned();
    }
    let rtl = has_rtl_letters(text);
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        if rtl {
            chars.remove(0);
        } else {
            chars.pop();
        }
        let rest: String = chars.iter().collect();
        let cand = if rtl {
            format!("{ell}{rest}")
        } else {
            format!("{rest}{ell}")
        };
        if fits(&cand) {
            return cand;
        }
    }
    ell.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_locales() {
        for lang in ["fa", "ar", "he", "ur"] {
            assert!(is_rtl(lang), "{lang}");
        }
        for lang in ["en", "fr", "ja", "zh", "ru", "es", "pt"] {
            assert!(!is_rtl(lang), "{lang}");
        }
    }

    #[test]
    fn units_are_ordered_for_the_painter() {
        // The painter emits runs in logical order (epaint has no bidi), so the
        // words have to arrive back to front. Letters stay logical.
        for (logical, painted) in [
            ("حداکثر سرعت دانلود کلی", "کلی دانلود سرعت حداکثر"),
            ("پورت 6802", "6802 پورت"),
            ("خاموش — کیفیت بالا", "بالا کیفیت — خاموش"),
            // A Latin phrase stays one unit; a Latin word next to Persian moves.
            ("افزونه Load unpacked", "Load unpacked افزونه"),
            ("مرورگر Chrome، Edge یا Brave", "Brave یا Chrome، Edge مرورگر"),
        ] {
            assert_eq!(visual(logical), painted, "{logical}");
        }
        // One word, or no RTL at all: nothing to order.
        for s in ["دانلود", "Download", "Load unpacked"] {
            assert_eq!(visual(s), s, "{s}");
        }
    }

    #[test]
    fn bidi_controls_are_not_painted() {
        assert_eq!(visual("\u{2067}سلام دنیا\u{2069}"), "دنیا سلام");
        // ZWNJ must survive: it keeps `ها` from joining the word.
        assert_eq!(visual("لینک\u{200C}ها"), "لینک\u{200C}ها");
    }

    #[test]
    fn truncate_cuts_the_sentence_end() {
        // Display order, so the sentence's end is at the start of the string.
        let vis = visual("دانلودهای فعال");
        let out = truncate(&vis, |s| s.chars().count() <= 8);
        assert!(out.starts_with('\u{2026}'), "{out}");
        let tail = &out['\u{2026}'.len_utf8()..];
        assert!(vis.ends_with(tail), "{out}");
        assert!(out.chars().count() <= 8, "{out}");
        // Latin text still loses its tail.
        let name = "a-long-filename.zip";
        let cut = truncate(name, |s| s.chars().count() <= 8);
        assert!(cut.ends_with('\u{2026}'), "{cut}");
        assert!(name.starts_with(&cut[..cut.len() - '\u{2026}'.len_utf8()]), "{cut}");
    }

    /// Characters that actually reach the screen, in left-to-right order.
    ///
    /// epaint appends zero-advance placeholder glyphs with no atlas allocation
    /// for cursor mapping (`emit_continuation_glyphs`), so those are filtered
    /// out — only glyphs with a real texture rect are painted.
    fn painted_chars(text: &str) -> Vec<char> {
        let ctx = egui::Context::default();
        crate::ui::theme::install_fonts(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
        out.textures_delta.clear(); // epaint panics if a delta is dropped unapplied
        let font = egui::FontId::proportional(16.0);
        let galley =
            ctx.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font, egui::Color32::WHITE));
        let mut glyphs: Vec<(f32, char)> = galley
            .rows
            .iter()
            .flat_map(|r| r.row.glyphs.iter())
            .filter(|g| g.uv_rect.size.x > 0.0)
            .map(|g| (g.pos.x, g.chr))
            .collect();
        glyphs.sort_by(|a, b| a.0.total_cmp(&b.0));
        glyphs.into_iter().map(|(_, c)| c).collect()
    }

    #[test]
    fn epaint_shapes_a_word_and_orders_its_glyphs() {
        // What epaint does get right: the shaper reverses an RTL run's glyphs,
        // so a single word needs nothing from us. If this fails, `harfrust` is
        // gone and the shaping has to come back here.
        assert_eq!(painted_chars("تنظیمات"), "تامیظنت".chars().collect::<Vec<_>>());
        assert_eq!(painted_chars("aria2"), "aria2".chars().collect::<Vec<_>>());
    }

    #[test]
    fn epaint_does_not_order_words() {
        // Why `visual()` reverses units at all: the space resolves to Nunito and
        // the letters to Vazirmatn, so each word is its own run and the runs
        // come out in logical order — the "words back to front" report. If
        // epaint ever grows bidi, this test is the one to delete.
        let painted: String = painted_chars("هنوز دانلود ندارد").into_iter().collect();
        assert_eq!(painted, "زونهدولناددرادن");
    }

    #[test]
    fn a_whole_sentence_paints_right_to_left() {
        let s = "هنوز دانلودی وجود ندارد";
        let painted: String = painted_chars(&visual(s)).into_iter().collect();
        // Painted left to right, a correct RTL sentence is its logical
        // characters in reverse (spaces have no ink).
        let want: String = s.chars().filter(|c| !c.is_whitespace()).rev().collect();
        assert_eq!(painted, want);
    }
}
