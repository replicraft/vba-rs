//! Structural classification of the whole indirect table.
//!
//! [`crate::RecordMap`] only knows records that code reaches directly. The
//! indirect table is a heap in which records are allocated one after the
//! other, so most of the remaining bytes are further records of the same
//! families (symbol records chained to the reachable ones, unnamed slot
//! records, type descriptors) and arrays of pointers. This module recognises
//! them by their first word and by validity of their pointer fields, and
//! reports everything else as unknown.
//!
//! All shapes are `[obs]` (VBA 7.0 x86); sizes and first-word sets come from
//! the observed material and are only applied to that layout.

use std::ops::Range;

use crate::cache::ModuleCache;
use crate::layout::{Bitness, VersionFamily};
use crate::recordmap::{RecordKind, RecordMap};

/// Kinds of regions of the indirect table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IndirectKind {
    /// Function record (64 bytes), reachable from code.
    Function,
    /// Argument record (32 bytes), reachable from code.
    Argument,
    /// Variable record (24 bytes), reachable from code.
    Variable,
    /// User-defined-type record (24 bytes), reachable from code.
    UserType,
    /// Type descriptor (8 bytes), reachable from code.
    TypeDescriptor,
    /// Function record found by shape, not reached from code.
    FunctionByShape,
    /// Argument record found by shape.
    ArgumentByShape,
    /// Named symbol record of variable layout found by shape.
    VariableByShape,
    /// Symbol record of variable layout without a name (name word
    /// `0xFFFE` in almost all cases): a slot of the module or a procedure.
    SlotRecord,
    /// Type descriptor found by shape.
    TypeDescriptorByShape,
    /// A run of dwords that are `0xFFFFFFFF` or offsets of recognised
    /// records (symbol hash buckets and similar).
    PointerArray,
    /// A run of `0xFFFFFFFF` dwords.
    Fill,
}

impl IndirectKind {
    /// Whether the kind was recognised only by shape (not reached from
    /// code).
    pub fn by_shape(self) -> bool {
        matches!(
            self,
            IndirectKind::FunctionByShape
                | IndirectKind::ArgumentByShape
                | IndirectKind::VariableByShape
                | IndirectKind::SlotRecord
                | IndirectKind::TypeDescriptorByShape
        )
    }
}

/// The classified regions of an indirect table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndirectMap {
    /// Table length.
    pub table_len: usize,
    /// Non-overlapping regions in ascending order.
    pub regions: Vec<(Range<usize>, IndirectKind)>,
}

/// Coverage figures of an [`IndirectMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IndirectCoverage {
    /// Table length.
    pub table: usize,
    /// Bytes in records reached from code.
    pub reachable: usize,
    /// Bytes in records recognised by shape.
    pub by_shape: usize,
    /// Bytes in pointer arrays and fill.
    pub arrays: usize,
    /// Bytes in no recognised structure.
    pub unknown: usize,
}

impl IndirectCoverage {
    /// Share of the table in understood regions (percent).
    pub fn understood_percent(&self) -> f64 {
        if self.table == 0 {
            return 0.0;
        }
        (self.table - self.unknown) as f64 * 100.0 / self.table as f64
    }
}

const FUNCTION_WORDS: &[u16] = &[0x110c, 0x112c, 0x120b, 0x122b, 0x212c, 0x410c];
const ARGUMENT_WORDS: &[u16] = &[0x8329, 0x8349, 0x8369];
const VARIABLE_WORDS: &[u16] = &[
    0x8022, 0x8028, 0x8062, 0x8068, 0x806a, 0x80a2, 0x80e2, 0x80ea, 0x8420, 0x8440, 0x8450, 0x8460,
    0x8480, 0x84e0, 0x9062, 0x90c2, 0x9440, 0x9460, 0x9842, 0x9862, 0x98c2, 0x98e2, 0x9c40, 0x9c60,
    0xa420,
];
const SLOT_WORDS: &[u16] = &[0x0440, 0x0460];
const DESCRIPTOR_D_WORDS: &[u16] = &[0x001d, 0x011d, 0x401d, 0x411d, 0x501d];
const DESCRIPTOR_B_WORDS: &[u16] = &[0x001b, 0x081b, 0x091b, 0x101b];

