//! Run-time data structures: locations, frames, objects.

use vba_automation::Cookie;
use vba_ir::{BlockId, ErrMode, Reg, StmtId};
use vba_sema::{GlobalId, ModuleId, ProcId};
use vba_stdlib::interaction::PrintHelper;
use vba_value::{ObjectHandle, UdtHandle, Variant, VbaString};

/// Handles with this bit set name objects owned by the VM; all others name
/// objects in the host's [`vba_automation::ObjectTable`].
pub(crate) const OBJ_BIT: u64 = vba_automation::FOREIGN_HANDLE_BIT;
/// Id of the `Err` object.
pub(crate) const ERR_ID: u64 = 1;
/// Id of the `Debug` object.
pub(crate) const DEBUG_ID: u64 = 2;

pub(crate) fn vm_handle(id: u64) -> ObjectHandle {
    ObjectHandle(id | OBJ_BIT)
}

pub(crate) fn vm_id(h: ObjectHandle) -> Option<u64> {
    (h.0 & OBJ_BIT != 0).then_some(h.0 & !OBJ_BIT)
}

/// Array subscripts; up to three are stored inline so that taking a
/// reference to an element does not allocate for them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Idx {
    Inline(u8, [i32; 3]),
    Heap(Vec<i32>),
}

impl Idx {
    pub(crate) fn from_slice(s: &[i32]) -> Idx {
        if s.len() <= 3 {
            let mut v = [0; 3];
            v[..s.len()].copy_from_slice(s);
            Idx::Inline(s.len() as u8, v)
        } else {
            Idx::Heap(s.to_vec())
        }
    }
}

impl From<Vec<i32>> for Idx {
    fn from(v: Vec<i32>) -> Idx {
        Idx::from_slice(&v)
    }
}

impl std::ops::Deref for Idx {
    type Target = [i32];
    fn deref(&self) -> &[i32] {
        match self {
            Idx::Inline(n, v) => &v[..usize::from(*n)],
            Idx::Heap(v) => v,
        }
    }
}

/// A storage location a reference can designate.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Loc {
    /// A slot of the frame at this index of the frame stack.
    Local { frame: usize, slot: u32 },
    /// A variable of a standard module (index by [`GlobalId`]).
    Global(u32),
    /// A field of a class instance (index into the instance's fields).
    Field { obj: u64, index: u32 },
    /// An element of the array stored at `base`.
    Elem { base: Box<Loc>, idx: Idx },
    /// A field of the user-defined-type value stored at `base`.
    UdtField { base: Box<Loc>, index: u32 },
}

/// A frame slot: a value, or a binding to a caller's location (`ByRef`).
#[derive(Debug, Clone)]
pub(crate) enum Slot {
    Val(Variant),
    Bound(Loc),
}

/// A virtual register.
#[derive(Debug, Default)]
pub(crate) enum RegVal {
    #[default]
    Empty,
    V(Variant),
    /// A reference produced by `MakeRef`.
    Ref(Loc),
    /// A lazily loaded array (avoids copying an array to index it).
    Alias(Loc),
    /// A `For Each` iterator.
    Iter(Box<Iter>),
    /// The result of `Tab` or `Spc`.
    Print(PrintHelper),
}

/// The state of a `For Each` loop.
#[derive(Debug)]
pub(crate) enum Iter {
    Array { items: Vec<Variant>, pos: usize },
    Coll { id: u64, pos: usize },
    Ext { enumerator: ObjectHandle },
}

/// One activation of a procedure (or module initialiser).
#[derive(Debug)]
pub(crate) struct Frame {
    pub func: usize,
    pub proc: Option<ProcId>,
    pub module: ModuleId,
    pub block: u32,
    pub idx: u32,
    pub slots: Vec<Slot>,
    pub regs: Vec<RegVal>,
    pub me: Option<u64>,
    pub mode: ErrMode,
    pub in_handler: bool,
    pub fail: Option<(u32, u32, StmtId)>,
    pub gosub: Vec<BlockId>,
    pub line: u64,
    pub owned: Vec<Variant>,
    pub held: Vec<Variant>,
    pub ret_dst: Option<Reg>,
    pub cur_stmt: u32,
}

/// A user-defined-type value.
#[derive(Debug, Clone)]
pub(crate) struct UdtVal {
    pub udt: vba_sema::UdtId,
    pub fields: Vec<Variant>,
}

/// An event sink connected to a source object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sink {
    pub obj: u64,
    pub var: GlobalId,
}

/// An event source outside the VM that a `WithEvents` variable listens to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExtConn {
    pub var: GlobalId,
    pub source: ObjectHandle,
    pub cookie: Cookie,
}

/// An item of a `Collection`.
#[derive(Debug, Clone)]
pub(crate) struct CollItem {
    pub key: Option<String>,
    pub value: Variant,
}

/// The items of a `Collection` in insertion order, with a key index.
///
/// The index holds every key and maps it to a position hint. Hints of items
/// below `clean` are exact; a hint is always checked against the item before
/// use, so a stale one can only cost a scan, never a wrong answer. Appending
/// keeps everything clean; inserting or removing in the middle lowers `clean`
/// to that position. A lookup of a key whose hint is not usable scans from
/// `clean` without touching the index for the first few lookups after an edit
/// (a program that edits between lookups pays what a linear scan costs), then
/// repairs the hints on its way and raises `clean`, so a run of lookups after
/// one edit is amortised O(1) each.
#[derive(Debug, Default)]
pub(crate) struct Coll {
    items: Vec<CollItem>,
    index: std::collections::HashMap<String, usize>,
    clean: usize,
    /// Plain scans since the last edit.
    scans: u8,
}

