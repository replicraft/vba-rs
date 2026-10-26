//! Character-set conversions behind `StrConv`, `Asc` and `Chr`: width
//! (`vbWide`/`vbNarrow`), kana (`vbKatakana`/`vbHiragana`) and ANSI code page
//! encoding (`vbFromUnicode`/`vbUnicode`).
//!
//! Width and kana conversions are pure Unicode transformations, the same
//! character classes Windows' `LCMapString` (`LCMAP_FULLWIDTH`,
//! `LCMAP_HALFWIDTH`, `LCMAP_KATAKANA`, `LCMAP_HIRAGANA`) covers; exact
//! agreement with Windows for rare characters is **unverified**. Code pages
//! are served by `encoding_rs` only; any other page is
//! [`StdError::Unsupported`]. Characters a code page cannot represent
//! encode as `?` in `StrConv(vbFromUnicode)` (the Windows default character;
//! **unverified**), and unpaired surrogates likewise.

use crate::error::{StdError, StdResult};
use encoding_rs::Encoding;
use vba_value::VbaString;

/// The `encoding_rs` encoding for a Windows code page, if supported.
///
/// Supported: 874, 932 (Shift_JIS), 936 (GBK), 949 (EUC-KR), 950 (Big5),
/// 1250-1258, 10000 (Macintosh Roman), 20866 (KOI8-R), 21866 (KOI8-U),
/// 28592-28598 and 28600-28605 and 28915 (ISO-8859-x), 65001 (UTF-8).
pub fn encoding_for(code_page: u32) -> Option<&'static Encoding> {
    let label = match code_page {
        874 => "windows-874",
        932 => "shift_jis",
        936 => "gbk",
        949 => "euc-kr",
        950 => "big5",
        1250..=1258 => return Encoding::for_label(format!("windows-{code_page}").as_bytes()),
        10000 => "macintosh",
        20866 => "koi8-r",
        21866 => "koi8-u",
        28592..=28598 | 28600..=28603 | 28605 => {
            return Encoding::for_label(format!("iso-8859-{}", code_page - 28590).as_bytes());
        }
        28604 => "iso-8859-10",
        28915 => "iso-8859-16",
        65001 => "utf-8",
        _ => return None,
    };
    Encoding::for_label(label.as_bytes())
}

fn unsupported_page() -> StdError {
    StdError::Unsupported("code page not supported by encoding_rs")
}

/// `true` for the double-byte Windows code pages 932, 936, 949 and 950.
pub fn is_double_byte(code_page: u32) -> bool {
    matches!(code_page, 932 | 936 | 949 | 950)
}

/// The ANSI code page of a locale identifier, for the few LCIDs listed here
/// (**unverified** table of well-known pairs): en-US, de-DE, fr-FR, es-ES,
/// it-IT, nl-NL, pt-BR and en-GB are 1252; ja-JP 932; zh-CN 936; ko-KR 949;
/// zh-TW 950; ru-RU 1251; pl-PL 1250; el-GR 1253; tr-TR 1254; he-IL 1255;
/// ar-SA 1256.
pub fn lcid_code_page(lcid: i32) -> Option<u32> {
    Some(match lcid {
        0x0409 | 0x0407 | 0x040C | 0x0C0A | 0x0410 | 0x0413 | 0x0416 | 0x0809 => 1252,
        0x0411 => 932,
        0x0804 => 936,
        0x0412 => 949,
        0x0404 => 950,
        0x0419 => 1251,
        0x0415 => 1250,
        0x0408 => 1253,
        0x041F => 1254,
        0x040D => 1255,
        0x0401 => 1256,
        _ => return None,
    })
}

