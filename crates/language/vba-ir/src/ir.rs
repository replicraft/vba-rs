//! IR data types.

use vba_ast::{CompareMode, Name, OpenAccess, OpenLock, OpenMode};
use vba_sema::bound::Constant;
use vba_sema::{
    DeclareId, EventId, GlobalId, LibraryType, LocalId, ModuleId, ProcId, Program, Type, UdtId,
};
use vba_source::Span;
use vba_value::ops::{ArithOp, CompareOp, CompareOptions, LogicOp, Overflow};

/// A virtual register: a function-frame temporary assigned exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Reg(pub u32);

/// A basic block of a function; block 0 is the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

/// A source statement of a function (index into [`Function::stmts`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StmtId(pub u32);

/// A function of the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

/// An instruction operand.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// A register.
    Reg(Reg),
    /// A constant.
    Const(Constant),
}

/// How a store treats the stored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreMode {
    /// Value assignment (`Let`): the value is coerced to the declared type of
    /// the location by the lowering (explicit `Convert`) and copied.
    Let,
    /// Reference assignment (`Set`): the old reference is released and the new
    /// one retained (reference counting drives `Class_Terminate`).
    Set,
    /// `Set` of a `WithEvents` variable: also reconnects event handlers.
    SetWithEvents,
    /// `LSet`.
    LSet,
    /// `RSet`.
    RSet,
}

/// A reference-producing location.
#[derive(Debug, Clone, PartialEq)]
pub enum RefPlace {
    /// A local slot.
    Local(LocalId),
    /// A static variable of a standard module.
    Global(GlobalId),
    /// A field of a class instance.
    Field {
        /// The instance.
        obj: Operand,
        /// The field.
        global: GlobalId,
    },
    /// An array element; `base` holds a reference to the array.
    Elem {
        /// Reference to the array.
        base: Reg,
        /// Subscripts.
        indices: Vec<Operand>,
    },
    /// A UDT field; `base` holds a reference to the UDT value.
    UdtField {
        /// Reference to the UDT value.
        base: Reg,
        /// The UDT.
        udt: UdtId,
        /// Field index.
        index: u32,
    },
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// Negation.
    Neg,
    /// Unary plus.
    Plus,
    /// `Not`.
    Not,
}

/// Binary operators, expressed with the `vba-value` operator families so a
/// VM maps them one-to-one onto `vba_value::ops`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+ - * / \ Mod ^`.
    Arith(ArithOp),
    /// `&`.
    Concat,
    /// `= <> < > <= >=` with the string-comparison options in force.
    Compare(CompareOp, CompareOptions),
    /// `And Or Xor Eqv Imp`.
    Logic(LogicOp),
    /// `Like` with the `Option Compare` mode.
    Like(CompareMode),
    /// `Is`: reference equality.
    Is,
}

/// A call target.
#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    /// A project procedure. With a receiver it is dispatched on the
    /// receiver's class, honouring `Implements`.
    Proc(ProcId),
    /// An external procedure.
    Declare(DeclareId),
    /// A library intrinsic.
    Intrinsic {
        /// Canonical name.
        name: String,
        /// Called with the `$` suffix.
        dollar: bool,
    },
}

/// An argument of a call.
#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    /// By value.
    Val(Operand),
    /// By reference: a register holding a reference (from `MakeRef`).
    Ref(Reg),
    /// An omitted optional argument.
    Missing,
}

/// The member of a late-bound access.
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    /// By name.
    Named(String),
    /// The default member (DISPID 0). With arguments on a `Variant` receiver
    /// the run time decides between an array index and a default-member
    /// invocation.
    Default,
}

/// The `IDispatch::Invoke` flavour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokeKind {
    /// `DISPATCH_METHOD | DISPATCH_PROPERTYGET`.
    Get,
    /// `DISPATCH_PROPERTYPUT`.
    Put,
    /// `DISPATCH_PROPERTYPUTREF`.
    PutRef,
}

/// A late-bound argument; for `Put`/`PutRef` the value is the last one.
#[derive(Debug, Clone, PartialEq)]
pub struct LateArg {
    /// Name of a named argument.
    pub name: Option<String>,
    /// The argument.
    pub arg: CallArg,
}