const PLAIN_SCANS: u8 = 3;

impl Coll {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, pos: usize) -> Option<&CollItem> {
        self.items.get(pos)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CollItem> {
        self.items.iter()
    }

    pub fn into_values(self) -> Vec<Variant> {
        self.items.into_iter().map(|i| i.value).collect()
    }

    /// Whether an item has this (already lower-cased) key.
    pub fn contains(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// Position of the item with this (already lower-cased) key.
    pub fn find(&mut self, key: &str) -> Option<usize> {
        let &h = self.index.get(key)?;
        if h < self.clean
            && self
                .items
                .get(h)
                .is_some_and(|i| i.key.as_deref() == Some(key))
        {
            return Some(h);
        }
        // The key exists and lies at or beyond `clean`.
        if self.scans < PLAIN_SCANS {
            self.scans += 1;
            return (self.clean..self.items.len())
                .find(|&at| self.items[at].key.as_deref() == Some(key));
        }
        let mut at = self.clean;
        while at < self.items.len() {
            let found = if let Some(k) = self.items[at].key.as_deref() {
                if let Some(slot) = self.index.get_mut(k) {
                    *slot = at;
                }
                k == key
            } else {
                false
            };
            at += 1;
            if found {
                self.clean = at;
                return Some(at - 1);
            }
        }
        self.clean = at;
        None
    }

    pub fn insert(&mut self, at: usize, item: CollItem) {
        let append = at == self.items.len();
        if let Some(k) = &item.key {
            self.index.insert(k.clone(), at);
        }
        self.items.insert(at, item);
        if append && self.clean == at {
            self.clean = at + 1;
        } else {
            self.clean = self.clean.min(at);
        }
        self.scans = 0;
    }

    pub fn remove(&mut self, at: usize) -> CollItem {
        let item = self.items.remove(at);
        if let Some(k) = &item.key {
            self.index.remove(k);
        }
        self.clean = self.clean.min(at);
        self.scans = 0;
        item
    }
}

/// What an object is.
#[derive(Debug)]
pub(crate) enum ObjKind {
    Class {
        module: ModuleId,
        fields: Vec<Variant>,
        ext_conns: Vec<ExtConn>,
    },
    Err,
    Debug,
    Collection(Coll),
}

/// An object owned by the VM.
#[derive(Debug)]
pub(crate) struct Obj {
    pub kind: ObjKind,
    pub refs: u32,
    pub terminating: bool,
    pub sinks: Vec<Sink>,
    /// The host object that supplies the members a document or designer
    /// module does not define (owned: one reference in the host's table).
    pub host_base: Option<ObjectHandle>,
}

/// References released by the destruction of an object, processed by the
/// loop that started the cascade so that long chains do not nest the Rust
/// stack.
#[derive(Debug)]
pub(crate) struct Cascade {
    /// Frame count at which the cascade runs.
    pub base: usize,
    /// Values still to release, next one last.
    pub stack: Vec<Variant>,
}

/// An event from an object outside the VM, waiting for delivery.
#[derive(Debug)]
pub(crate) struct PendingEvent {
    pub sink: u64,
    pub var: GlobalId,
    pub event: String,
    pub args: Vec<Variant>,
}

/// Where an error was raised.
#[derive(Debug, Clone, Default)]
pub(crate) struct Site {
    pub module: String,
    /// `Module.Procedure`.
    pub procedure: String,
    pub line: u64,
    pub source_line: u32,
    pub stack: Vec<String>,
    pub frames: Vec<crate::error::StackFrame>,
}

/// The `Err` object's contents.
#[derive(Debug, Clone, Default)]
pub(crate) struct ErrState {
    /// Where the error in `number` was raised (set when it is handled).
    pub site: Option<Site>,
    pub number: i32,
    pub description: String,
    pub source: String,
    pub help_file: String,
    pub help_context: i32,
    pub last_dll_error: i32,
}

pub(crate) fn nul_string(n: u32) -> Variant {
    Variant::String(VbaString::from_units(vec![0; n as usize]))
}

pub(crate) fn udt_variant(id: u64) -> Variant {
    Variant::UserDefined(UdtHandle(id))
}

#[cfg(test)]
mod coll_tests {
    use super::*;

    fn naive(items: &[CollItem], key: &str) -> Option<usize> {
        items.iter().position(|i| i.key.as_deref() == Some(key))
    }

    /// The key index agrees with a linear scan over random sequences of
    /// appends, middle inserts, removes and lookups.
    #[test]
    fn key_index_matches_linear_scan() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        for _ in 0..200 {
            let mut c = Coll::default();
            let mut model: Vec<CollItem> = Vec::new();
            for _ in 0..300 {
                let key = format!("k{}", next(40));
                match next(5) {
                    0 | 1 => {
                        let keyed = next(4) != 0;
                        if keyed && naive(&model, &key).is_some() {
                            assert!(c.find(&key).is_some());
                            continue;
                        }
                        let at = if next(2) == 0 {
                            model.len()
                        } else {
                            next(model.len() as u64 + 1) as usize
                        };
                        let item = CollItem {
                            key: keyed.then(|| key.clone()),
                            value: Variant::Long(0),
                        };
                        model.insert(at, item.clone());
                        c.insert(at, item);
                    }
                    2 if !model.is_empty() => {
                        let at = next(model.len() as u64) as usize;
                        model.remove(at);
                        c.remove(at);
                    }
                    _ => {
                        assert_eq!(c.find(&key), naive(&model, &key));
                    }
                }
                assert_eq!(c.len(), model.len());
            }
            for i in 0..40 {
                let key = format!("k{i}");
                assert_eq!(c.find(&key), naive(&model, &key));
            }
        }
    }
}
