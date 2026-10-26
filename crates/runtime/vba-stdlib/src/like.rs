//! The `Like` operator (MS-VBAL §5.6.9.5 and the Microsoft language
//! reference).
//!
//! Pattern characters: `?` any one character, `*` zero or more characters,
//! `#` one digit (`0`-`9`), `[charlist]` any one character in the list,
//! `[!charlist]` any one character not in the list. A list holds characters
//! and ranges `a-z` in ascending order. `-` is literal at the start or end of
//! a list, `!` is literal when not first, and `?*#[` are literal inside a
//! list. The sequence `[]` matches the empty string (consumes nothing).
//!
//! Errors: an unterminated `[`, or a descending range, is error 93 "Invalid
//! pattern string" (the descending-range rule is **doc**-derived and
//! unverified).
//!
//! With `Option Compare Text` characters are compared after simple case
//! folding; Windows collation is not reproduced.

use crate::error::{INVALID_PATTERN, StdError, StdResult};
use crate::strings::upper;
use vba_value::collation::fold_unit as cfold;
use vba_value::{CompareMode, VbaResult, VbaString};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Lit(u16),
    One,
    Digit,
    Star,
    Empty,
    Set {
        negate: bool,
        items: Vec<(u16, u16)>,
    },
}

fn parse(pattern: &[u16]) -> VbaResult<Vec<Tok>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < pattern.len() {
        let c = pattern[i];
        i += 1;
        match c {
            0x3F => out.push(Tok::One),
            0x23 => out.push(Tok::Digit),
            0x2A => {
                if out.last() != Some(&Tok::Star) {
                    out.push(Tok::Star);
                }
            }
            0x5B => {
                let close = pattern[i..]
                    .iter()
                    .position(|&u| u == 0x5D)
                    .ok_or(INVALID_PATTERN)?;
                let body = &pattern[i..i + close];
                i += close + 1;
                if body.is_empty() {
                    out.push(Tok::Empty);
                    continue;
                }
                let (negate, body) = match body {
                    [0x21, rest @ ..] if !rest.is_empty() => (true, rest),
                    _ => (false, body),
                };
                let mut items = Vec::new();
                let mut j = 0;
                while j < body.len() {
                    let lo = body[j];
                    if j + 2 < body.len() && body[j + 1] == 0x2D {
                        let hi = body[j + 2];
                        if hi < lo {
                            return Err(INVALID_PATTERN);
                        }
                        items.push((lo, hi));
                        j += 3;
                    } else {
                        items.push((lo, lo));
                        j += 1;
                    }
                }
                out.push(Tok::Set { negate, items });
            }
            _ => out.push(Tok::Lit(c)),
        }
    }
    Ok(out)
}

/// The unaccented lower-case base of an accented Latin-1 Supplement or Latin
/// Extended-A letter; any other code unit (ligatures, eth, thorn and sharp s
/// have no single base) is returned unchanged.
fn accent_base(u: u16) -> u16 {
    let base = match u {
        0xC0..=0xC5 | 0xE0..=0xE5 | 0x100..=0x105 => 'a',
        0xC7 | 0xE7 | 0x106..=0x10D => 'c',
        0x10E..=0x111 => 'd',
        0xC8..=0xCB | 0xE8..=0xEB | 0x112..=0x11B => 'e',
        0x11C..=0x123 => 'g',
        0x124..=0x127 => 'h',
        0xCC..=0xCF | 0xEC..=0xEF | 0x128..=0x131 => 'i',
        0x134 | 0x135 => 'j',
        0x136 | 0x137 => 'k',
        0x139..=0x142 => 'l',
        0xD1 | 0xF1 | 0x143..=0x148 => 'n',
        0xD2..=0xD6 | 0xD8 | 0xF2..=0xF6 | 0xF8 | 0x14C..=0x151 => 'o',
        0x154..=0x159 => 'r',
        0x15A..=0x161 => 's',
        0x162..=0x167 => 't',
        0xD9..=0xDC | 0xF9..=0xFC | 0x168..=0x173 => 'u',
        0x174 | 0x175 => 'w',
        0xDD | 0xFD | 0xFF | 0x176..=0x178 => 'y',
        0x179..=0x17E => 'z',
        _ => return u,
    };
    base as u16
}

fn matches_one(tok: &Tok, c: u16, mode: CompareMode) -> bool {
    match tok {
        Tok::Lit(l) => match mode {
            CompareMode::Binary => *l == c,
            CompareMode::Text => cfold(*l) == cfold(c),
        },
        Tok::One => true,
        Tok::Digit => (0x30..=0x39).contains(&c),
        Tok::Set { negate, items } => {
            let hit = |x: u16| items.iter().any(|&(lo, hi)| lo <= x && x <= hi);
            let found = match mode {
                CompareMode::Binary => hit(c),
                CompareMode::Text => {
                    let t = |x: u16| {
                        items.iter().any(|&(lo, hi)| {
                            let (lo, hi) = (cfold(lo), cfold(hi));
                            lo <= x && x <= hi
                        })
                    };
                    // An accented letter sorts right after its base letter, so
                    // it lies in `lo-hi` when its base is in `lo..hi` (the
                    // accented form of `hi` itself is past the end).
                    let accent = || {
                        let f = cfold(c);
                        let b = accent_base(f);
                        b != f
                            && items.iter().any(|&(lo, hi)| {
                                let (lo, hi) = (cfold(lo), cfold(hi));
                                lo <= b && b < hi
                            })
                    };
                    hit(c) || hit(cfold(c)) || hit(upper(c)) || t(cfold(c)) || accent()
                }
            };
            found != *negate
        }
        Tok::Star | Tok::Empty => false,
    }
}

