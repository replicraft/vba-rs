//! Region map of one module cache (Phase 1 structure decomposition).

use crate::error::{PcodeError, Result};
use crate::layout::{Layout, VersionFamily};
use crate::lines::{LINE_RECORD_SIZE, LineTable};
use crate::module::{Span, TableLocations};
use crate::reader::u32_at;
use crate::sections::{MemberTable, ReferenceList};

/// What a region of the cache is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionKind {
    /// The constant five-byte prefix (`01 16 01 00 06` in the target).
    Prefix,
    /// The seven header dwords at offsets 5 to 0x1C plus the end offset at
    /// 0x1D (all offsets of the cache sections).
    OffsetFields,
    /// Declaration table including its length dword.
    DeclarationTable,
    /// Object table including its length dword.
    ObjectTable,
    /// Indirect table including its length dword.
    IndirectTable,
    /// The type-library reference list.
    ReferenceList,
    /// The member table (class modules only).
    MemberTable,
    /// The `0xCAFE` line table header (marker, word, count).
    LineTableHeader,
    /// The 12-byte line records.
    LineRecords,
    /// The 10-byte line table trailer.
    LineTableTrailer,
    /// The code area: the concatenated per-line instruction streams.
    Code,
    /// The final trailer after the last section.
    Trailer,
    /// A block whose extent is known but whose content is not interpreted.
    Opaque(OpaqueKind),
}

/// Names of the delimited but uninterpreted blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpaqueKind {
    /// Header bytes between the offset fields and the declaration table.
    HeaderTail,
    /// The block from `P` to the indirect table length (header of the
    /// indirect table).
    IndirectHeader,
    /// The block from `Q` to the object table length (header of the object
    /// table).
    ObjectHeader,
    /// The 60-byte block between the line table pointer and the line table.
    PreLineTable,
    /// The block between the last code byte and the final trailer.
    PostCode,
    /// The block from the end of the declaration table to the start of the
    /// object table header (contains the module-info block that the header
    /// offset at 0x0D points into).
    PostDeclaration,
    /// The block from the end of the object table to the indirect table
    /// header.
    PostObjectTable,
    /// The block from the end of the indirect table to the reference list.
    PostIndirectTable,
    /// Bytes between identified structures for which no structure is known.
    Unattributed,
}

/// One region of the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Byte range in the module stream.
    pub span: Span,
    /// What it is.
    pub kind: RegionKind,
}

impl Region {
    /// Whether the content of the region is interpreted by this crate.
    pub fn is_interpreted(&self) -> bool {
        !matches!(self.kind, RegionKind::Opaque(_))
    }
}

/// A fully located module cache.
#[derive(Debug, Clone)]
pub struct ModuleCache<'a> {
    /// The cache bytes (stream bytes before the source offset).
    pub bytes: &'a [u8],
    /// The layout used.
    pub layout: Layout,
    /// Table locations.
    pub tables: TableLocations,
    /// The line table.
    pub lines: LineTable,
    /// The reference list, when the section parsed.
    pub references: Option<ReferenceList>,
    /// The member table, when present and parsed.
    pub members: Option<MemberTable>,
    /// Gap-free sequence of regions covering `0..bytes.len()`.
    pub regions: Vec<Region>,
}

/// Size of the final trailer observed in the target layout.
pub const TRAILER_SIZE: usize = 6;

