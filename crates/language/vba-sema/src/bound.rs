//! The bound, typed model of procedure bodies.
//!
//! Every name is resolved to an identifier, every call has been classified
//! (procedure, `Declare`, intrinsic, late-bound), implicit conversions are
//! explicit [`BExprKind::Convert`] nodes, and `ByRef`/`ByVal`/missing
//! arguments are spelled out in positional order.

use vba_ast::{BinaryOp, CompareMode, ExitKind, Name, OpenAccess, OpenLock, OpenMode, UnaryOp};
use vba_source::Span;
use vba_value::Variant;

use crate::types::{
    DeclareId, EventId, GlobalId, LabelId, LibraryType, LocalId, ModuleId, ProcId, Type, UdtId,
};

/// A compile-time constant value.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// A value of a `Variant` subtype.
    Value(Variant),
    /// A date literal whose text could not be evaluated statically; the text
    /// between the `#`s, to be parsed by the run time with its locale.
    DateText(Vec<u8>),
}

/// A typed, bound expression.
#[derive(Debug, Clone, PartialEq)]
pub struct BExpr {
    /// Source span.
    pub span: Span,
    /// Static type of the value.
    pub ty: Type,
    /// What it is.
    pub kind: BExprKind,
}

impl BExpr {
    /// Creates an expression.
    pub fn new(span: Span, ty: Type, kind: BExprKind) -> BExpr {
        BExpr { span, ty, kind }
    }
}

/// The target of `New`.
#[derive(Debug, Clone, PartialEq)]
pub enum NewTarget {
    /// A class module of the project.
    Class(ModuleId),
    /// A library class.
    Library(LibraryType),
}

/// The target of `AddressOf`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressTarget {
    /// A project procedure.
    Proc(ProcId),
    /// A `Declare`d external procedure.
    Declare(DeclareId),
}

/// Expression kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum BExprKind {
    /// A constant.
    Const(Constant),
    /// A local slot.
    Local(LocalId),
    /// A static variable of a standard module.
    Global(GlobalId),
    /// A public variable (field) of a class instance.
    Field {
        /// The instance (`Me` for an unqualified reference inside the class).
        receiver: Box<BExpr>,
        /// The field.
        global: GlobalId,
    },
    /// `Me`.
    Me,
    /// A global object or constant provided by a library (`Err`,
    /// `Application`), resolved by name at run time.
    LibGlobal(String),
    /// The predeclared default instance of a class, document or designer
    /// module (`UserForm1`, `Sheet1`, `Attribute VB_PredeclaredId`).
    DefaultInstance(ModuleId),
    /// Unary operator; the operand is already converted where required.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        operand: Box<BExpr>,
    },
    /// Binary operator.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        lhs: Box<BExpr>,
        /// Right operand.
        rhs: Box<BExpr>,
        /// `Option Compare` mode in force (comparison and `Like` only).
        compare: CompareMode,
    },
    /// Implicit conversion of the operand to this expression's type.
    Convert(Box<BExpr>),
    /// A statically bound call.
    Call(Box<BCall>),
    /// An array element.
    Index {
        /// The array.
        base: Box<BExpr>,
        /// Subscripts.
        indices: Vec<BExpr>,
    },
    /// A field of a user-defined type value.
    UdtField {
        /// The value.
        base: Box<BExpr>,
        /// The UDT.
        udt: UdtId,
        /// Field index.
        index: u32,
    },
    /// A late-bound access.
    Late(Box<BLate>),
    /// `New T`.
    New(NewTarget),
    /// `AddressOf p`.
    AddressOf(AddressTarget),
    /// `TypeOf e Is T`.
    TypeOfIs {
        /// The tested value.
        expr: Box<BExpr>,
        /// The type.
        ty: Type,
    },
}

/// What a call statically resolves to.
#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    /// A procedure of the project. With a receiver it is a method of that
    /// object; calls through an `Implements` interface are dispatched at run
    /// time to the implementing class.
    Proc(ProcId),
    /// An external `Declare`d procedure.
    Declare(DeclareId),
    /// An intrinsic function (`vba-stdlib` registry or run-time provided).
    Intrinsic {
        /// Canonical name.
        name: String,
        /// Called with the `$` suffix.
        dollar: bool,
    },
}