pub use vba_sema::bound::NewTarget;

/// What `AddressOf` takes the address of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncRef {
    /// A project procedure.
    Proc(ProcId),
    /// An external procedure.
    Declare(DeclareId),
}

/// The error-handling state selected by `On Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrMode {
    /// No handler (`On Error GoTo 0`): errors propagate to the caller.
    Propagate,
    /// `On Error GoTo label`: control transfers to the block.
    Handler(BlockId),
    /// `On Error Resume Next`.
    ResumeNext,
}

/// An item of a `Print`/`Write`/`Debug.Print` list.
#[derive(Debug, Clone, PartialEq)]
pub enum OutItem {
    /// A value.
    Value(Operand),
    /// `;`
    Semicolon,
    /// `,`
    Comma,
}

/// File and I/O operations.
#[derive(Debug, Clone, PartialEq)]
pub enum IoOp {
    /// `Open path For mode ... As #file [Len = n]`.
    Open {
        /// Path.
        path: Operand,
        /// Mode.
        mode: OpenMode,
        /// Access.
        access: Option<OpenAccess>,
        /// Lock.
        lock: Option<OpenLock>,
        /// File number.
        file: Operand,
        /// Record length.
        record_len: Option<Operand>,
    },
    /// `Close` (empty closes all).
    Close(Vec<Operand>),
    /// `Print #file, items`.
    Print {
        /// File number.
        file: Operand,
        /// Items.
        items: Vec<OutItem>,
    },
    /// `Write #file, items`.
    Write {
        /// File number.
        file: Operand,
        /// Items.
        items: Vec<OutItem>,
    },
    /// `Input #file, targets` (references).
    Input {
        /// File number.
        file: Operand,
        /// Target references.
        targets: Vec<Reg>,
    },
    /// `Line Input #file, target`.
    LineInput {
        /// File number.
        file: Operand,
        /// Target reference.
        target: Reg,
    },
    /// `Get`.
    Get {
        /// File number.
        file: Operand,
        /// Record number.
        record: Option<Operand>,
        /// Target reference.
        target: Option<Reg>,
    },
    /// `Put`.
    Put {
        /// File number.
        file: Operand,
        /// Record number.
        record: Option<Operand>,
        /// Value.
        value: Option<Operand>,
    },
    /// `Seek`.
    Seek {
        /// File number.
        file: Operand,
        /// Position.
        position: Option<Operand>,
    },
    /// `Lock`.
    Lock {
        /// File number.
        file: Operand,
        /// Range start.
        from: Option<Operand>,
        /// Range end.
        to: Option<Operand>,
    },
    /// `Unlock`.
    Unlock {
        /// File number.
        file: Operand,
        /// Range start.
        from: Option<Operand>,
        /// Range end.
        to: Option<Operand>,
    },
    /// `Width`.
    Width {
        /// File number.
        file: Operand,
        /// Width.
        width: Option<Operand>,
    },
    /// `Name old As new`.
    Name {
        /// Old path.
        old: Operand,
        /// New path.
        new: Operand,
    },
}

/// An instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct Inst {
    /// Source span of the expression or statement it implements.
    pub span: Span,
    /// The statement it belongs to (for `Resume Next` and `Erl`).
    pub stmt: StmtId,
    /// What it does.
    pub kind: InstKind,
}