impl<'a> ModuleCache<'a> {
    /// Locates and decomposes the cache.
    ///
    /// # Errors
    ///
    /// Typed errors for truncation, bad offsets, a missing `0xCAFE` marker,
    /// unsupported layouts and overlapping sections.
    pub fn parse(bytes: &'a [u8], layout: &Layout) -> Result<Self> {
        let len = bytes.len();
        let tables = TableLocations::locate(bytes, layout)?;
        let lines = LineTable::parse(bytes, tables.line_table_at)?;
        let vba5 = layout.family == VersionFamily::Vba5;

        let mut known: Vec<Region> = Vec::new();
        let mut push = |start: usize, end: usize, kind: RegionKind| {
            if start < end {
                known.push(Region {
                    span: start..end,
                    kind,
                });
            }
        };
        let mut references = None;
        let mut members = None;

        if !vba5 {
            push(0, 5, RegionKind::Prefix);
            push(5, 0x21, RegionKind::OffsetFields);
            let decl_at = tables.declarations_len_at;
            push(0x21, decl_at, RegionKind::Opaque(OpaqueKind::HeaderTail));
            let h9 = ptr(bytes, 9, "reference list pointer")?;
            let h21 = u32_at(bytes, 21, "member table pointer")?;
            let r = ptr(bytes, 0x19, "line table pointer")?;
            let list_end = if h21 == u32::MAX {
                r
            } else {
                ptr(bytes, 21, "member table pointer")?
            };
            if h9 < list_end && list_end <= len {
                let span = h9..list_end;
                references = ReferenceList::parse(bytes, &span).ok();
                if references.is_some() {
                    push(span.start, span.end, RegionKind::ReferenceList);
                }
            }
            if h21 != u32::MAX {
                let start = ptr(bytes, 21, "member table pointer")?;
                if start < r && r <= len {
                    let span = start..r;
                    members = MemberTable::parse(bytes, &span).ok();
                    if members.is_some() {
                        push(span.start, span.end, RegionKind::MemberTable);
                    }
                }
            }
            push(
                r,
                tables.line_table_at,
                RegionKind::Opaque(OpaqueKind::PreLineTable),
            );
        }
        push(
            tables.declarations_len_at,
            tables.declarations.end,
            RegionKind::DeclarationTable,
        );
        push(
            tables.objects_base,
            tables.objects_len_at,
            RegionKind::Opaque(OpaqueKind::ObjectHeader),
        );
        push(
            tables.objects_len_at,
            tables.objects.end,
            RegionKind::ObjectTable,
        );
        push(
            tables.indirect_base,
            tables.indirect_len_at,
            RegionKind::Opaque(OpaqueKind::IndirectHeader),
        );
        push(
            tables.indirect_len_at,
            tables.indirect.end,
            RegionKind::IndirectTable,
        );
        let t = lines.at;
        let rec_end = t + 6 + lines.lines.len() * LINE_RECORD_SIZE;
        push(t, t + 6, RegionKind::LineTableHeader);
        push(t + 6, rec_end, RegionKind::LineRecords);
        push(
            lines.trailer.start,
            lines.trailer.end,
            RegionKind::LineTableTrailer,
        );
        let mut code_end = lines.code_base;
        for i in 0..lines.lines.len() {
            if let Some(r) = lines.slot_range(i, len)? {
                code_end = code_end.max(r.end);
            }
        }
        push(lines.code_base, code_end, RegionKind::Code);
        if !vba5 {
            let end_ptr = ptr(bytes, 29, "end pointer")?;
            if end_ptr >= code_end && end_ptr <= len {
                push(code_end, end_ptr, RegionKind::Opaque(OpaqueKind::PostCode));
                push(end_ptr, len, RegionKind::Trailer);
            }
        }

        let regions = tile(known, len)?;
        Ok(Self {
            bytes,
            layout: *layout,
            tables,
            lines,
            references,
            members,
            regions,
        })
    }

    /// Statistics of how much of the cache is interpreted.
    pub fn coverage(&self) -> Coverage {
        let mut c = Coverage {
            total: self.bytes.len(),
            ..Coverage::default()
        };
        for r in &self.regions {
            let n = r.span.len();
            match r.kind {
                RegionKind::Opaque(OpaqueKind::Unattributed) => c.unattributed += n,
                RegionKind::Opaque(_) => c.opaque += n,
                _ => c.interpreted += n,
            }
        }
        c
    }

    /// Bytes of the code area not claimed by any line (holes in the area), and
    /// bytes claimed by more than one line (overlaps).
    pub fn code_area_check(&self) -> CodeAreaCheck {
        let len = self.bytes.len();
        let mut spans: Vec<Span> = (0..self.lines.lines.len())
            .filter_map(|i| self.lines.slot_range(i, len).ok().flatten())
            .collect();
        spans.sort_by_key(|s| (s.start, s.end));
        let area_start = self.lines.code_base;
        let mut pos = area_start;
        let mut holes = 0;
        let mut overlaps = 0;
        for s in &spans {
            if s.start > pos {
                holes += s.start - pos;
            } else if s.start < pos {
                overlaps += pos.min(s.end) - s.start;
            }
            pos = pos.max(s.end);
        }
        CodeAreaCheck {
            area: pos - area_start,
            holes,
            overlaps,
        }
    }
}

