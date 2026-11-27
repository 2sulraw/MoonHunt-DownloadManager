//! Translations — the old `src/i18n/index.ts` tables flattened into
//! `assets/i18n.json` (`{locale: {english key: text}}`). Keys are the English
//! strings, exactly like the Vue app, so `en` or an unknown key returns the key.
//!
//! RTL locales go through `rtl::visual`, which puts the units in display order
//! for epaint (which shapes a run but has no bidi). The visual strings are
//! leaked once per (locale, key) so `t` can keep returning `&'a str`.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static TABLE: OnceLock<serde_json::Map<String, Value>> = OnceLock::new();

fn table() -> &'static serde_json::Map<String, Value> {
    TABLE.get_or_init(|| serde_json::from_str(include_str!("assets/i18n.json")).unwrap_or_default())
}

/// Logical (unshaped) table value for `key` — the form to splice numbers into
/// before the whole sentence is ordered. `t` returns the *painted* form, and
/// gluing two painted fragments together reverses the sentence.
pub fn raw<'a>(lang: &str, key: &'a str) -> &'a str {
    if lang == "en" {
        return key;
    }
    table()
        .get(lang)
        .and_then(|m| m.get(key))
        .and_then(Value::as_str)
        .unwrap_or(key)
}

/// Visual form of a sentence assembled in logical order (see `raw`).
pub fn show(lang: &str, text: &str) -> String {
    if crate::rtl::is_rtl(lang) {
        visual(lang, text).to_owned()
    } else {
        text.to_owned()
    }
}

/// Translate a message whose head is a table key and whose tail is a detail:
/// `"Failed to pause: <io error>"` → `«توقف ناموفق بود: <io error>»`. The head is
/// looked up logically, the detail is kept verbatim, and the whole sentence is
/// ordered once — detail at the left end, where a Persian sentence ends.
pub fn show_detail(lang: &str, text: &str) -> String {
    match text.split_once(": ") {
        Some((head, detail)) => show(lang, &format!("{}: {detail}", raw(lang, head))),
        None => show(lang, text),
    }
}

/// `t(key)` for the active locale — `normalizeLocale` + `messages[locale][key]`
/// with the key itself as the fallback.
pub fn t<'a>(lang: &str, key: &'a str) -> &'a str {
    let text = raw(lang, key);
    if crate::rtl::is_rtl(lang) {
        visual(lang, text)
    } else {
        text
    }
}

/// Cached visual-order copy of `text`; leaked so the `&'a str` signature holds.
fn visual<'a>(lang: &str, text: &str) -> &'a str {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let key = format!("{lang}\u{1}{text}");
    if let Some(hit) = cache.get(&key) {
        return hit;
    }
    let out: &'static str = Box::leak(crate::rtl::visual(text).into_boxed_str());
    cache.insert(key, out);
    out
}

#[cfg(test)]
mod tests {
    use super::t;

    /// `fa` text comes back shaped and in visual order (see `rtl`), so compare
    /// against the same transformation instead of the raw table value.
    /// `raw` + `show` is the pattern for numbers: translate the frame with its
    /// `{n}` placeholder, splice the value, then order the sentence once. Doing
    /// it the other way round (gluing two already-painted fragments) puts the
    /// words back to front.
    /// Corpus dump for the reference diff: run
    /// `RTL_DUMP=/path/out.tsv cargo test --target-dir target-agent/alt12 -p moonhunt
    /// --bin moonhunt -- --ignored dump_corpus`, then compare that file with
    /// `get_display(reshape(text), base_dir='R')` in Python. Dumping from the real
    /// `visual()` keeps the Python side from re-implementing it (it drifted twice).
    #[test]
    #[ignore]
    fn dump_corpus() {
        let Ok(path) = std::env::var("RTL_DUMP") else {
            return;
        };
        // `RTL_DUMP_LANG=ar` dumps that locale instead of fa.
        let lang = std::env::var("RTL_DUMP_LANG").unwrap_or_else(|_| "fa".to_owned());
        let table = super::table();
        let rows = table
            .get(&lang)
            .and_then(|v| v.as_object())
            .expect("locale table");
        let mut out = String::new();
        for (key, val) in rows {
            let text = val.as_str().unwrap_or_default();
            if text.is_ascii() {
                continue;
            }
            out.push_str(key);
            out.push('\t');
            out.push_str(&crate::rtl::visual(text));
            out.push('\n');
        }
        std::fs::write(path, out).expect("write dump");
    }

    #[test]
    fn message_details_take_the_rtl_side() {
        // Toasts carry `<key>: <io error>`; the head must be translated and the
        // pair kept as one sentence, or Persian users see the English key.
        // Display order puts the sentence's end (the detail) first.
        let out = super::show_detail("fa", "Failed to pause: boom");
        assert_eq!(out, "boom بود: ناموفق توقف");
        // A payload with no key head is passed through untouched.
        assert_eq!(super::show_detail("en", "Download cancelled"), "Download cancelled");
    }

    #[test]
    fn composed_sentences_are_ordered_once() {
        let logical = super::raw("fa", "m ago").replace("{n}", "5");
        let painted = super::show("fa", &logical);
        assert_eq!(painted, "پیش دقیقه 5");
    }

    #[test]
    fn translates_known_keys_and_falls_back() {
        assert_eq!(t("xx", "Downloads"), "Downloads");
        assert_eq!(t("en", "Downloads"), "Downloads");
        // Untranslated key ("Addon" is absent from the old tables).
        assert_eq!(t("fa", "Addon"), "Addon");
        // One word is already its own display order.
        assert_eq!(t("fa", "Downloads"), "دانلودها");
        // Non-RTL locales are untouched.
        assert_eq!(t("ja", "Settings"), "設定");
    }
}
