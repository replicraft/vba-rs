//! Exact code page conversion for the code pages vba-rs can reproduce.
//!
//! Only code pages whose mapping is fully specified are provided: US-ASCII
//! (20127), ISO-8859-1 (28591), Windows-1252 and UTF-8 (65001). Windows
//! "best-fit" substitution tables are NOT reproduced; an unmappable character
//! is therefore either rejected or replaced by an explicit default byte,
//! never silently approximated.

use thiserror::Error;

/// A supported code page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Codepage {
    /// US-ASCII (code page 20127).
    Ascii,
    /// ISO-8859-1 (code page 28591).
    Latin1,
    /// Windows-1252.
    Windows1252,
    /// UTF-8 (code page 65001).
    Utf8,
}

/// What to do with a character the code page cannot represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmappable {
    /// Fail with [`CodecError::Unmappable`].
    Reject,
    /// Substitute this byte (the `lpDefaultChar` of `WideCharToMultiByte`).
    Replace(u8),
}

/// Result of an encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The encoded bytes.
    pub bytes: Vec<u8>,
    /// Whether any default replacement was used.
    pub used_default: bool,
}

/// Conversion failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CodecError {
    /// Input bytes are not valid in the code page (offset of the first one).
    #[error("invalid byte sequence at offset {offset}")]
    InvalidSequence {
        /// Byte offset.
        offset: usize,
    },
    /// A UTF-16 character has no representation in the code page.
    #[error("character U+{ch:04X} at unit {offset} is not representable")]
    Unmappable {
        /// UTF-16 unit offset.
        offset: usize,
        /// Code point (a lone surrogate reports its unit value).
        ch: u32,
    },
}

/// Windows-1252 mapping of 0x80..=0x9F (undefined bytes map to the same C1
/// control code point, as `MultiByteToWideChar` does).
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

impl Codepage {
    /// Looks a code page up by its Windows identifier.
    pub fn from_id(id: u32) -> Option<Codepage> {
        match id {
            20127 => Some(Codepage::Ascii),
            28591 => Some(Codepage::Latin1),
            1252 => Some(Codepage::Windows1252),
            65001 => Some(Codepage::Utf8),
            _ => None,
        }
    }

    /// The Windows identifier.
    pub const fn id(self) -> u32 {
        match self {
            Codepage::Ascii => 20127,
            Codepage::Latin1 => 28591,
            Codepage::Windows1252 => 1252,
            Codepage::Utf8 => 65001,
        }
    }

    /// Decodes bytes to UTF-16.
    ///
    /// ASCII rejects bytes above 0x7F. UTF-8 fails on invalid input when
    /// `strict`, otherwise replaces each maximal invalid subpart by U+FFFD.
    pub fn decode(self, bytes: &[u8], strict: bool) -> Result<Vec<u16>, CodecError> {
        match self {
            Codepage::Ascii => {
                if let Some(offset) = bytes.iter().position(|b| *b >= 0x80) {
                    return Err(CodecError::InvalidSequence { offset });
                }
                Ok(bytes.iter().map(|b| u16::from(*b)).collect())
            }
            Codepage::Latin1 => Ok(bytes.iter().map(|b| u16::from(*b)).collect()),
            Codepage::Windows1252 => Ok(bytes
                .iter()
                .map(|b| match *b {
                    0x80..=0x9F => CP1252_HIGH[usize::from(*b - 0x80)],
                    other => u16::from(other),
                })
                .collect()),
            Codepage::Utf8 => match std::str::from_utf8(bytes) {
                Ok(s) => Ok(s.encode_utf16().collect()),
                Err(e) if strict => Err(CodecError::InvalidSequence {
                    offset: e.valid_up_to(),
                }),
                Err(_) => Ok(String::from_utf8_lossy(bytes).encode_utf16().collect()),
            },
        }
    }

    /// Encodes UTF-16 to bytes.
    ///
    /// For UTF-8 a lone surrogate fails when `strict` and is otherwise
    /// replaced by U+FFFD; `unmappable` is irrelevant. For single-byte code
    /// pages `strict` is ignored and `unmappable` decides. A surrogate pair
    /// counts as one unmappable character.
    pub fn encode(
        self,
        units: &[u16],
        unmappable: Unmappable,
        strict: bool,
    ) -> Result<Encoded, CodecError> {
        let mut out = Vec::with_capacity(units.len());
        let mut used_default = false;
        let mut i = 0;
        while i < units.len() {
            let (ch, width) = match char::decode_utf16(units[i..].iter().copied()).next() {
                Some(Ok(c)) => (Ok(c), c.len_utf16()),
                Some(Err(e)) => (Err(e.unpaired_surrogate()), 1),
                None => break,
            };
            match (self, ch) {
                (Codepage::Utf8, Ok(c)) => {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                (Codepage::Utf8, Err(s)) => {
                    if strict {
                        return Err(CodecError::Unmappable {
                            offset: i,
                            ch: u32::from(s),
                        });
                    }
                    out.extend_from_slice("\u{FFFD}".as_bytes());
                }
                (_, ch) => {
                    let mapped = match ch {
                        Ok(c) => self.single_byte(c),
                        Err(_) => None,
                    };
                    match mapped {
                        Some(b) => out.push(b),
                        None => match unmappable {
                            Unmappable::Reject => {
                                let code = match ch {
                                    Ok(c) => u32::from(c),
                                    Err(s) => u32::from(s),
                                };
                                return Err(CodecError::Unmappable {
                                    offset: i,
                                    ch: code,
                                });
                            }
                            Unmappable::Replace(b) => {
                                out.push(b);
                                used_default = true;
                            }
                        },
                    }
                }
            }
            i += width;
        }
        Ok(Encoded {
            bytes: out,
            used_default,
        })
    }

    fn single_byte(self, c: char) -> Option<u8> {
        let v = u32::from(c);
        match self {
            Codepage::Ascii => (v < 0x80).then_some(v as u8),
            Codepage::Latin1 => (v < 0x100).then_some(v as u8),
            Codepage::Windows1252 => {
                if v < 0x80 || (0xA0..0x100).contains(&v) {
                    Some(v as u8)
                } else {
                    CP1252_HIGH
                        .iter()
                        .position(|u| u32::from(*u) == v)
                        .map(|p| 0x80 + p as u8)
                }
            }
            Codepage::Utf8 => None,
        }
    }
}
