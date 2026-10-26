//! Conversion between project text and bytes in a Windows code page.
//!
//! MS-OVBA stores most text as MBCS in the code page named by
//! PROJECTCODEPAGE (§2.3.4.2.1.5). Multi-byte and Windows pages use
//! `encoding_rs`; the common OEM single-byte pages it lacks (437, 737, 775,
//! 850, 852, 855, 857, 858, 860 to 865, 869) use small tables in
//! `codepage_tables` generated from Microsoft's published mapping files.
//! Other pages are reported as [`Error::UnsupportedCodePage`] rather than
//! guessed. The raw bytes of a project are preserved regardless.

use crate::codepage_tables as tables;
use crate::error::Error;
use encoding_rs::Encoding;
use std::sync::{LazyLock, OnceLock};

/// A single-byte code page defined by a table.
struct Table {
    code_page: u16,
    forward: [u16; 256],
    /// `(unicode, byte)` sorted by Unicode value, built on first use. When
    /// several bytes map to one scalar the lowest byte wins.
    reverse: OnceLock<Vec<(u16, u8)>>,
}

impl Table {
    fn new(code_page: u16, forward: [u16; 256]) -> Self {
        Self {
            code_page,
            forward,
            reverse: OnceLock::new(),
        }
    }

    fn reverse(&self) -> &[(u16, u8)] {
        self.reverse.get_or_init(|| {
            let mut v: Vec<(u16, u8)> = (0..=255u8)
                .filter(|&b| self.forward[usize::from(b)] != tables::UNDEF)
                .map(|b| (self.forward[usize::from(b)], b))
                .collect();
            v.sort_unstable();
            v.dedup_by_key(|e| e.0);
            v
        })
    }
}

static TABLES: LazyLock<Vec<Table>> = LazyLock::new(|| {
    let mut cp858 = tables::CP850;
    cp858[0xD5] = 0x20AC; // dotless i of CP850 becomes the euro sign
    vec![
        Table::new(437, tables::CP437),
        Table::new(737, tables::CP737),
        Table::new(775, tables::CP775),
        Table::new(850, tables::CP850),
        Table::new(852, tables::CP852),
        Table::new(855, tables::CP855),
        Table::new(857, tables::CP857),
        Table::new(858, cp858),
        Table::new(860, tables::CP860),
        Table::new(861, tables::CP861),
        Table::new(862, tables::CP862),
        Table::new(863, tables::CP863),
        Table::new(864, tables::CP864),
        Table::new(865, tables::CP865),
        Table::new(869, tables::CP869),
    ]
});

fn table_for(code_page: u16) -> Option<&'static Table> {
    TABLES.iter().find(|t| t.code_page == code_page)
}

/// Whether text in `code_page` can be converted.
pub fn is_supported(code_page: u16) -> bool {
    table_for(code_page).is_some() || encoding_for(code_page).is_ok()
}

/// Returns the `encoding_rs` converter for a Windows code page number.
///
/// Table-based OEM pages are not `encoding_rs` encodings and yield
/// [`Error::UnsupportedCodePage`] here; [`decode`] and [`encode`] handle
/// them.
pub fn encoding_for(code_page: u16) -> Result<&'static Encoding, Error> {
    use encoding_rs::*;
    Ok(match code_page {
        866 => IBM866,
        874 => WINDOWS_874,
        932 => SHIFT_JIS,
        936 => GBK,
        949 => EUC_KR,
        950 => BIG5,
        1250 => WINDOWS_1250,
        1251 => WINDOWS_1251,
        1252 => WINDOWS_1252,
        1253 => WINDOWS_1253,
        1254 => WINDOWS_1254,
        1255 => WINDOWS_1255,
        1256 => WINDOWS_1256,
        1257 => WINDOWS_1257,
        1258 => WINDOWS_1258,
        10000 => MACINTOSH,
        20866 => KOI8_R,
        21866 => KOI8_U,
        28592 => ISO_8859_2,
        28593 => ISO_8859_3,
        28594 => ISO_8859_4,
        28595 => ISO_8859_5,
        28596 => ISO_8859_6,
        28597 => ISO_8859_7,
        28598 => ISO_8859_8,
        28603 => ISO_8859_13,
        28605 => ISO_8859_15,
        51932 => EUC_JP,
        54936 => GB18030,
        65001 => UTF_8,
        other => return Err(Error::UnsupportedCodePage(other)),
    })
}

