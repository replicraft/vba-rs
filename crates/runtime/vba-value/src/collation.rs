//! Text comparison (`Option Compare Text`, `vbTextCompare`).
//!
//! VBA's text comparison calls the Windows `CompareString` family with the
//! flags `NORM_IGNORECASE | NORM_IGNOREKANATYPE | NORM_IGNOREWIDTH` for the
//! user locale. Reproducing Windows' locale tables is out of scope. A
//! [`Collation`] abstracts the decision so a host can supply its own; the
//! provided [`EnUs`] collation is an **explicit approximation** of that
//! behavior for en-US, **not** an LCID-exact implementation:
//!
//! * case: Unicode simple case folding of the BMP (lower-casing plus the
//!   `CaseFolding.txt` common/simple exceptions listed in [`fold_unit`]);
//! * width: fullwidth ASCII (U+FF01..U+FF5E) and the ideographic space fold to
//!   their ASCII forms; halfwidth katakana fold to fullwidth katakana (no
//!   combining of a base with a following voiced mark);
//! * kana type: katakana (U+30A1..U+30F6) folds to hiragana;
//! * accents are significant (no `NORM_IGNORENONSPACE`), but accented letters
//!   are NOT ordered next to their base letters: ordering is the order of the
//!   folded UTF-16 code units. Symbols, digits and letters therefore sort in
//!   code point order after lower-casing, which matches Windows only
//!   approximately;
//! * characters outside the BMP are compared by code unit; unpaired
//!   surrogates are unchanged.

use std::cmp::Ordering;

/// A text-comparison strategy.
///
/// `fold` must return a vector of the same length as its input, and strings a
/// collation considers equal must fold to identical vectors: `InStr`,
/// `Replace`, `Split` and `Filter` search in folded text and map positions
/// back one to one.
pub trait Collation {
    /// Folds `units` to a canonical form for equality and searching.
    fn fold(&self, units: &[u16]) -> Vec<u16>;

    /// Orders two strings. The default compares folded text.
    fn compare(&self, a: &[u16], b: &[u16]) -> Ordering {
        self.fold(a).cmp(&self.fold(b))
    }
}

/// The en-US approximation described in the [module documentation](self).
#[derive(Debug, Clone, Copy, Default)]
pub struct EnUs;

/// Shared instance of [`EnUs`].
pub static EN_US: EnUs = EnUs;

impl Collation for EnUs {
    fn fold(&self, units: &[u16]) -> Vec<u16> {
        units.iter().map(|&u| fold_unit(u)).collect()
    }
}

/// Fullwidth forms of U+FF61..=U+FF9F (halfwidth punctuation and katakana) in
/// order.
pub const HALF_KANA_TO_FULL: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// Case-folding exceptions to plain lower-casing (`CaseFolding.txt`, status
/// C or S; a subset: the BMP letters where lower-casing alone disagrees).
const FOLD_EXCEPTIONS: [(u16, u16); 12] = [
    (0x00B5, 0x03BC),
    (0x017F, 0x0073),
    (0x0345, 0x03B9),
    (0x03C2, 0x03C3),
    (0x03D0, 0x03B2),
    (0x03D1, 0x03B8),
    (0x03D5, 0x03C6),
    (0x03D6, 0x03C0),
    (0x03F0, 0x03BA),
    (0x03F1, 0x03C1),
    (0x03F5, 0x03B5),
    (0x1E9B, 0x1E61),
];

/// Folds one code unit: width, then kana type, then simple case folding.
pub fn fold_unit(u: u16) -> u16 {
    let u = match u {
        0x3000 => 0x20,
        0xFF01..=0xFF5E => u - 0xFEE0,
        0xFF61..=0xFF9F => HALF_KANA_TO_FULL
            .chars()
            .nth(usize::from(u - 0xFF61))
            .map_or(u, |c| c as u32 as u16),
        _ => u,
    };
    let u = match u {
        0x30A1..=0x30F6 => u - 0x60,
        _ => u,
    };
    if let Some(&(_, to)) = FOLD_EXCEPTIONS.iter().find(|(from, _)| *from == u) {
        return to;
    }
    let Some(c) = char::from_u32(u32::from(u)) else {
        return u;
    };
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => u16::try_from(u32::from(l)).unwrap_or(u),
        _ => u,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn folds_case_width_and_kana() {
        let c = EN_US;
        assert_eq!(c.compare(&u("Straße"), &u("STRAßE")), Ordering::Equal);
        assert_eq!(c.compare(&u("ΑΣ"), &u("ας")), Ordering::Equal);
        assert_eq!(c.compare(&u("ABC"), &u("ａｂｃ")), Ordering::Equal);
        assert_eq!(c.compare(&u("a b"), &u("a\u{3000}B")), Ordering::Equal);
        assert_eq!(c.compare(&u("ひらがな"), &u("ヒラガナ")), Ordering::Equal);
        assert_eq!(c.compare(&u("ｱｲｳ"), &u("あいう")), Ordering::Equal);
        assert_eq!(c.compare(&u("ｱｲｳ"), &u("アイウ")), Ordering::Equal);
        // Accents remain significant.
        assert_ne!(c.compare(&u("e"), &u("é")), Ordering::Equal);
        // Ordering is by folded code unit: lower-case sorts after '_' and '['.
        assert_eq!(c.compare(&u("_"), &u("a")), Ordering::Less);
        assert_eq!(c.compare(&u("B"), &u("a")), Ordering::Greater);
        assert_eq!(c.compare(&u("a"), &u("ab")), Ordering::Less);
    }

    #[test]
    fn fold_keeps_length_and_surrogates() {
        let s = vec![0xD800, 0x41, 0xDC00, 0xFF71];
        assert_eq!(EN_US.fold(&s), vec![0xD800, 0x61, 0xDC00, 0x3042]);
        // Idempotent over the whole BMP.
        for x in 0..=0xFFFFu16 {
            let f = fold_unit(x);
            assert_eq!(fold_unit(f), f, "{x:#x}");
        }
    }
}