fn w(t: &[u8], o: usize) -> Option<u16> {
    t.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn d(t: &[u8], o: usize) -> Option<u32> {
    t.get(o..o + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// A pointer field: `0xFFFFFFFF` (none), `0` (none) or an 8-aligned offset
/// inside the table.
fn ptr_ok(t: &[u8], o: usize) -> bool {
    match d(t, o) {
        Some(v) => v == u32::MAX || v == 0 || (v % 8 == 0 && (v as usize) < t.len()),
        None => false,
    }
}

/// Recognises a record starting at `off` by its first word and pointer
/// fields.
fn classify(t: &[u8], off: usize) -> Option<(IndirectKind, usize)> {
    let word = w(t, off)?;
    let (kind, size) = if FUNCTION_WORDS.contains(&word) {
        if !(ptr_ok(t, off + 4) && ptr_ok(t, off + 40)) {
            return None;
        }
        (IndirectKind::FunctionByShape, 64)
    } else if ARGUMENT_WORDS.contains(&word) {
        if !(ptr_ok(t, off + 4) && ptr_ok(t, off + 20)) {
            return None;
        }
        (IndirectKind::ArgumentByShape, 32)
    } else if VARIABLE_WORDS.contains(&word) || SLOT_WORDS.contains(&word) {
        let type_ok = w(t, off + 14) == Some(0xFFFF) || ptr_ok(t, off + 12);
        if !(ptr_ok(t, off + 4) && type_ok) {
            return None;
        }
        if SLOT_WORDS.contains(&word) {
            (IndirectKind::SlotRecord, 24)
        } else {
            (IndirectKind::VariableByShape, 24)
        }
    } else if DESCRIPTOR_D_WORDS.contains(&word) {
        if d(t, off + 4) != Some(0x25) {
            return None;
        }
        (IndirectKind::TypeDescriptorByShape, 8)
    } else if DESCRIPTOR_B_WORDS.contains(&word) {
        if w(t, off + 6) != Some(8) {
            return None;
        }
        (IndirectKind::TypeDescriptorByShape, 8)
    } else {
        return None;
    };
    (off + size <= t.len()).then_some((kind, size))
}

fn reach_kind(k: RecordKind) -> IndirectKind {
    match k {
        RecordKind::Function => IndirectKind::Function,
        RecordKind::Argument => IndirectKind::Argument,
        RecordKind::Variable => IndirectKind::Variable,
        RecordKind::UserType => IndirectKind::UserType,
        RecordKind::TypeDescriptor => IndirectKind::TypeDescriptor,
    }
}

impl IndirectMap {
    /// Classifies the indirect table of `cache`. Returns an empty map for
    /// layouts without observed record sizes.
    pub fn build(cache: &ModuleCache<'_>) -> Self {
        let l = &cache.layout;
        let table = &cache.bytes[cache.tables.indirect.clone()];
        let mut map = IndirectMap {
            table_len: table.len(),
            regions: Vec::new(),
        };
        if l.family != VersionFamily::Vba7 || l.bitness != Bitness::X86 {
            return map;
        }
        let seeds = RecordMap::collect(cache);
        let seed_list: Vec<(usize, RecordKind)> = seeds
            .records
            .iter()
            .map(|(&o, &k)| (o as usize, k))
            .collect();
        let mut si = 0usize;
        let mut pos = 0usize;
        // Pass 1: records, in address order; seeds win over shapes.
        while pos + 8 <= table.len() {
            while si < seed_list.len() && seed_list[si].0 < pos {
                si += 1;
            }
            if let Some(&(o, k)) = seed_list.get(si)
                && o == pos
                && let Some(size) = k.size(cache)
            {
                si += 1;
                map.regions.push((pos..pos + size, reach_kind(k)));
                pos += size;
                continue;
            }
            // Do not run a shape across the next seed.
            let limit = seed_list.get(si).map_or(table.len(), |s| s.0);
            match classify(table, pos) {
                Some((k, size)) if pos + size <= limit => {
                    map.regions.push((pos..pos + size, k));
                    pos += size;
                }
                _ => pos += 8,
            }
        }
        map.fill_arrays(table);
        map
    }

    /// Pass 2: classify runs between regions as pointer arrays or fill.
    fn fill_arrays(&mut self, table: &[u8]) {
        let starts: std::collections::BTreeSet<u32> = self
            .regions
            .iter()
            .map(|(r, _)| u32::try_from(r.start).unwrap_or(u32::MAX))
            .collect();
        let mut out: Vec<(Range<usize>, IndirectKind)> = Vec::new();
        let mut cursor = 0usize;
        let gaps: Vec<Range<usize>> = {
            let mut g = Vec::new();
            for (r, _) in &self.regions {
                if r.start > cursor {
                    g.push(cursor..r.start);
                }
                cursor = r.end;
            }
            if cursor < table.len() {
                g.push(cursor..table.len());
            }
            g
        };
        for g in gaps {
            // Dword-granular scan: split the gap into maximal runs of
            // "array-like" dwords.
            let mut i = g.start;
            while i + 4 <= g.end {
                let array_like = |o: usize| match d(table, o) {
                    Some(u32::MAX) => true,
                    Some(v) => v % 8 == 0 && v != 0 && starts.contains(&v),
                    None => false,
                };
                if !array_like(i) {
                    i += 4;
                    continue;
                }
                let s = i;
                let mut pointers = 0usize;
                while i + 4 <= g.end && array_like(i) {
                    if d(table, i) != Some(u32::MAX) {
                        pointers += 1;
                    }
                    i += 4;
                }
                let len = i - s;
                if len >= 32 {
                    let kind = if pointers == 0 {
                        IndirectKind::Fill
                    } else {
                        IndirectKind::PointerArray
                    };
                    out.push((s..i, kind));
                }
            }
        }
        self.regions.extend(out);
        self.regions.sort_by_key(|(r, _)| r.start);
    }

    /// Coverage figures.
    pub fn coverage(&self) -> IndirectCoverage {
        let mut c = IndirectCoverage {
            table: self.table_len,
            ..Default::default()
        };
        let mut known = 0;
        for (r, k) in &self.regions {
            let n = r.len();
            known += n;
            match k {
                IndirectKind::PointerArray | IndirectKind::Fill => c.arrays += n,
                k if k.by_shape() => c.by_shape += n,
                _ => c.reachable += n,
            }
        }
        c.unknown = self.table_len.saturating_sub(known);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_slot_and_descriptor_by_shape() {
        let mut t = vec![0u8; 64];
        // slot record at 0: word 0x0440, name 0xFFFE, next none, type word 3.
        t[0..2].copy_from_slice(&0x0440u16.to_le_bytes());
        t[2..4].copy_from_slice(&0xFFFEu16.to_le_bytes());
        t[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        t[12..14].copy_from_slice(&3u16.to_le_bytes());
        t[14..16].copy_from_slice(&0xFFFFu16.to_le_bytes());
        assert_eq!(classify(&t, 0), Some((IndirectKind::SlotRecord, 24)));
        // descriptor at 32.
        t[32..34].copy_from_slice(&0x001du16.to_le_bytes());
        t[36..40].copy_from_slice(&0x25u32.to_le_bytes());
        assert_eq!(
            classify(&t, 32),
            Some((IndirectKind::TypeDescriptorByShape, 8))
        );
        // wrong pointer field rejects.
        t[4..8].copy_from_slice(&0x1234_5679u32.to_le_bytes());
        assert_eq!(classify(&t, 0), None);
        // truncated table rejects.
        assert_eq!(classify(&t[..20], 0), None);
    }

    #[test]
    fn hostile_short_tables_do_not_panic() {
        for n in 0..40 {
            let t = vec![0xFFu8; n];
            for o in 0..n + 2 {
                let _ = classify(&t, o);
            }
        }
    }
}