/// Result of [`ModuleCache::code_area_check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodeAreaCheck {
    /// Size of the code area (up to the end of the last line).
    pub area: usize,
    /// Bytes inside the area claimed by no line.
    pub holes: usize,
    /// Bytes claimed by more than one line.
    pub overlaps: usize,
}

/// Byte accounting of a region map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Coverage {
    /// Cache size.
    pub total: usize,
    /// Bytes in interpreted regions.
    pub interpreted: usize,
    /// Bytes in delimited but uninterpreted regions.
    pub opaque: usize,
    /// Bytes in no identified structure.
    pub unattributed: usize,
}

impl Coverage {
    /// Percentage of bytes in interpreted regions.
    pub fn interpreted_percent(&self) -> f64 {
        percent(self.interpreted, self.total)
    }

    /// Percentage of bytes in a delimited region (interpreted or opaque).
    pub fn delimited_percent(&self) -> f64 {
        percent(self.interpreted + self.opaque, self.total)
    }

    /// Bytes not interpreted (opaque plus unattributed).
    pub fn unexplained(&self) -> usize {
        self.opaque + self.unattributed
    }
}

fn percent(a: usize, b: usize) -> f64 {
    if b == 0 {
        100.0
    } else {
        a as f64 * 100.0 / b as f64
    }
}

fn ptr(bytes: &[u8], at: usize, what: &'static str) -> Result<usize> {
    let v = u32_at(bytes, at, what)?;
    usize::try_from(v)
        .ok()
        .filter(|&u| u <= bytes.len())
        .ok_or(PcodeError::OutOfBounds {
            what,
            offset: u64::from(v),
            size: 0,
            limit: bytes.len() as u64,
        })
}

/// Names a gap by the regions around it.
fn gap_kind(prev: Option<RegionKind>, next: RegionKind) -> OpaqueKind {
    use RegionKind as K;
    match (prev, next) {
        (Some(K::DeclarationTable), K::Opaque(OpaqueKind::ObjectHeader)) => {
            OpaqueKind::PostDeclaration
        }
        (Some(K::ObjectTable), K::Opaque(OpaqueKind::IndirectHeader)) => {
            OpaqueKind::PostObjectTable
        }
        (Some(K::IndirectTable), K::ReferenceList) => OpaqueKind::PostIndirectTable,
        _ => OpaqueKind::Unattributed,
    }
}

/// Sorts `known`, rejects overlaps, and fills gaps with
/// [`OpaqueKind::Unattributed`] so the result tiles `0..len`.
fn tile(mut known: Vec<Region>, len: usize) -> Result<Vec<Region>> {
    known.sort_by_key(|r| (r.span.start, r.span.end));
    let mut out = Vec::with_capacity(known.len() * 2 + 1);
    let mut pos = 0usize;
    for r in known {
        if r.span.end > len {
            return Err(PcodeError::OutOfBounds {
                what: "region",
                offset: r.span.start as u64,
                size: r.span.len() as u64,
                limit: len as u64,
            });
        }
        if r.span.start < pos {
            return Err(PcodeError::OutOfBounds {
                what: "overlapping regions",
                offset: r.span.start as u64,
                size: r.span.len() as u64,
                limit: pos as u64,
            });
        }
        if r.span.start > pos {
            let prev = out.last().map(|x: &Region| x.kind);
            out.push(Region {
                span: pos..r.span.start,
                kind: RegionKind::Opaque(gap_kind(prev, r.kind)),
            });
        }
        pos = r.span.end;
        out.push(r);
    }
    if pos < len {
        out.push(Region {
            span: pos..len,
            kind: RegionKind::Opaque(OpaqueKind::Unattributed),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_fills_gaps_and_rejects_overlap() {
        let k = vec![
            Region {
                span: 2..4,
                kind: RegionKind::Prefix,
            },
            Region {
                span: 6..8,
                kind: RegionKind::Trailer,
            },
        ];
        let t = tile(k, 10).unwrap();
        let spans: Vec<_> = t.iter().map(|r| r.span.clone()).collect();
        assert_eq!(spans, vec![0..2, 2..4, 4..6, 6..8, 8..10]);
        let bad = vec![
            Region {
                span: 0..5,
                kind: RegionKind::Prefix,
            },
            Region {
                span: 4..6,
                kind: RegionKind::Trailer,
            },
        ];
        assert!(tile(bad, 10).is_err());
        let oob = vec![Region {
            span: 0..11,
            kind: RegionKind::Prefix,
        }];
        assert!(tile(oob, 10).is_err());
    }
}