/// A statically bound call.
#[derive(Debug, Clone, PartialEq)]
pub struct BCall {
    /// The callee.
    pub target: CallTarget,
    /// The object the procedure is invoked on (`Me` or an explicit instance).
    pub receiver: Option<BExpr>,
    /// Arguments in parameter order. A `ParamArray` callee gets its extra
    /// arguments appended.
    pub args: Vec<BArg>,
}

/// A bound argument.
#[derive(Debug, Clone, PartialEq)]
pub enum BArg {
    /// Passed by value.
    Value(BExpr),
    /// Passed by reference to a variable, element or field.
    Ref(BPlace),
    /// An omitted `Optional` argument.
    Missing,
}

/// The member a late-bound access names.
#[derive(Debug, Clone, PartialEq)]
pub enum LateMember {
    /// `obj.Name` / `obj!Name` (the key is then an argument).
    Named(Name),
    /// The default member (DISPID 0). With arguments on a `Variant` receiver
    /// it is an array index or default-member invocation, decided at run
    /// time.
    Default,
}

/// The access kind of a late-bound member (the `IDispatch::Invoke` flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateKind {
    /// `DISPATCH_METHOD | DISPATCH_PROPERTYGET`.
    Get,
    /// `DISPATCH_PROPERTYPUT` (`Let`).
    PropertyPut,
    /// `DISPATCH_PROPERTYPUTREF` (`Set`).
    PropertyPutRef,
}

/// A late-bound argument: optionally named.
#[derive(Debug, Clone, PartialEq)]
pub struct BLateArg {
    /// Name of a named argument.
    pub name: Option<Name>,
    /// The value.
    pub arg: BArg,
}

/// A late-bound access resolved at run time by name.
#[derive(Debug, Clone, PartialEq)]
pub struct BLate {
    /// The object.
    pub receiver: BExpr,
    /// The member.
    pub member: LateMember,
    /// The access kind.
    pub kind: LateKind,
    /// Arguments.
    pub args: Vec<BLateArg>,
    /// The library type of the receiver when it is statically known.
    pub static_receiver: Option<LibraryType>,
    /// The member id (`DISPID`) the library declares for the member. When
    /// present, the access is early-bound: the arguments are already in
    /// parameter order, converted to the declared parameter types, with the
    /// declared defaults filled in, and the run time invokes the member by
    /// id.
    pub dispid: Option<i32>,
}

/// An assignable location.
#[derive(Debug, Clone, PartialEq)]
pub enum BPlace {
    /// A local slot (including the function result slot).
    Local(LocalId),
    /// A static variable.
    Global(GlobalId),
    /// A field of a class instance.
    Field {
        /// The instance.
        receiver: Box<BExpr>,
        /// The field.
        global: GlobalId,
    },
    /// An array element.
    Index {
        /// The array location.
        base: Box<BPlace>,
        /// Subscripts.
        indices: Vec<BExpr>,
        /// Element type.
        elem: Type,
    },
    /// A field of a user-defined type location.
    UdtField {
        /// The UDT location.
        base: Box<BPlace>,
        /// The UDT.
        udt: UdtId,
        /// Field index.
        index: u32,
        /// Field type.
        ty: Type,
    },
    /// `Property Let`/`Set` of a project property: assignment is a call with
    /// the value as last argument.
    Property {
        /// The call without the value argument.
        call: Box<BCall>,
        /// Whether `Property Set` (otherwise `Let`).
        set: bool,
        /// Type of the assigned value.
        ty: Type,
    },
    /// A late-bound property put.
    Late {
        /// The access without the value argument (`kind` is the put kind).
        late: Box<BLate>,
        /// Type of the assigned value (`Variant`).
        ty: Type,
        /// The location of a `Variant` receiver, when it has one: the run
        /// time stores into the array element when the variable holds an
        /// array (`v(i) = x`), and invokes the default member otherwise.
        receiver_place: Option<Box<BPlace>>,
    },
}