/// `text Like pattern` for non-`Null` operands.
pub fn like(text: &VbaString, pattern: &VbaString, mode: CompareMode) -> VbaResult<bool> {
    match like_metered(text, pattern, mode, &mut |_| Ok(())) {
        Ok(b) => Ok(b),
        Err(StdError::Vba(e)) => Err(e),
        Err(_) => unreachable!("an unmetered match is never cut short"),
    }
}

/// How many comparisons run between two reports to the meter.
const REPORT_EVERY: u64 = 1024;

/// [`like`] that reports the work it does: `meter` is called with the number
/// of comparisons done so far (in batches) and returns an error to stop the
/// match, which then fails with that error (see [`crate::WorkMeter`]). Matching a
/// pattern with many `*` against a long text can take quadratic time; this
/// is how the step budget bounds it.
pub fn like_metered(
    text: &VbaString,
    pattern: &VbaString,
    mode: CompareMode,
    meter: &mut dyn FnMut(u64) -> StdResult<()>,
) -> StdResult<bool> {
    let toks = parse(pattern.as_units())?;
    let t = text.as_units();
    let mut work = 0u64;
    // Iterative matcher with single-star backtracking.
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    loop {
        work += 1;
        if work >= REPORT_EVERY {
            meter(work)?;
            work = 0;
        }
        if pi < toks.len() {
            match &toks[pi] {
                Tok::Star => {
                    star = Some((pi, ti));
                    pi += 1;
                    continue;
                }
                Tok::Empty => {
                    pi += 1;
                    continue;
                }
                tok => {
                    if let Tok::Set { items, .. } = tok {
                        work += items.len() as u64;
                    }
                    if ti < t.len() && matches_one(tok, t[ti], mode) {
                        ti += 1;
                        pi += 1;
                        continue;
                    }
                }
            }
        } else if ti == t.len() {
            meter(work)?;
            return Ok(true);
        }
        match star {
            Some((sp, st)) if st < t.len() => {
                star = Some((sp, st + 1));
                pi = sp + 1;
                ti = st + 1;
            }
            _ => {
                meter(work)?;
                return Ok(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(t: &str, p: &str, m: CompareMode) -> VbaResult<bool> {
        like(&VbaString::from(t), &VbaString::from(p), m)
    }
    const B: CompareMode = CompareMode::Binary;
    const T: CompareMode = CompareMode::Text;

    /// Documented examples from the Like operator reference.
    #[test]
    fn documented_examples() {
        assert_eq!(l("aBBBa", "a*a", B), Ok(true));
        assert_eq!(l("F", "[A-Z]", B), Ok(true));
        assert_eq!(l("F", "[!A-Z]", B), Ok(false));
        assert_eq!(l("a2a", "a#a", B), Ok(true));
        assert_eq!(l("aM5b", "a[L-P]#[!c-e]", B), Ok(true));
        assert_eq!(l("BAT123khg", "B?T*", B), Ok(true));
        assert_eq!(l("CAT123khg", "B?T*", B), Ok(false));
        assert_eq!(l("ab", "a[]b", B), Ok(true));
    }

    /// Remaining examples of the reference page (Microsoft Learn, Like
    /// operator), including the documented error 93.
    #[test]
    fn documented_examples_with_brackets() {
        assert_eq!(l("ab", "a*b", B), Ok(true));
        assert_eq!(l("a*b", "a [*]b", B), Ok(false));
        assert_eq!(l("axxxxxb", "a [*]b", B), Ok(false));
        assert_eq!(l("a [xyz", "a [[]*", B), Ok(true));
        assert_eq!(l("a [xyz", "a [*", B), Err(INVALID_PATTERN));
        // `[A-E]` in text mode matches A a B b E e; accented letters sort
        // after the unaccented ones, so the page says Ê/ê do not match.
        for c in ["A", "a", "B", "b", "E", "e"] {
            assert_eq!(l(c, "[A-E]", T), Ok(true), "{c}");
        }
        assert_eq!(l("\u{ca}", "[A-E]", T), Ok(false));
        assert_eq!(l("\u{ea}", "[A-E]", T), Ok(false));
        // Binary: A, B and E only (and not a).
        assert_eq!(l("a", "[A-E]", B), Ok(false));
        // The page says À and à also match [A-E] in text mode (accents are
        // ordered next to their base letter).
        assert_eq!(l("\u{c0}", "[A-E]", T), Ok(true));
        assert_eq!(l("\u{e0}", "[A-E]", T), Ok(true));
        assert_eq!(l("\u{c0}", "[A-E]", B), Ok(false));
    }

    #[test]
    fn text_mode_and_errors() {
        assert_eq!(l("ABC", "abc", B), Ok(false));
        assert_eq!(l("ABC", "abc", T), Ok(true));
        assert_eq!(l("B", "[a-c]", T), Ok(true));
        assert_eq!(l("x", "[a-", B), Err(INVALID_PATTERN));
        assert_eq!(l("x", "[z-a]", B), Err(INVALID_PATTERN));
        assert_eq!(l("", "*", B), Ok(true));
        assert_eq!(l("", "", B), Ok(true));
        assert_eq!(l("-", "[a-]", B), Ok(true));
        assert_eq!(l("*", "[*]", B), Ok(true));
        assert_eq!(l("a", "[!]", B), Ok(false));
        assert_eq!(l("!", "[!]", B), Ok(true));
    }
}
