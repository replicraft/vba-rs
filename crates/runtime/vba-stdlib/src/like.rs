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

use crate::error::INVALID_PATTERN;
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
                    hit(c) || hit(cfold(c)) || hit(upper(c)) || t(cfold(c))
                }
            };
            found != *negate
        }
        Tok::Star | Tok::Empty => false,
    }
}

/// `text Like pattern` for non-`Null` operands.
pub fn like(text: &VbaString, pattern: &VbaString, mode: CompareMode) -> VbaResult<bool> {
    let toks = parse(pattern.as_units())?;
    let t = text.as_units();
    // Iterative matcher with single-star backtracking.
    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    loop {
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
                    if ti < t.len() && matches_one(tok, t[ti], mode) {
                        ti += 1;
                        pi += 1;
                        continue;
                    }
                }
            }
        } else if ti == t.len() {
            return Ok(true);
        }
        match star {
            Some((sp, st)) if st < t.len() => {
                star = Some((sp, st + 1));
                pi = sp + 1;
                ti = st + 1;
            }
            _ => return Ok(false),
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