/// Instruction kinds. Every instruction that can fail raises a VBA run-time
/// error through the error-handling state; none has an implicit side effect
/// on registers other than its `dst`.
#[derive(Debug, Clone, PartialEq)]
pub enum InstKind {
    /// `dst = src`.
    Copy {
        /// Destination.
        dst: Reg,
        /// Source.
        src: Operand,
    },
    /// `dst = op src` with the static result type.
    Unary {
        /// Destination.
        dst: Reg,
        /// Operator.
        op: UnOp,
        /// Static result type.
        ty: Type,
        /// How integer overflow is handled.
        overflow: Overflow,
        /// Operand.
        src: Operand,
    },
    /// `dst = lhs op rhs` with the static result type. `overflow` is
    /// [`Overflow::Error`] when both operand types are statically typed and
    /// [`Overflow::Promote`] when a `Variant` is involved.
    Binary {
        /// Destination.
        dst: Reg,
        /// Operator.
        op: BinOp,
        /// Static result type.
        ty: Type,
        /// Overflow handling.
        overflow: Overflow,
        /// Left operand.
        lhs: Operand,
        /// Right operand.
        rhs: Operand,
    },
    /// Converts to a declared type as an assignment or `Cxxx` would
    /// (`vba_value::convert::convert_to`); to an object type it is a checked
    /// interface cast (`QueryInterface`), to `Variant` it boxes.
    Convert {
        /// Destination.
        dst: Reg,
        /// Source.
        src: Operand,
        /// Target type.
        to: Type,
    },
    /// Reads a local slot (a function-local variable, parameter or result).
    LoadLocal {
        /// Destination.
        dst: Reg,
        /// The slot.
        slot: LocalId,
    },
    /// Writes a local slot.
    StoreLocal {
        /// The slot.
        slot: LocalId,
        /// Value.
        src: Operand,
        /// Store flavour.
        mode: StoreMode,
    },
    /// Reads a static variable.
    LoadGlobal {
        /// Destination.
        dst: Reg,
        /// The variable.
        global: GlobalId,
    },
    /// Writes a static variable.
    StoreGlobal {
        /// The variable.
        global: GlobalId,
        /// Value.
        src: Operand,
        /// Store flavour.
        mode: StoreMode,
    },
    /// Reads a field of a class instance.
    LoadField {
        /// Destination.
        dst: Reg,
        /// The instance.
        obj: Operand,
        /// The field.
        global: GlobalId,
    },
    /// Writes a field of a class instance.
    StoreField {
        /// The instance.
        obj: Operand,
        /// The field.
        global: GlobalId,
        /// Value.
        src: Operand,
        /// Store flavour.
        mode: StoreMode,
    },
    /// `Me`.
    LoadMe {
        /// Destination.
        dst: Reg,
    },
    /// The predeclared default instance of a class-like module.
    LoadDefaultInstance {
        /// Destination.
        dst: Reg,
        /// The module.
        module: ModuleId,
    },
    /// A global object or constant provided by a library, by name.
    LoadLibGlobal {
        /// Destination.
        dst: Reg,
        /// Its name.
        name: String,
    },
    /// Creates a reference to a location.
    MakeRef {
        /// Destination (holds the reference).
        dst: Reg,
        /// The location.
        place: RefPlace,
    },
    /// Reads through a reference.
    LoadRef {
        /// Destination.
        dst: Reg,
        /// The reference.
        r: Reg,
    },
    /// Writes through a reference.
    StoreRef {
        /// The reference.
        r: Reg,
        /// Value.
        src: Operand,
        /// Store flavour.
        mode: StoreMode,
    },
    /// Reads an array element.
    Index {
        /// Destination.
        dst: Reg,
        /// The array value.
        array: Operand,
        /// Subscripts.
        indices: Vec<Operand>,
    },
    /// Reads a field of a user-defined type value.
    UdtField {
        /// Destination.
        dst: Reg,
        /// The UDT value.
        base: Operand,
        /// The UDT.
        udt: UdtId,
        /// Field index.
        index: u32,
    },
    /// Calls a procedure.
    Call {
        /// Destination of the result, if used.
        dst: Option<Reg>,
        /// The callee.
        callee: Callee,
        /// The object a method is invoked on.
        receiver: Option<Operand>,
        /// Arguments in parameter order (`ParamArray` extras last).
        args: Vec<CallArg>,
    },
    /// A late-bound member access (`IDispatch::Invoke`).
    Invoke {
        /// Destination of the result, if used.
        dst: Option<Reg>,
        /// The object.
        receiver: Operand,
        /// The member.
        member: Member,
        /// The flavour.
        kind: InvokeKind,
        /// Arguments (for puts the value is last).
        args: Vec<LateArg>,
        /// The receiver's library type when statically known.
        static_receiver: Option<LibraryType>,
        /// The member id the library declares: the access is early-bound.
        /// The arguments are then already in parameter order, converted to
        /// the declared types, with declared defaults filled in, and none is
        /// named. The run time invokes the member by id when the receiver
        /// accepts the id, and looks the name up otherwise.
        dispid: Option<i32>,
    },
    /// `New`.
    New {
        /// Destination.
        dst: Reg,
        /// What to create.
        target: NewTarget,
    },
    /// `As New` auto-instantiation: if the referenced variable is `Nothing`,
    /// create the object and store it.
    EnsureNew {
        /// Reference to the variable.
        r: Reg,
        /// What to create.
        target: NewTarget,
    },
    /// `AddressOf`.
    AddressOf {
        /// Destination.
        dst: Reg,
        /// The procedure.
        func: FuncRef,
    },
    /// `TypeOf x Is T`.
    TypeOfIs {
        /// Destination (Boolean).
        dst: Reg,
        /// The value.
        src: Operand,
        /// The type.
        ty: Type,
    },
    /// `ReDim [Preserve]`.
    ReDim {
        /// Reference to the array variable.
        r: Reg,
        /// `Preserve`.
        preserve: bool,
        /// `(lower, upper)` per dimension; a missing lower bound is
        /// `Option Base`.
        bounds: Vec<(Option<Operand>, Operand)>,
        /// Element type.
        elem: Type,
    },
    /// Allocates a fixed-size array (declaration-time `Dim a(1 To n)`).
    DimFixed {
        /// Reference to the variable.
        r: Reg,
        /// Constant bounds per dimension.
        bounds: Vec<(i32, i32)>,
        /// Element type.
        elem: Type,
    },
    /// `Erase`.
    Erase {
        /// Reference to the array variable.
        r: Reg,
    },
    /// Starts a `For Each` enumeration over an array or collection.
    IterInit {
        /// Destination (iterator state).
        dst: Reg,
        /// The array or collection.
        src: Operand,
    },
    /// Tests the loop condition of a `For` loop: true when
    /// `step >= 0 ? counter <= end : counter >= end`.
    ForTest {
        /// Destination (Boolean).
        dst: Reg,
        /// Current counter value.
        counter: Operand,
        /// End value.
        end: Operand,
        /// Step value.
        step: Operand,
    },
    /// Selects the error-handling state (`On Error`).
    SetErrorMode {
        /// The state.
        mode: ErrMode,
    },
    /// `On Error GoTo -1`: clears the pending error and the active-handler
    /// flag.
    ClearError,
    /// A line-number label was executed (`Erl`).
    SetLine {
        /// The line number.
        line: u64,
    },
    /// `RaiseEvent`.
    RaiseEvent {
        /// The event.
        event: EventId,
        /// Arguments.
        args: Vec<CallArg>,
    },
    /// `Debug.Print`.
    DebugPrint {
        /// Items.
        items: Vec<OutItem>,
    },
    /// File statements.
    Io(IoOp),
}

