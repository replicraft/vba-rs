//! Small sections of the module cache found by observation of the test
//! project: the type-library reference list and the member table.
//!
//! Both layouts are `[obs]` for VBA 7.0 x86; they are not described by the
//! repository documents beyond the header offsets that locate them.

use crate::error::{PcodeError, Result};
use crate::module::Span;
use crate::reader::{Cursor, slice, u16_at, u32_at};

/// Magic dword that starts the reference list.
pub const REFERENCE_LIST_MAGIC: u32 = 0x5AFE_B7A9;
/// Byte that ends the reference list.
pub const REFERENCE_LIST_END: u8 = 0xDF;

/// The list of type-library reference strings stored in a module cache.
///
/// Layout: magic dword, a word (observed value 9, meaning unknown), then
/// entries `01 | u16 byte length | UTF-16LE text`, then a `0xDF` byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceList {
    /// The word after the magic.
    pub word: u16,
    /// Entry texts decoded from UTF-16 (lossy).
    pub entries: Vec<String>,
}

impl ReferenceList {
    /// Parses the list occupying exactly `span` of `cache`.
    ///
    /// # Errors
    ///
    /// Errors when the magic or terminator is wrong or the entries do not
    /// tile the span.
    pub fn parse(cache: &[u8], span: &Span) -> Result<Self> {
        let data = slice(cache, span.start, span.len(), "reference list")?;
        let magic = u32_at(data, 0, "reference list magic")?;
        if magic != REFERENCE_LIST_MAGIC {
            return Err(PcodeError::ProjectWalk {
                offset: span.start,
                reason: "reference list magic mismatch",
            });
        }
        let word = u16_at(data, 4, "reference list word")?;
        let mut c = Cursor::new(data, 6);
        let mut entries = Vec::new();
        loop {
            let tag = c.u8("reference list tag")?;
            if tag == REFERENCE_LIST_END {
                break;
            }
            if tag != 1 {
                return Err(PcodeError::ProjectWalk {
                    offset: span.start + c.pos() - 1,
                    reason: "unexpected reference list tag",
                });
            }
            let n = usize::from(c.u16("reference entry length")?);
            let raw = c.bytes(n, "reference entry text")?;
            let units: Vec<u16> = raw
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&p| u16::from_le_bytes(p))
                .collect();
            entries.push(String::from_utf16_lossy(&units));
        }
        if c.pos() != data.len() {
            return Err(PcodeError::ProjectWalk {
                offset: span.start + c.pos(),
                reason: "reference list does not fill its span",
            });
        }
        Ok(Self { word, entries })
    }
}

/// Size of one member-table record.
pub const MEMBER_RECORD_SIZE: usize = 12;

/// One record of the member table (three dwords).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberRecord {
    /// First dword (observed `0` or `0xFFFFFFFF`).
    pub flags: u32,
    /// Second dword.
    pub id: u32,
    /// Third dword.
    pub value: u32,
}

/// The member table: a word count followed by that many 12-byte records,
/// exactly filling the section. Present in class modules only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberTable {
    /// The records.
    pub records: Vec<MemberRecord>,
}

impl MemberTable {
    /// Parses the table occupying exactly `span` of `cache`.
    ///
    /// # Errors
    ///
    /// Errors when `2 + 12 * count` differs from the span length.
    pub fn parse(cache: &[u8], span: &Span) -> Result<Self> {
        let data = slice(cache, span.start, span.len(), "member table")?;
        let n = usize::from(u16_at(data, 0, "member count")?);
        if 2 + n * MEMBER_RECORD_SIZE != data.len() {
            return Err(PcodeError::OutOfBounds {
                what: "member table size",
                offset: span.start as u64,
                size: data.len() as u64,
                limit: (2 + n * MEMBER_RECORD_SIZE) as u64,
            });
        }
        let mut records = Vec::with_capacity(n);
        for i in 0..n {
            let o = 2 + i * MEMBER_RECORD_SIZE;
            records.push(MemberRecord {
                flags: u32_at(data, o, "member flags")?,
                id: u32_at(data, o + 4, "member id")?,
                value: u32_at(data, o + 8, "member value")?,
            });
        }
        Ok(Self { records })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn reference_list_roundtrip() {
        let mut b = REFERENCE_LIST_MAGIC.to_le_bytes().to_vec();
        b.extend_from_slice(&9u16.to_le_bytes());
        for s in ["*\\Rab", "*\\Rcd"] {
            let t = utf16(s);
            b.push(1);
            b.extend_from_slice(&(t.len() as u16).to_le_bytes());
            b.extend_from_slice(&t);
        }
        b.push(REFERENCE_LIST_END);
        let l = ReferenceList::parse(&b, &(0..b.len())).unwrap();
        assert_eq!(l.word, 9);
        assert_eq!(l.entries, vec!["*\\Rab", "*\\Rcd"]);
        // Trailing garbage is rejected.
        let mut bad = b.clone();
        bad.push(0);
        assert!(ReferenceList::parse(&bad, &(0..bad.len())).is_err());
        // Truncation is rejected.
        assert!(ReferenceList::parse(&b, &(0..b.len() - 3)).is_err());
    }

    #[test]
    fn member_table() {
        let mut b = 2u16.to_le_bytes().to_vec();
        for (a, i, v) in [(0u32, 0x1776u32, 0x34u32), (u32::MAX, 0x50, 0x40002)] {
            b.extend_from_slice(&a.to_le_bytes());
            b.extend_from_slice(&i.to_le_bytes());
            b.extend_from_slice(&v.to_le_bytes());
        }
        let t = MemberTable::parse(&b, &(0..b.len())).unwrap();
        assert_eq!(t.records.len(), 2);
        assert_eq!(t.records[1].flags, u32::MAX);
        assert!(MemberTable::parse(&b, &(0..b.len() - 1)).is_err());
    }
}
