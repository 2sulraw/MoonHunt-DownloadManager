"""Regenerate app/src/rtl_forms.rs from the UCD Arabic Presentation Forms.

Run: python3 scripts/gen_rtl_forms.py
"""
import pathlib
import unicodedata as ud

from fontTools.ttLib import TTFont

# Only keep forms the bundled font can actually draw: Vazirmatn has no glyph
# for the extended (Urdu/Pashto) presentation forms, so emitting one paints a
# tofu box. `shape()` already treats `0` as "use the base letter instead".
FONT = pathlib.Path("app/src/assets/fonts/Vazirmatn-Regular.ttf")
HAVE = set(TTFont(FONT, lazy=True).getBestCmap())

forms, ligs = {}, {}
for cp in list(range(0xFB50, 0xFE00)) + list(range(0xFE70, 0xFF00)):
    if cp not in HAVE:
        continue
    d = ud.decomposition(chr(cp))
    if not d.startswith("<"):
        continue
    tag, rest = d.split(">", 1)
    tag, parts = tag.strip("<> "), rest.split()
    if len(parts) == 1 and tag in ("isolated", "final", "initial", "medial"):
        forms.setdefault(int(parts[0], 16), {})[tag] = cp
    elif len(parts) == 2 and tag in ("isolated", "final"):
        a, b = int(parts[0], 16), int(parts[1], 16)
        # lam-alef only — the rest are optional GSUB substitutions.
        if a == 0x644 and b in (0x622, 0x623, 0x625, 0x627):
            ligs.setdefault((a, b), {})[tag] = cp

def rows(items):
    return "\n".join(
        "    (" + ", ".join(f"0x{x:04X}" for x in it) + ")," for it in items
    )

body = f"""//! Generated from the Unicode Character Database decompositions
//! (`UnicodeData.txt` <isolated>/<final>/<initial>/<medial> mappings) — do not
//! hand-edit. `0` means the form does not exist for that letter.
//!
//! Regenerate with `scripts/gen_rtl_forms.py`.

/// `(base, isolated, final, initial, medial)`
pub const FORMS: &[(u32, u32, u32, u32, u32)] = &[
{rows((b, f.get("isolated", 0), f.get("final", 0), f.get("initial", 0), f.get("medial", 0)) for b, f in sorted(forms.items()))}
];

/// `(first, second, isolated, final)` — mandatory Arabic ligatures (lam-alef).
pub const LIGATURES: &[(u32, u32, u32, u32)] = &[
{rows((a, b, v.get("isolated", 0), v.get("final", 0)) for (a, b), v in sorted(ligs.items()) if v.get("isolated") or v.get("final"))}
];
"""
pathlib.Path("app/src/rtl_forms.rs").write_text(body, encoding="utf-8")
print("wrote app/src/rtl_forms.rs")