/// How a `Resume` continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeKind {
    /// Re-execute the instruction that failed.
    Same,
    /// Continue at the [`StmtInfo::resume_next`] point of the failed
    /// statement.
    Next,
    /// Continue at a block.
    Block(BlockId),
}

/// A block terminator.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    /// Unconditional jump.
    Jump(BlockId),
    /// Conditional jump. A `Boolean` is tested directly; for a `Variant` the
    /// `If` rule applies: `Null` is false, anything else is converted with
    /// `CBool`.
    Branch {
        /// Condition.
        cond: Operand,
        /// Target when true.
        then_block: BlockId,
        /// Target when false.
        else_block: BlockId,
    },
    /// `On expr GoTo`: selector 1..=n picks `targets[n-1]`, 0 or greater
    /// than n falls to `default`, negative or above 255 raises error 5.
    Switch {
        /// Selector (Long).
        selector: Operand,
        /// Targets.
        targets: Vec<BlockId>,
        /// Fall-through block.
        default: BlockId,
    },
    /// `GoSub`: pushes `ret` on the frame's return stack and jumps.
    GoSub {
        /// Target.
        target: BlockId,
        /// Where `Return` continues.
        ret: BlockId,
    },
    /// `Return`: pops the return stack (error 3 when empty).
    ReturnFromGoSub,
    /// Advances a `For Each` enumeration: when an element exists it is stored
    /// in `item` and control goes to `body`, otherwise to `done`.
    ForEachNext {
        /// Iterator from `IterInit`.
        iter: Reg,
        /// Receives the element.
        item: Reg,
        /// Loop body.
        body: BlockId,
        /// After the loop.
        done: BlockId,
    },
    /// `Resume`.
    Resume(ResumeKind),
    /// Returns from the procedure. A function's value is the result slot;
    /// all object-typed slots are released.
    Return,
    /// `End`.
    End,
    /// `Stop`.
    Stop,
    /// Control cannot reach here.
    Unreachable,
}