/// Encodes `s` in `code_page`; unmappable characters become `?`.
pub fn encode(s: &VbaString, code_page: u32) -> StdResult<Vec<u8>> {
    let enc = encoding_for(code_page).ok_or_else(unsupported_page)?;
    let mut out = Vec::new();
    let mut buf = [0u8; 4];
    for r in char::decode_utf16(s.as_units().iter().copied()) {
        let Ok(c) = r else {
            out.push(b'?');
            continue;
        };
        let t = c.encode_utf8(&mut buf);
        let (bytes, _, bad) = enc.encode(t);
        if bad {
            out.push(b'?');
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    Ok(out)
}

/// Decodes `bytes` from `code_page`; invalid sequences become U+FFFD.
pub fn decode(bytes: &[u8], code_page: u32) -> StdResult<VbaString> {
    let enc = encoding_for(code_page).ok_or_else(unsupported_page)?;
    let (text, _) = enc.decode_without_bom_handling(bytes);
    Ok(VbaString::from(text.as_ref()))
}

// ---------------------------------------------------------------------------
// Width

/// Fullwidth forms of U+FF61..=U+FF9F in order.
const HALF_KANA_TO_FULL: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";

/// Kana (fullwidth) that take a voiced mark by adding 1 to the code point.
const VOICEABLE: &str = "カキクケコサシスセソタチツテトハヒフヘホ";
/// Kana (fullwidth) that take a semi-voiced mark by adding 2.
const SEMI_VOICEABLE: &str = "ハヒフヘホ";

/// Other halfwidth/fullwidth symbol pairs (U+00A2.. to U+FFE0..).
const SYMBOLS: [(char, char); 7] = [
    ('\u{A2}', '\u{FFE0}'),
    ('\u{A3}', '\u{FFE1}'),
    ('\u{AC}', '\u{FFE2}'),
    ('\u{AF}', '\u{FFE3}'),
    ('\u{A6}', '\u{FFE4}'),
    ('\u{A5}', '\u{FFE5}'),
    ('\u{20A9}', '\u{FFE6}'),
];

fn half_kana_full(c: char) -> Option<char> {
    let i = (c as u32).checked_sub(0xFF61)? as usize;
    HALF_KANA_TO_FULL.chars().nth(i)
}

/// `vbWide`: ASCII and halfwidth katakana to fullwidth forms. Halfwidth
/// katakana followed by a (semi-)voiced sound mark combine into one
/// character.
pub fn to_wide(s: &VbaString) -> VbaString {
    let units = s.as_units();
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        let next = units.get(i + 1).copied();
        let c = char::from_u32(u32::from(u));
        let mapped = match (u, c) {
            (0x20, _) => Some('\u{3000}'),
            (0x21..=0x7E, _) => char::from_u32(u32::from(u) - 0x21 + 0xFF01),
            (_, Some(c)) => {
                if let Some(&(_, w)) = SYMBOLS.iter().find(|(h, _)| *h == c) {
                    Some(w)
                } else if (0xFF61..=0xFF9F).contains(&u) {
                    let full = half_kana_full(c).expect("in range");
                    let combined = match next {
                        Some(0xFF9E) if u == 0xFF73 => Some('ヴ'),
                        Some(0xFF9E) if VOICEABLE.contains(full) => char::from_u32(full as u32 + 1),
                        Some(0xFF9F) if SEMI_VOICEABLE.contains(full) => {
                            char::from_u32(full as u32 + 2)
                        }
                        _ => None,
                    };
                    if let Some(k) = combined {
                        i += 1;
                        Some(k)
                    } else {
                        Some(full)
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        match mapped {
            Some(m) => out.push(m as u32 as u16),
            None => out.push(u),
        }
        i += 1;
    }
    VbaString::from_units(out)
}

/// `vbNarrow`: fullwidth ASCII forms and katakana to halfwidth.
pub fn to_narrow(s: &VbaString) -> VbaString {
    let mut out: Vec<u16> = Vec::with_capacity(s.len());
    let half_of = |full: char| -> Option<u16> {
        HALF_KANA_TO_FULL
            .chars()
            .position(|f| f == full)
            .map(|i| 0xFF61 + i as u16)
    };
    for &u in s.as_units() {
        let Some(c) = char::from_u32(u32::from(u)) else {
            out.push(u);
            continue;
        };
        match u {
            0x3000 => out.push(0x20),
            0xFF01..=0xFF5E => out.push(u - 0xFF01 + 0x21),
            _ => {
                if let Some(&(h, _)) = SYMBOLS.iter().find(|(_, w)| *w == c) {
                    out.push(h as u32 as u16);
                } else if let Some(h) = half_of(c) {
                    out.push(h);
                } else if c == 'ヴ' {
                    out.extend([0xFF73, 0xFF9E]);
                } else if let Some(base) = VOICEABLE
                    .chars()
                    .find(|b| *b as u32 + 1 == c as u32)
                    .and_then(half_of)
                {
                    out.extend([base, 0xFF9E]);
                } else if let Some(base) = SEMI_VOICEABLE
                    .chars()
                    .find(|b| *b as u32 + 2 == c as u32)
                    .and_then(half_of)
                {
                    out.extend([base, 0xFF9F]);
                } else {
                    out.push(u);
                }
            }
        }
    }
    VbaString::from_units(out)
}

// ---------------------------------------------------------------------------
// Kana

/// `vbKatakana`: hiragana (U+3041..=U+3096, U+309D, U+309E) to katakana.
pub fn to_katakana(s: &VbaString) -> VbaString {
    VbaString::from_units(
        s.as_units()
            .iter()
            .map(|&u| match u {
                0x3041..=0x3096 | 0x309D | 0x309E => u + 0x60,
                _ => u,
            })
            .collect(),
    )
}

/// `vbHiragana`: katakana (U+30A1..=U+30F6, U+30FD, U+30FE) to hiragana.
/// The prolonged sound mark and halfwidth katakana are unchanged.
pub fn to_hiragana(s: &VbaString) -> VbaString {
    VbaString::from_units(
        s.as_units()
            .iter()
            .map(|&u| match u {
                0x30A1..=0x30F6 | 0x30FD | 0x30FE => u - 0x60,
                _ => u,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> VbaString {
        VbaString::from(x)
    }

    #[test]
    fn wide_and_narrow() {
        assert_eq!(to_wide(&s("Ab 1!~")), s("Ａｂ　１！～"));
        assert_eq!(to_narrow(&s("Ａｂ　１！～")), s("Ab 1!~"));
        assert_eq!(to_wide(&s("ｶﾞｷﾞﾊﾟｳﾞｱ")), s("ガギパヴア"));
        assert_eq!(to_narrow(&s("ガギパヴア")), s("ｶﾞｷﾞﾊﾟｳﾞｱ"));
        assert_eq!(
            to_wide(&s("ｱｲｳｴｵ｡｢｣､･ｦｧｰﾝ")),
            s("アイウエオ。「」、・ヲァーン")
        );
        assert_eq!(to_wide(&s("ﾞ")), s("゛"));
        assert_eq!(to_wide(&s("¥")), s("￥"));
        assert_eq!(to_narrow(&s("￥")), s("¥"));
        // Unpaired surrogates survive.
        let lone = VbaString::from_units(vec![0xD800, 0x41]);
        assert_eq!(to_wide(&lone).as_units(), &[0xD800, 0xFF21]);
        // Every halfwidth katakana round-trips through wide and narrow.
        for u in 0xFF61u16..=0xFF9F {
            if u == 0xFF9E || u == 0xFF9F {
                continue;
            }
            let h = VbaString::from_units(vec![u]);
            assert_eq!(to_narrow(&to_wide(&h)), h, "{u:#x}");
        }
        // All ASCII round-trips.
        let ascii: String = (0x20u8..0x7F).map(char::from).collect();
        assert_eq!(to_narrow(&to_wide(&s(&ascii))), s(&ascii));
    }

    #[test]
    fn kana() {
        assert_eq!(to_katakana(&s("ひらがな")), s("ヒラガナ"));
        assert_eq!(to_hiragana(&s("カタカナ")), s("かたかな"));
        assert_eq!(to_hiragana(&s("ヴーabc")), s("ゔーabc"));
        assert_eq!(to_katakana(&s("ゝゞ")), s("ヽヾ"));
        assert_eq!(to_hiragana(&s("ﾊﾝ")), s("ﾊﾝ"));
    }

    #[test]
    fn code_pages() {
        assert_eq!(encode(&s("aé€"), 1252).unwrap(), [0x61, 0xE9, 0x80]);
        assert_eq!(encode(&s("Ω"), 1252).unwrap(), [b'?']);
        assert_eq!(encode(&s("あ"), 932).unwrap(), [0x82, 0xA0]);
        assert_eq!(decode(&[0x82, 0xA0], 932).unwrap(), s("あ"));
        assert_eq!(decode(&[0xE9], 1252).unwrap(), s("é"));
        assert_eq!(decode(&[0xC0], 1251).unwrap(), s("А"));
        assert_eq!(encode(&s("é"), 65001).unwrap(), [0xC3, 0xA9]);
        assert!(matches!(
            encode(&s("a"), 437),
            Err(StdError::Unsupported(_))
        ));
        assert_eq!(lcid_code_page(0x0409), Some(1252));
        assert_eq!(lcid_code_page(0x0411), Some(932));
        assert_eq!(lcid_code_page(0x7777), None);
        for cp in [
            874, 932, 936, 949, 950, 1250, 1258, 10000, 20866, 28592, 28605, 65001,
        ] {
            assert!(encoding_for(cp).is_some(), "{cp}");
        }
    }
}
