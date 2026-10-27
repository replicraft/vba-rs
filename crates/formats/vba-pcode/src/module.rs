//! Module-stream header and table locations (`pcode-format.md` section 4).

use std::ops::Range;

use crate::error::{PcodeError, Result, UnsupportedLayout};
use crate::layout::{Bitness, Layout, VersionFamily};
use crate::reader::{u16_at, u32_at};

/// A byte range of the cache whose first byte is at `start` and that ends
/// (exclusive) at `end`.
pub type Span = Range<usize>;

/// Where the tables of one module cache live. All offsets are absolute in the
/// module stream (the cache starts at stream offset 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableLocations {
    /// Declaration table bytes (after its 4-byte length).
    pub declarations: Span,
    /// Position of the length field of the declaration table.
    pub declarations_len_at: usize,
    /// Indirect table bytes (after its 4-byte length).
    pub indirect: Span,
    /// Position of the length field of the indirect table.
    pub indirect_len_at: usize,
    /// The U32 `P` that locates the indirect table.
    pub indirect_base: usize,
    /// Object table bytes (after its 4-byte length).
    pub objects: Span,
    /// Position of the length field of the object table.
    pub objects_len_at: usize,
    /// The U32 `Q` that locates the object table.
    pub objects_base: usize,
    /// Position of the line table header (`R + 0x3C`).
    pub line_table_at: usize,
}

fn to_usize(v: u32, what: &'static str, limit: usize) -> Result<usize> {
    usize::try_from(v)
        .ok()
        .filter(|&u| u <= limit)
        .ok_or(PcodeError::OutOfBounds {
            what,
            offset: u64::from(v),
            size: 0,
            limit: limit as u64,
        })
}

fn table(cache: &[u8], len_at: usize, what: &'static str) -> Result<Span> {
    let len = u32_at(cache, len_at, what)?;
    let start = len_at + 4;
    let end = u64::from(len) + start as u64;
    if end > cache.len() as u64 {
        return Err(PcodeError::OutOfBounds {
            what,
            offset: start as u64,
            size: u64::from(len),
            limit: cache.len() as u64,
        });
    }
    Ok(start..end as usize)
}

/// Skips a U32-length-prefixed block starting at `pos`, returning the
/// position after it.
fn skip_block(cache: &[u8], pos: usize, limit: usize) -> Result<usize> {
    let l = u32_at(cache, pos, "length-prefixed block")? as usize;
    pos.checked_add(4)
        .and_then(|p| p.checked_add(l))
        .filter(|&e| e <= limit)
        .ok_or(PcodeError::OutOfBounds {
            what: "length-prefixed block",
            offset: pos as u64,
            size: l as u64,
            limit: limit as u64,
        })
}

impl TableLocations {
    /// Locates the tables of a module cache (`cache` is the stream bytes
    /// before the source offset).
    ///
    /// # Errors
    ///
    /// Typed errors for truncation, out-of-range offsets and unsupported
    /// layouts. The VBA5 arrangement is implemented from the specification
    /// and unverified.
    pub fn locate(cache: &[u8], layout: &Layout) -> Result<Self> {
        let w = u16_at(cache, 2, "module stream word at 2")?;
        if w > 0xFF {
            return Err(PcodeError::UnsupportedLayout(UnsupportedLayout::BigEndian));
        }
        let limit = cache.len();
        match layout.family {
            VersionFamily::Vba5 => Self::locate_vba5(cache, limit),
            _ => {
                let x64 = layout.bitness == Bitness::X64;
                // `[obs]`: the word at offset 2 is the platform kind the cache
                // was compiled for (1 or 3) in all 492 modules seen.
                let expected = if x64 { 3 } else { 1 };
                if matches!(w, 1 | 3) && w != expected {
                    return Err(PcodeError::UnsupportedLayout(
                        UnsupportedLayout::PlatformMismatch { found: w, expected },
                    ));
                }
                // The declaration table length sits at 0x3F in every layout
                // observed, 64-bit ones included.
                let declarations_len_at = 0x3F;
                let declarations = table(cache, declarations_len_at, "declaration table")?;
                let p = to_usize(
                    u32_at(cache, 0x11, "indirect base")?,
                    "indirect base",
                    limit,
                )?;
                let indirect_len_at = p + if x64 { 12 } else { 10 };
                let indirect = table(cache, indirect_len_at, "indirect table")?;
                let q = to_usize(u32_at(cache, 0x05, "object base")?, "object base", limit)?;
                let objects_len_at = q + 0x8A;
                let objects = table(cache, objects_len_at, "object table")?;
                let r = to_usize(
                    u32_at(cache, 0x19, "line table pointer")?,
                    "line table pointer",
                    limit,
                )?;
                Ok(Self {
                    declarations,
                    declarations_len_at,
                    indirect,
                    indirect_len_at,
                    indirect_base: p,
                    objects,
                    objects_len_at,
                    objects_base: q,
                    line_table_at: r + 0x3C,
                })
            }
        }
    }

    fn locate_vba5(cache: &[u8], limit: usize) -> Result<Self> {
        let declarations_len_at = 11;
        let declarations = table(cache, declarations_len_at, "declaration table")?;
        let mut pos = declarations.end + 64;
        let n = usize::from(u16_at(cache, pos, "vba5 entry count")?);
        pos += 2 + n * 16;
        if pos > limit {
            return Err(PcodeError::OutOfBounds {
                what: "vba5 entries",
                offset: pos as u64,
                size: 0,
                limit: limit as u64,
            });
        }
        pos = skip_block(cache, pos, limit)?;
        pos += 6;
        let x = skip_block(cache, pos, limit)?;
        let s = to_usize(
            u32_at(cache, x + 8, "indirect base")?,
            "indirect base",
            limit,
        )?;
        let indirect_len_at = s + 10;
        let indirect = table(cache, indirect_len_at, "indirect table")?;
        let q = to_usize(u32_at(cache, x, "object base")?, "object base", limit)?;
        let objects_len_at = q + 0x8A;
        let objects = table(cache, objects_len_at, "object table")?;
        let r = to_usize(
            u32_at(cache, x + 77, "line table pointer")?,
            "line table pointer",
            limit,
        )?;
        Ok(Self {
            declarations,
            declarations_len_at,
            indirect,
            indirect_len_at,
            indirect_base: s,
            objects,
            objects_len_at,
            objects_base: q,
            line_table_at: r + 0x3C,
        })
    }
}
