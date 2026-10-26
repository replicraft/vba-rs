//! The line table (`pcode-format.md` section 7).

use crate::error::{PcodeError, Result};
use crate::module::Span;
use crate::reader::{slice, u16_at};

/// Marker word at the start of the line table.
pub const LINE_MARKER: u16 = 0xCAFE;
/// Size of a line record in bytes.
pub const LINE_RECORD_SIZE: usize = 12;
/// Alignment of the per-line code slots.
pub const CODE_ALIGNMENT: usize = 8;
/// Size of the unknown trailer after the line records.
pub const LINE_TRAILER_SIZE: usize = 10;

/// One 12-byte line record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRecord {
    /// The four unknown bytes at +0.
    pub unknown0: [u8; 4],
    /// Bytes of code of this line.
    pub length: u16,
    /// The two unknown bytes at +6.
    pub unknown6: [u8; 2],
    /// Offset of the code relative to [`LineTable::code_base`].
    pub offset: u32,
}

/// The parsed line table of one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTable {
    /// Position of the `0xCAFE` marker.
    pub at: usize,
    /// The unknown word at +2.
    pub unknown2: u16,
    /// Line records, line 0 first.
    pub lines: Vec<LineRecord>,
    /// The unknown 10-byte trailer after the records.
    pub trailer: Span,
    /// Start of the code area; line offsets are relative to it.
    pub code_base: usize,
}

impl LineTable {
    /// Parses the line table at `at` in `cache`.
    ///
    /// # Errors
    ///
    /// [`PcodeError::BadLineMarker`] when the marker is wrong, truncation and
    /// bounds errors otherwise.
    pub fn parse(cache: &[u8], at: usize) -> Result<Self> {
        let marker = u16_at(cache, at, "line table marker")?;
        if marker != LINE_MARKER {
            return Err(PcodeError::BadLineMarker { found: marker });
        }
        let unknown2 = u16_at(cache, at + 2, "line table word")?;
        let n = usize::from(u16_at(cache, at + 4, "line count")?);
        let recs = slice(cache, at + 6, n * LINE_RECORD_SIZE, "line records")?;
        let mut lines = Vec::with_capacity(n);
        for r in recs.as_chunks::<LINE_RECORD_SIZE>().0 {
            lines.push(LineRecord {
                unknown0: [r[0], r[1], r[2], r[3]],
                length: u16::from_le_bytes([r[4], r[5]]),
                unknown6: [r[6], r[7]],
                offset: u32::from_le_bytes([r[8], r[9], r[10], r[11]]),
            });
        }
        let trailer_start = at + 6 + n * LINE_RECORD_SIZE;
        slice(
            cache,
            trailer_start,
            LINE_TRAILER_SIZE,
            "line table trailer",
        )?;
        Ok(Self {
            at,
            unknown2,
            lines,
            trailer: trailer_start..trailer_start + LINE_TRAILER_SIZE,
            code_base: trailer_start + LINE_TRAILER_SIZE,
        })
    }

    /// The absolute byte range of the code of line `index`, bounds-checked
    /// against `cache`. `None` for a line without code (length 0).
    ///
    /// # Errors
    ///
    /// [`PcodeError::OutOfBounds`] when the line points outside the cache.
    pub fn code_range(&self, index: usize, cache_len: usize) -> Result<Option<Span>> {
        let Some(rec) = self.lines.get(index) else {
            return Ok(None);
        };
        if rec.length == 0 {
            return Ok(None);
        }
        let start = (self.code_base as u64) + u64::from(rec.offset);
        let end = start + u64::from(rec.length);
        if end > cache_len as u64 {
            return Err(PcodeError::OutOfBounds {
                what: "line code",
                offset: start,
                size: u64::from(rec.length),
                limit: cache_len as u64,
            });
        }
        Ok(Some(start as usize..end as usize))
    }

    /// The slot of line `index`: the code range rounded up to a multiple of
    /// [`CODE_ALIGNMENT`] bytes. `[obs]`: consecutive lines with code occupy
    /// consecutive slots, and the bytes between a line's end and the end of
    /// its slot hold no instructions.
    ///
    /// # Errors
    ///
    /// [`PcodeError::OutOfBounds`] when the slot leaves the cache.
    pub fn slot_range(&self, index: usize, cache_len: usize) -> Result<Option<Span>> {
        let Some(code) = self.code_range(index, cache_len)? else {
            return Ok(None);
        };
        let len = code.len().div_ceil(CODE_ALIGNMENT) * CODE_ALIGNMENT;
        let end = code.start + len;
        if end > cache_len {
            return Err(PcodeError::OutOfBounds {
                what: "line slot",
                offset: code.start as u64,
                size: len as u64,
                limit: cache_len as u64,
            });
        }
        Ok(Some(code.start..end))
    }
}
