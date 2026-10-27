//! The records of the indirect table that are reachable from the code.

use std::collections::{BTreeMap, HashSet};

use crate::cache::ModuleCache;
use crate::decode::{Operands, decode_line};
use crate::layout::Layout;
use crate::layout::{Bitness, VersionFamily};
use crate::opcode::Opcode;
use crate::records::{ArgumentRecord, FunctionRecord, VariableRecord};

/// Kinds of indirect-table records that code can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecordKind {
    /// Function (procedure, property or `Declare`) record.
    Function,
    /// Argument record.
    Argument,
    /// Variable record.
    Variable,
    /// User-defined-type record.
    UserType,
    /// Type descriptor.
    TypeDescriptor,
}

impl RecordKind {
    /// Size in bytes of a record of this kind.
    ///
    /// `[obs]` (VBA 6 and 7, x86 and x64): the modal distance between
    /// consecutive reachable records of the kind in observed material; the
    /// specification only gives minimum sizes. Only the function record
    /// differs between the bitnesses (64 bytes on x86, 88 on x64). Returns
    /// `None` for VBA 5, which has no observed sizes.
    pub fn size(self, cache: &ModuleCache<'_>) -> Option<usize> {
        let l = &cache.layout;
        if l.family == VersionFamily::Vba5 {
            return None;
        }
        Some(match self {
            RecordKind::Function if l.bitness == Bitness::X64 => 88,
            RecordKind::Function => 64,
            RecordKind::Argument => 32,
            RecordKind::Variable | RecordKind::UserType => 24,
            RecordKind::TypeDescriptor => 8,
        })
    }
}

/// Longest argument chain followed.
const MAX_ARGUMENT_CHAIN: usize = 4096;

/// Walks an argument chain like `argument_chain`, but stops at a record
/// already attributed to an argument (the rest of the chain was recorded
/// then) and remembers the records of chains that failed, so a cycle or a
/// bad link is paid for once. `None` when the chain is unreadable or longer
/// than the limit.
fn walk_arguments(
    indirect: &[u8],
    first: Option<u32>,
    layout: &Layout,
    map: &RecordMap,
    bad: &mut HashSet<u32>,
) -> Option<Vec<ArgumentRecord>> {
    let mut out: Vec<ArgumentRecord> = Vec::new();
    let mut next = first;
    while let Some(off) = next {
        if map.records.get(&off) == Some(&RecordKind::Argument) {
            break;
        }
        if bad.contains(&off) || out.len() >= MAX_ARGUMENT_CHAIN {
            bad.extend(out.iter().map(|a| a.offset));
            return None;
        }
        match ArgumentRecord::parse(indirect, off, layout) {
            Ok(r) => {
                next = r.next;
                out.push(r);
            }
            Err(_) => {
                bad.extend(out.iter().map(|a| a.offset));
                return None;
            }
        }
    }
    Some(out)
}

/// Records of the indirect table reached from instruction operands,
/// argument chains and type fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordMap {
    /// Offset (in the indirect table) to kind.
    pub records: BTreeMap<u32, RecordKind>,
    /// Offsets that were reached under two different kinds.
    pub conflicts: Vec<u32>,
}

impl RecordMap {
    /// Walks all lines of `cache` and collects the reachable records.
    /// Unparseable references are skipped.
    pub fn collect(cache: &ModuleCache<'_>) -> Self {
        let ind = &cache.bytes[cache.tables.indirect.clone()];
        let layout = &cache.layout;
        let mut map = Self::default();
        let add = |m: &mut Self, off: u32, kind: RecordKind| match m.records.insert(off, kind) {
            Some(old) if old != kind => m.conflicts.push(off),
            _ => {}
        };
        let desc = |raw: u32| raw != u32::MAX && raw >> 16 != 0xFFFF;
        // Many instructions can name one record and many records can share
        // an argument chain, or end in a cycle; remember what has been
        // walked so the work is bounded by the table, not by instructions
        // times chain length.
        let mut seen_functions: HashSet<u32> = HashSet::new();
        let mut bad_arguments: HashSet<u32> = HashSet::new();
        for i in 0..cache.lines.lines.len() {
            let Ok(Some(range)) = cache.lines.code_range(i, cache.bytes.len()) else {
                continue;
            };
            let d = decode_line(&cache.bytes[range], layout);
            for ins in &d.instructions {
                match (&ins.operands, ins.opcode) {
                    (
                        Operands::Rec(r),
                        Opcode::ProcHeader | Opcode::ProcHeaderAlt | Opcode::DeclareEvent,
                    ) => {
                        if !seen_functions.insert(r.0) {
                            continue;
                        }
                        let Ok(f) = FunctionRecord::parse(ind, r.0, layout) else {
                            continue;
                        };
                        add(&mut map, r.0, RecordKind::Function);
                        if let Some(args) =
                            walk_arguments(ind, f.first_arg, layout, &map, &mut bad_arguments)
                        {
                            for a in args {
                                add(&mut map, a.offset, RecordKind::Argument);
                                if a.has_as_clause() && desc(a.type_raw) {
                                    add(&mut map, a.type_raw, RecordKind::TypeDescriptor);
                                }
                            }
                        }
                        if f.has_return_type() && desc(f.return_type) {
                            add(&mut map, f.return_type, RecordKind::TypeDescriptor);
                        }
                    }
                    (Operands::VarDecl(r, _), Opcode::VarDecl) => {
                        if let Ok(v) = VariableRecord::parse(ind, r.0, layout) {
                            add(&mut map, r.0, RecordKind::Variable);
                            if v.has_as_clause() && desc(v.type_pos) {
                                add(&mut map, v.type_pos, RecordKind::TypeDescriptor);
                            }
                        }
                    }
                    (Operands::Rec(r), Opcode::TypeDecl) => {
                        add(&mut map, r.0, RecordKind::UserType);
                    }
                    _ => {}
                }
            }
        }
        map
    }

    /// Number of records per kind.
    pub fn counts(&self) -> BTreeMap<RecordKind, usize> {
        let mut c = BTreeMap::new();
        for k in self.records.values() {
            *c.entry(*k).or_default() += 1;
        }
        c
    }

    /// Bytes of the indirect table covered by the records, using
    /// [`RecordKind::size`], and the number of records whose extent overlaps
    /// the previous one or leaves the table. `None` when the layout has no
    /// observed sizes.
    pub fn coverage(&self, cache: &ModuleCache<'_>) -> Option<RecordCoverage> {
        let table_len = cache.tables.indirect.len();
        let mut cov = RecordCoverage {
            table: table_len,
            ..RecordCoverage::default()
        };
        let mut end = 0usize;
        for (&off, &kind) in &self.records {
            let size = kind.size(cache)?;
            let start = off as usize;
            if start < end || start.saturating_add(size) > table_len {
                cov.irregular += 1;
            }
            let stop = start.saturating_add(size).min(table_len);
            cov.covered += stop.saturating_sub(start.max(end));
            end = end.max(stop);
        }
        Some(cov)
    }
}

/// Result of [`RecordMap::coverage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecordCoverage {
    /// Size of the indirect table.
    pub table: usize,
    /// Bytes covered by reachable records.
    pub covered: usize,
    /// Records overlapping an earlier record or leaving the table.
    pub irregular: usize,
}