/// Which assignment statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignMode {
    /// `Let` (value assignment, may use a default member).
    Let,
    /// `Set` (reference assignment).
    Set,
    /// `LSet`.
    LSet,
    /// `RSet`.
    RSet,
}

/// An item of a `Print`/`Write` list.
#[derive(Debug, Clone, PartialEq)]
pub enum BOutItem {
    /// A value.
    Value(BExpr),
    /// `;`
    Semicolon,
    /// `,`
    Comma,
}

/// A `Case` item.
#[derive(Debug, Clone, PartialEq)]
pub enum BCaseItem {
    /// `Case v`
    Value(BExpr),
    /// `Case a To b`
    Range(BExpr, BExpr),
    /// `Case Is op v`
    Is(BinaryOp, BExpr),
}

/// A `Case` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct BCase {
    /// Items (empty for `Case Else`).
    pub items: Vec<BCaseItem>,
    /// Whether `Case Else`.
    pub is_else: bool,
    /// Body.
    pub body: Vec<BStmt>,
}

/// `On Error` actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BOnError {
    /// `GoTo label`.
    GoTo(LabelId),
    /// `GoTo 0`: disable the handler.
    Disable,
    /// `GoTo -1`: clear the error state.
    ClearError,
    /// `Resume Next`.
    ResumeNext,
}

/// `Resume` forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BResume {
    /// Re-execute the faulting statement.
    Same,
    /// Continue after the faulting statement.
    Next,
    /// Continue at a label.
    Label(LabelId),
}

/// A bound bounds pair of `ReDim`.
#[derive(Debug, Clone, PartialEq)]
pub struct BBound {
    /// Lower bound; `None` means `Option Base`.
    pub lower: Option<BExpr>,
    /// Upper bound.
    pub upper: BExpr,
}

/// One `ReDim` target.
#[derive(Debug, Clone, PartialEq)]
pub struct BReDim {
    /// The array variable.
    pub target: BPlace,
    /// New bounds.
    pub bounds: Vec<BBound>,
    /// Element type.
    pub elem: Type,
}

/// File and I/O statements.
#[derive(Debug, Clone, PartialEq)]
pub enum IoStmt {
    /// `Open`.
    Open {
        /// Path.
        path: BExpr,
        /// Mode (`Random` when omitted).
        mode: OpenMode,
        /// Access clause.
        access: Option<OpenAccess>,
        /// Lock clause.
        lock: Option<OpenLock>,
        /// File number.
        file: BExpr,
        /// Record length.
        record_len: Option<BExpr>,
    },
    /// `Close` (empty closes all).
    Close(Vec<BExpr>),
    /// `Print #n, ...`.
    Print {
        /// File number.
        file: BExpr,
        /// Items.
        items: Vec<BOutItem>,
    },
    /// `Write #n, ...`.
    Write {
        /// File number.
        file: BExpr,
        /// Items.
        items: Vec<BOutItem>,
    },
    /// `Input #n, vars`.
    Input {
        /// File number.
        file: BExpr,
        /// Targets.
        targets: Vec<BPlace>,
    },
    /// `Line Input #n, var`.
    LineInput {
        /// File number.
        file: BExpr,
        /// Target.
        target: BPlace,
    },
    /// `Get`.
    Get {
        /// File number.
        file: BExpr,
        /// Record.
        record: Option<BExpr>,
        /// Target.
        target: Option<BPlace>,
    },
    /// `Put`.
    Put {
        /// File number.
        file: BExpr,
        /// Record.
        record: Option<BExpr>,
        /// Value.
        value: Option<BExpr>,
    },
    /// `Seek`.
    Seek {
        /// File number.
        file: BExpr,
        /// Position.
        position: Option<BExpr>,
    },
    /// `Lock`.
    Lock {
        /// File number.
        file: BExpr,
        /// Start.
        from: Option<BExpr>,
        /// End.
        to: Option<BExpr>,
    },
    /// `Unlock`.
    Unlock {
        /// File number.
        file: BExpr,
        /// Start.
        from: Option<BExpr>,
        /// End.
        to: Option<BExpr>,
    },
    /// `Width`.
    Width {
        /// File number.
        file: BExpr,
        /// Width.
        width: Option<BExpr>,
    },
    /// `Name old As new`.
    Name {
        /// Old path.
        old: BExpr,
        /// New path.
        new: BExpr,
    },
}