/// A basic block.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Straight-line instructions.
    pub insts: Vec<Inst>,
    /// The terminator.
    pub term: Terminator,
    /// Statement of the terminator.
    pub term_stmt: StmtId,
    /// Source span of the terminator.
    pub term_span: Span,
}

/// A position inside a function: before instruction `index` of `block`
/// (`index == insts.len()` is the terminator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// The block.
    pub block: BlockId,
    /// Instruction index.
    pub index: u32,
}

/// Debug and `Resume` information of a source statement.
#[derive(Debug, Clone, PartialEq)]
pub struct StmtInfo {
    /// Statement span.
    pub span: Span,
    /// The most recent line-number label preceding the statement in source
    /// order (a static hint; the dynamic value is set by `SetLine`).
    pub line: Option<u64>,
    /// Where `Resume Next` continues after an error inside the statement.
    pub resume_next: Option<Point>,
}

/// The origin of a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncKind {
    /// A project procedure.
    Proc(ProcId),
    /// The initialiser of a module: allocates fixed-size arrays and runs
    /// other declaration-time initialisation. For a class it runs on a new
    /// instance before `Class_Initialize`; for a standard module once at
    /// program start.
    ModuleInit(ModuleId),
}

/// A slot of a function frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// Name.
    pub name: Name,
    /// Static type.
    pub ty: Type,
    /// Whether the slot is a parameter, the result or a temporary.
    pub role: SlotRole,
}

/// Role of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRole {
    /// The n-th parameter.
    Param(u32),
    /// The result.
    Result,
    /// A local variable.
    Local,
    /// A compiler temporary (`With` object).
    Temp,
}

/// A function: one procedure or module initialiser.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Origin.
    pub kind: FuncKind,
    /// Display name (`Module.Proc`).
    pub name: String,
    /// Frame slots: parameters first, then the result, then locals.
    pub slots: Vec<Slot>,
    /// Types of registers.
    pub regs: Vec<Type>,
    /// Blocks; block 0 is the entry.
    pub blocks: Vec<Block>,
    /// Statement table.
    pub stmts: Vec<StmtInfo>,
}

/// Lifetime and dispatch information of a class-like module.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    /// The module.
    pub module: ModuleId,
    /// `Class_Initialize`.
    pub initialize: Option<ProcId>,
    /// `Class_Terminate`.
    pub terminate: Option<ProcId>,
    /// The field initialiser.
    pub init: Option<FuncId>,
    /// Instance fields (public and private variables).
    pub fields: Vec<GlobalId>,
    /// Implemented interfaces with their member mapping.
    pub implements: Vec<vba_sema::ImplementsInfo>,
    /// Event handlers for `WithEvents` fields.
    pub handlers: Vec<vba_sema::EventHandler>,
    /// Events the class raises.
    pub events: Vec<EventId>,
    /// Whether a predeclared default instance exists.
    pub predeclared: bool,
}

/// A lowered project.
#[derive(Debug, Clone)]
pub struct IrProgram {
    /// The semantic model the IR refers to (types, variables, signatures).
    pub program: Program,
    /// Functions. The first `program.procs.len()` entries correspond to
    /// procedures by [`ProcId`]; module initialisers follow.
    pub functions: Vec<Function>,
    /// The initialiser of each module, if it has one.
    pub module_init: Vec<Option<FuncId>>,
    /// Class information per module (`None` for standard modules).
    pub classes: Vec<Option<ClassInfo>>,
}

impl IrProgram {
    /// The function of a procedure.
    pub fn function_of(&self, p: ProcId) -> &Function {
        &self.functions[p.index()]
    }
}
