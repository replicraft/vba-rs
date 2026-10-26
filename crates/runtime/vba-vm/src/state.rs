//! Run-time data structures: locations, frames, objects.

use vba_automation::Cookie;
use vba_ir::{BlockId, ErrMode, Reg, StmtId};
use vba_sema::{GlobalId, ModuleId, ProcId};
use vba_stdlib::interaction::PrintHelper;
use vba_value::{ObjectHandle, UdtHandle, Variant, VbaString};

/// Handles with this bit set name objects owned by the VM; all others name
/// objects in the host's [`vba_automation::ObjectTable`].
pub(crate) const OBJ_BIT: u64 = 1 << 62;
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
    Elem { base: Box<Loc>, idx: Vec<i32> },
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
    Collection(Vec<CollItem>),
}

/// An object owned by the VM.
#[derive(Debug)]
pub(crate) struct Obj {
    pub kind: ObjKind,
    pub refs: u32,
    pub terminating: bool,
    pub sinks: Vec<Sink>,
}

/// An event from an object outside the VM, waiting for delivery.
#[derive(Debug)]
pub(crate) struct PendingEvent {
    pub sink: u64,
    pub var: GlobalId,
    pub event: String,
    pub args: Vec<Variant>,
}

/// The `Err` object's contents.
#[derive(Debug, Clone, Default)]
pub(crate) struct ErrState {
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