/// A bound statement.
#[derive(Debug, Clone, PartialEq)]
pub struct BStmt {
    /// Source span.
    pub span: Span,
    /// What it is.
    pub kind: BStmtKind,
}

/// Statement kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum BStmtKind {
    /// A label or line number definition.
    Label(LabelId),
    /// An assignment; the value is already converted to the target's type.
    Assign {
        /// Target.
        target: BPlace,
        /// Value.
        value: BExpr,
        /// Which assignment.
        mode: AssignMode,
    },
    /// A call whose result (if any) is discarded.
    Call(BExpr),
    /// `If`/`ElseIf`/`Else`: arms are tried in order.
    If {
        /// `(condition, body)` pairs.
        arms: Vec<(BExpr, Vec<BStmt>)>,
        /// `Else` body.
        else_body: Option<Vec<BStmt>>,
    },
    /// `Select Case`.
    Select {
        /// The tested expression.
        subject: BExpr,
        /// Clauses.
        cases: Vec<BCase>,
    },
    /// `For`.
    For {
        /// Control variable.
        counter: BPlace,
        /// Type of the control variable.
        counter_ty: Type,
        /// Start.
        start: BExpr,
        /// End.
        end: BExpr,
        /// Step.
        step: Option<BExpr>,
        /// Body.
        body: Vec<BStmt>,
    },
    /// `For Each`.
    ForEach {
        /// Control variable.
        item: BPlace,
        /// Type of the control variable.
        item_ty: Type,
        /// The collection or array.
        collection: BExpr,
        /// Body.
        body: Vec<BStmt>,
    },
    /// `Do ... Loop`, `While ... Wend`.
    Do {
        /// Condition before the body: `(is_until, condition)`.
        pre: Option<(bool, BExpr)>,
        /// Condition after the body.
        post: Option<(bool, BExpr)>,
        /// Body.
        body: Vec<BStmt>,
        /// Written as `While ... Wend` (which `Exit Do` does not leave).
        is_while: bool,
    },
    /// `With` on an object: the object is evaluated once into a hidden local.
    With {
        /// Hidden local holding the object.
        local: LocalId,
        /// The object expression.
        object: BExpr,
        /// Body.
        body: Vec<BStmt>,
    },
    /// A plain statement sequence (`With` on a user-defined type value, whose
    /// members are bound directly to the underlying variable).
    Block(Vec<BStmt>),
    /// `GoTo`.
    GoTo(LabelId),
    /// `GoSub`.
    GoSub(LabelId),
    /// `Return` (from `GoSub`).
    Return,
    /// `On Error`.
    OnError(BOnError),
    /// `On expr GoTo|GoSub`.
    OnGoTo {
        /// Selector (converted to `Long`).
        selector: BExpr,
        /// Targets.
        targets: Vec<LabelId>,
        /// `GoSub`.
        is_gosub: bool,
    },
    /// `Resume`.
    Resume(BResume),
    /// `Exit Sub|Function|Property|For|Do`.
    Exit(ExitKind),
    /// `End`.
    End,
    /// `Stop`.
    Stop,
    /// `ReDim`.
    ReDim {
        /// `Preserve`.
        preserve: bool,
        /// Targets.
        items: Vec<BReDim>,
    },
    /// `Erase`.
    Erase(Vec<BPlace>),
    /// File statements.
    Io(IoStmt),
    /// `RaiseEvent`.
    RaiseEvent {
        /// The event.
        event: EventId,
        /// Arguments.
        args: Vec<BArg>,
    },
    /// `Debug.Print`.
    DebugPrint(Vec<BOutItem>),
}