/// Decodes bytes in `code_page`. Invalid sequences, and bytes a table page
/// leaves undefined, become U+FFFD.
pub fn decode(code_page: u16, bytes: &[u8]) -> Result<String, Error> {
    if let Some(t) = table_for(code_page) {
        return Ok(bytes
            .iter()
            .map(|&b| match t.forward[usize::from(b)] {
                tables::UNDEF => '\u{FFFD}',
                u => char::from_u32(u32::from(u)).unwrap_or('\u{FFFD}'),
            })
            .collect());
    }
    let enc = encoding_for(code_page)?;
    let (text, _) = enc.decode_without_bom_handling(bytes);
    Ok(text.into_owned())
}

/// Encodes `text` in `code_page`, failing if any character is unmappable.
///
/// `what` names the text for the error message.
pub fn encode(code_page: u16, text: &str, what: &str) -> Result<Vec<u8>, Error> {
    let unencodable = || Error::Unencodable {
        what: what.to_owned(),
        code_page,
    };
    if let Some(t) = table_for(code_page) {
        let rev = t.reverse();
        let mut out = Vec::with_capacity(text.len());
        for c in text.chars() {
            let u = u16::try_from(u32::from(c)).map_err(|_| unencodable())?;
            let i = rev
                .binary_search_by_key(&u, |e| e.0)
                .map_err(|_| unencodable())?;
            out.push(rev[i].1);
        }
        return Ok(out);
    }
    let enc = encoding_for(code_page)?;
    let (bytes, _, unmappable) = enc.encode(text);
    if unmappable {
        return Err(unencodable());
    }
    Ok(bytes.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_1252_round_trip() {
        let text = "Größe € ä";
        let bytes = encode(1252, text, "t").unwrap();
        assert_eq!(bytes[0], b'G');
        assert!(bytes.contains(&0x80)); // euro sign
        assert_eq!(decode(1252, &bytes).unwrap(), text);
    }

    #[test]
    fn shift_jis_round_trip() {
        let text = "日本語 ABC";
        let bytes = encode(932, text, "t").unwrap();
        assert!(bytes.len() > 7);
        assert_eq!(decode(932, &bytes).unwrap(), text);
    }

    #[test]
    fn unmappable_is_an_error() {
        assert!(matches!(
            encode(1252, "日本", "module"),
            Err(Error::Unencodable {
                code_page: 1252,
                ..
            })
        ));
    }

    #[test]
    fn unknown_code_page_is_an_error() {
        assert!(matches!(
            decode(10006, b"x"),
            Err(Error::UnsupportedCodePage(10006))
        ));
    }

    #[test]
    fn invalid_bytes_decode_lossily() {
        assert_eq!(decode(65001, &[b'a', 0xFF, b'b']).unwrap(), "a\u{FFFD}b");
    }

    /// `(code page, [(byte, scalar)], undefined bytes)`, read off Microsoft's
    /// `CP<n>.TXT` mapping files (page 858: Python's `cp858`, which differs
    /// from 850 only at 0xD5).
    type Vectors<'a> = [(u16, &'a [(u8, u32)], &'a [u8]); 15];
    const VECTORS: Vectors<'static> = [
        (
            437,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00A2),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            737,
            &[
                (0x80, 0x0391),
                (0x9B, 0x03B4),
                (0xA1, 0x03BA),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x03AC),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            775,
            &[
                (0x80, 0x0106),
                (0x9B, 0x00F8),
                (0xA1, 0x012A),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x00B9),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            850,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00F8),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xD5, 0x0131),
                (0xE1, 0x00DF),
                (0xFB, 0x00B9),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            852,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x0164),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x0171),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            855,
            &[
                (0x80, 0x0452),
                (0x9B, 0x040F),
                (0xA1, 0x0410),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x0440),
                (0xFB, 0x0447),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            857,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00F8),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x00B9),
                (0xFF, 0x00A0),
            ],
            &[0xD5, 0xE7, 0xF2],
        ),
        (
            858,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00F8),
                (0xD5, 0x20AC),
                (0xE1, 0x00DF),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            860,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00A2),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            861,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00F8),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            862,
            &[
                (0x80, 0x05D0),
                (0x9B, 0x00A2),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            863,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00A2),
                (0xA1, 0x00B4),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            864,
            &[
                (0x80, 0x00B0),
                (0xA1, 0x00AD),
                (0xB0, 0x0660),
                (0xC4, 0xFE85),
                (0xE1, 0xFED3),
                (0xFB, 0xFEDD),
            ],
            &[0x9B, 0x9C, 0x9F, 0xA6, 0xA7, 0xFF],
        ),
        (
            865,
            &[
                (0x80, 0x00C7),
                (0x9B, 0x00F8),
                (0xA1, 0x00ED),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x00DF),
                (0xFB, 0x221A),
                (0xFF, 0x00A0),
            ],
            &[],
        ),
        (
            869,
            &[
                (0x9B, 0x03AC),
                (0xA1, 0x0390),
                (0xB0, 0x2591),
                (0xC4, 0x2500),
                (0xE1, 0x03B7),
                (0xFB, 0x03CB),
                (0xFF, 0x00A0),
            ],
            &[0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x87, 0x93, 0x94],
        ),
    ];

    #[test]
    fn oem_pages_match_the_published_mappings() {
        for (cp, pairs, undefined) in VECTORS {
            assert!(is_supported(cp), "{cp}");
            for &(byte, scalar) in pairs {
                let want = char::from_u32(scalar).unwrap();
                assert_eq!(
                    decode(cp, &[byte]).unwrap(),
                    want.to_string(),
                    "{cp} {byte:#x}"
                );
                assert_eq!(
                    encode(cp, &want.to_string(), "t").unwrap(),
                    [byte],
                    "{cp} {byte:#x}"
                );
            }
            for &byte in undefined {
                assert_eq!(decode(cp, &[byte]).unwrap(), "\u{FFFD}", "{cp} {byte:#x}");
            }
        }
    }

    #[test]
    fn oem_pages_round_trip_every_defined_byte() {
        for (cp, _, _) in VECTORS {
            for b in 0..=255u8 {
                let s = decode(cp, &[b]).unwrap();
                if s == "\u{FFFD}" {
                    assert!(encode(cp, &s, "t").is_err(), "{cp} {b:#x}");
                } else {
                    let back = encode(cp, &s, "t").unwrap();
                    assert_eq!(back.len(), 1);
                    // Where two bytes share a scalar the lowest is chosen.
                    assert_eq!(decode(cp, &back).unwrap(), s, "{cp} {b:#x}");
                    assert!(back[0] <= b);
                }
            }
        }
    }

    #[test]
    fn oem_pages_are_ascii_compatible_except_where_published() {
        for (cp, _, _) in VECTORS {
            for b in 0x20..0x7F_u8 {
                let s = decode(cp, &[b]).unwrap();
                if s != char::from(b).to_string() {
                    // Only code page 864 deviates (0x25 is the Arabic percent
                    // sign in Microsoft's table).
                    assert_eq!((cp, b), (864, 0x25), "{cp} {b:#x}");
                }
            }
        }
    }

    #[test]
    fn cp858_differs_from_850_only_at_d5() {
        for b in 0..=255u8 {
            let (a, c) = (decode(850, &[b]).unwrap(), decode(858, &[b]).unwrap());
            if b == 0xD5 {
                assert_eq!((a.as_str(), c.as_str()), ("\u{131}", "\u{20AC}"));
            } else {
                assert_eq!(a, c, "{b:#x}");
            }
        }
    }

    #[test]
    fn oem_text_and_astral_characters() {
        assert_eq!(encode(437, "Größe", "t").unwrap(), b"Gr\x94\xe1e");
        assert_eq!(decode(850, b"Gr\x94\xe1e").unwrap(), "Größe");
        assert_eq!(encode(866, "Привет", "t").unwrap().len(), 6);
        assert!(matches!(
            encode(437, "a\u{1F600}", "m"),
            Err(Error::Unencodable { code_page: 437, .. })
        ));
        assert!(matches!(
            encode(437, "日", "m"),
            Err(Error::Unencodable { .. })
        ));
    }
}
