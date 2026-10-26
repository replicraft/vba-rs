//! AST node types.
//!
//! Declarations ([`Item`], [`VarDecl`], [`ConstDecl`], ...) and executable
//! statements ([`StmtKind`]) are separate types. Local `Dim`/`Static`/`Const`
//! appear in procedure bodies as [`StmtKind::Dim`] and [`StmtKind::Const`]
//! wrapping the same declaration structs used at module level.

use vba_lexer::TypeSuffix;
use vba_source::Span;

use crate::name::{Ident, Name};

// ---- module ----------------------------------------------------------------

/// A parsed module (MS-VBAL §4).
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The whole module.
    pub span: Span,
    /// Exported-file header (`VERSION`, `BEGIN ... END`), if present.
    pub header: Option<ModuleHeader>,
    /// Declarations, procedures, attributes and conditional-compilation
    /// markers in source order.
    pub items: Vec<Item>,
}

/// The header of an exported class or form module. Not part of MS-VBAL;
/// it comes from the exported file format.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModuleHeader {
    /// The `VERSION 1.0 CLASS` line.
    pub version: Option<Span>,
    /// The `BEGIN ... END` block.
    pub block: Option<Span>,
    /// `name = value` lines of the outermost `BEGIN` block (for classes
    /// `MultiUse = -1`). Values are the raw text of the line after `=`
    /// without trailing comment.
    pub properties: Vec<HeaderProperty>,
}

/// One `name = value` line of the header block.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderProperty {
    /// Property name.
    pub name: Name,
    /// Raw value text.
    pub value: Vec<u8>,
    /// Span of the line without its terminator.
    pub span: Span,
}

/// A module-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `Option ...`
    Option(OptionDecl),
    /// `Attribute ...`
    Attribute(AttributeDecl),
    /// `DefInt A-Z` etc.
    DefType(DefTypeDecl),
    /// `Implements X`
    Implements(ImplementsDecl),
    /// `Declare ...`
    Declare(DeclareDecl),
    /// `Event ...`
    Event(EventDecl),
    /// `Const ...`
    Const(ConstDecl),
    /// `Dim`/`Private`/`Public`/`Global` variable declaration.
    Var(VarDecl),
    /// `Type ... End Type`
    Type(TypeDecl),
    /// `Enum ... End Enum`
    Enum(EnumDecl),
    /// `Sub`, `Function` or `Property`.
    Proc(ProcDecl),
    /// A conditional-compilation directive.
    Directive(Directive),
    /// Text of an inactive conditional-compilation branch.
    Inactive(Span),
    /// Source that failed to parse.
    Error(Span),
}

impl Item {
    /// The span of the item.
    pub fn span(&self) -> Span {
        match self {
            Item::Option(d) => d.span,
            Item::Attribute(d) => d.span,
            Item::DefType(d) => d.span,
            Item::Implements(d) => d.span,
            Item::Declare(d) => d.span,
            Item::Event(d) => d.span,
            Item::Const(d) => d.span,
            Item::Var(d) => d.span,
            Item::Type(d) => d.span,
            Item::Enum(d) => d.span,
            Item::Proc(d) => d.span,
            Item::Directive(d) => d.span,
            Item::Inactive(s) | Item::Error(s) => *s,
        }
    }
}

impl Module {
    /// The module name from `Attribute VB_Name = "..."`, if present.
    pub fn vb_name(&self) -> Option<Name> {
        self.items.iter().find_map(|i| match i {
            Item::Attribute(a) if a.object.is_none() && a.name.name.eq_ignore_case("VB_Name") => {
                match a.values.first().map(|e| &e.kind) {
                    Some(ExprKind::Literal(Literal::String(s))) => Some(Name::new(s)),
                    _ => None,
                }
            }
            _ => None,
        })
    }

    /// All procedures of the module.
    pub fn procs(&self) -> impl Iterator<Item = &ProcDecl> {
        self.items.iter().filter_map(|i| match i {
            Item::Proc(p) => Some(p),
            _ => None,
        })
    }
}

// ---- directives --------------------------------------------------------------

/// Which conditional-compilation directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectiveKind {
    /// `#If`
    If,
    /// `#ElseIf`
    ElseIf,
    /// `#Else`
    Else,
    /// `#End If`
    EndIf,
    /// `#Const`
    Const,
}

/// A conditional-compilation directive (MS-VBAL §4).
#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    /// The directive.
    pub kind: DirectiveKind,
    /// Span of the directive line without its terminator.
    pub span: Span,
    /// The evaluated condition for `#If`/`#ElseIf` (when it was evaluated).
    pub value: Option<bool>,
}

// ---- option / attribute / def ------------------------------------------------

/// `Option ...` (§5.2.1).
#[derive(Debug, Clone, PartialEq)]
pub struct OptionDecl {
    /// Span.
    pub span: Span,
    /// Which option; `None` for a malformed statement.
    pub kind: Option<OptionKind>,
}

/// The kinds of `Option` statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    /// `Option Explicit`
    Explicit,
    /// `Option Base 0|1`
    Base(u8),
    /// `Option Compare ...`
    Compare(CompareMode),
    /// `Option Private Module`
    PrivateModule,
}

/// String comparison mode of `Option Compare`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareMode {
    /// `Binary`
    Binary,
    /// `Text`
    Text,
    /// `Database`
    Database,
}

/// `Attribute [object.]name = value, ...` (§4.2).
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeDecl {
    /// Span.
    pub span: Span,
    /// The object qualifier of member attributes (`Foo` in
    /// `Attribute Foo.VB_Description`).
    pub object: Option<Ident>,
    /// The attribute name.
    pub name: Ident,
    /// The values.
    pub values: Vec<Expr>,
}

/// The type named by a `Def...` statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefType {
    /// `DefBool`
    Bool,
    /// `DefByte`
    Byte,
    /// `DefCur`
    Cur,
    /// `DefDate`
    Date,
    /// `DefDbl`
    Dbl,
    /// `DefInt`
    Int,
    /// `DefLng`
    Lng,
    /// `DefLngLng`
    LngLng,
    /// `DefLngPtr`
    LngPtr,
    /// `DefObj`
    Obj,
    /// `DefSng`
    Sng,
    /// `DefStr`
    Str,
    /// `DefVar`
    Var,
    /// `DefDec` (reserved for future use by the specification)
    Dec,
}

/// `DefInt A-C, X` (§5.2.1.1).
#[derive(Debug, Clone, PartialEq)]
pub struct DefTypeDecl {
    /// Span.
    pub span: Span,
    /// The type.
    pub ty: DefType,
    /// Inclusive upper-case letter ranges.
    pub ranges: Vec<(u8, u8)>,
}

/// `Implements Interface`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplementsDecl {
    /// Span.
    pub span: Span,
    /// The implemented interface.
    pub interface: TypeRef,
}

// ---- types -------------------------------------------------------------------

/// Built-in types that are reserved words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinType {
    /// `Boolean`
    Boolean,
    /// `Byte`
    Byte,
    /// `Currency`
    Currency,
    /// `Date`
    Date,
    /// `Double`
    Double,
    /// `Integer`
    Integer,
    /// `Long`
    Long,
    /// `LongLong`
    LongLong,
    /// `LongPtr`
    LongPtr,
    /// `Single`
    Single,
    /// `String`
    String,
    /// `Variant`
    Variant,
    /// `Any` (external procedure parameters)
    Any,
}

/// A type reference.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    /// Span.
    pub span: Span,
    /// What it names.
    pub kind: TypeRefKind,
}

/// The target of a [`TypeRef`].
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRefKind {
    /// A reserved type word.
    Builtin(BuiltinType),
    /// A (possibly qualified) class, UDT or enum name: `Excel.Range` is
    /// `[Excel, Range]`. `Object` is a plain named type.
    Named(Vec<Ident>),
    /// Incomplete syntax.
    Missing,
}

/// `As [New] Type [* length]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AsClause {
    /// Span.
    pub span: Span,
    /// Whether `New` was written.
    pub is_new: bool,
    /// The type.
    pub ty: TypeRef,
    /// The length of a fixed-length string (`As String * 10`).
    pub fixed_length: Option<Expr>,
    /// Whether the type is followed by `()`: the array result type of a
    /// function (`Function F() As Long()`).
    pub is_array: bool,
}

// ---- declarations -------------------------------------------------------------

/// Declared visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// `Public`
    Public,
    /// `Private`
    Private,
    /// `Friend`
    Friend,
    /// `Global`
    Global,
}

/// The keyword that started a variable declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKeyword {
    /// `Dim`
    Dim,
    /// `Private`
    Private,
    /// `Public`
    Public,
    /// `Global`
    Global,
    /// `Static`
    Static,
}

/// `Dim`/`Private`/`Public`/`Global`/`Static` variable declaration
/// (§5.2.3.1, §5.4.3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    /// Span.
    pub span: Span,
    /// The leading keyword.
    pub keyword: VarKeyword,
    /// Visibility implied by the keyword (`Private`, `Public`, `Global`).
    pub visibility: Option<Visibility>,
    /// `Static`.
    pub is_static: bool,
    /// `Shared` (legacy).
    pub shared: bool,
    /// `WithEvents`.
    pub with_events: bool,
    /// The declared variables.
    pub declarators: Vec<Declarator>,
}

/// One variable in a declaration: `name[(bounds)] [As ...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Declarator {
    /// Span.
    pub span: Span,
    /// The name (with optional type suffix).
    pub name: Ident,
    /// `None` for a scalar; `Some(empty)` for a dynamic array `a()`.
    pub bounds: Option<Vec<Bound>>,
    /// The `As` clause.
    pub as_clause: Option<AsClause>,
}

/// One array dimension: `[lower To] upper`.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    /// Span.
    pub span: Span,
    /// The lower bound if written.
    pub lower: Option<Expr>,
    /// The upper bound. [`ExprKind::Error`] when missing.
    pub upper: Expr,
}

/// `[vis] Const name [As T] = value, ...` (§5.2.3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    /// Span.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// The constants.
    pub items: Vec<ConstItem>,
}

/// One constant.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstItem {
    /// Span.
    pub span: Span,
    /// Name.
    pub name: Ident,
    /// Declared type.
    pub as_clause: Option<AsClause>,
    /// Value expression.
    pub value: Expr,
}

/// `Type ... End Type` (§5.2.3.3).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    /// Span.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// Name.
    pub name: Ident,
    /// Members.
    pub members: Vec<Declarator>,
    /// Conditional-compilation directives inside the body, in order.
    pub directives: Vec<Directive>,
}

/// `Enum ... End Enum` (§5.2.3.4).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    /// Span.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// Name.
    pub name: Ident,
    /// Members.
    pub members: Vec<EnumMember>,
    /// Conditional-compilation directives inside the body, in order.
    pub directives: Vec<Directive>,
}

/// An enum member.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    /// Span.
    pub span: Span,
    /// Name.
    pub name: Ident,
    /// Explicit value.
    pub value: Option<Expr>,
}

/// `Event name(params)` (§5.2.3.4).
#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    /// Span.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// Name.
    pub name: Ident,
    /// Parameters.
    pub params: Vec<Param>,
}

/// `Sub` or `Function`, for `Declare`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclareKind {
    /// `Declare Sub`
    Sub,
    /// `Declare Function`
    Function,
}

/// `Declare [PtrSafe] Sub|Function name Lib "..." [Alias "..."]` (§5.2.3.5).
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareDecl {
    /// Span.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// `PtrSafe` written.
    pub ptr_safe: bool,
    /// Sub or Function.
    pub kind: DeclareKind,
    /// Name.
    pub name: Ident,
    /// Library string, quotes removed.
    pub lib: Vec<u8>,
    /// Alias string, quotes removed.
    pub alias: Option<Vec<u8>>,
    /// Parameters.
    pub params: Vec<Param>,
    /// Return type.
    pub return_type: Option<AsClause>,
}

/// The kind of a procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcKind {
    /// `Sub`
    Sub,
    /// `Function`
    Function,
    /// `Property Get`
    PropertyGet,
    /// `Property Let`
    PropertyLet,
    /// `Property Set`
    PropertySet,
}

/// How a parameter is passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassBy {
    /// `ByVal`
    ByVal,
    /// `ByRef`
    ByRef,
}

/// A formal parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Span.
    pub span: Span,
    /// `Optional`.
    pub optional: bool,
    /// `ByVal` / `ByRef` if written (`ByRef` is the default otherwise).
    pub pass_by: Option<PassBy>,
    /// `ParamArray`.
    pub param_array: bool,
    /// Name.
    pub name: Ident,
    /// Written with `()`.
    pub is_array: bool,
    /// Declared type.
    pub as_clause: Option<AsClause>,
    /// Default value.
    pub default: Option<Expr>,
}

/// A procedure (§5.3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ProcDecl {
    /// Span from the first modifier to `End ...`.
    pub span: Span,
    /// Visibility.
    pub visibility: Option<Visibility>,
    /// `Static`.
    pub is_static: bool,
    /// Kind.
    pub kind: ProcKind,
    /// Name.
    pub name: Ident,
    /// Parameters.
    pub params: Vec<Param>,
    /// Return type (`Function`, `Property Get`).
    pub return_type: Option<AsClause>,
    /// Body.
    pub body: Block,
}

impl ProcDecl {
    /// Member attributes written at the start of the body
    /// (`Attribute Foo.VB_Description = "..."`).
    pub fn attributes(&self) -> impl Iterator<Item = &AttributeDecl> {
        self.body.stmts.iter().filter_map(|s| match &s.kind {
            StmtKind::Attribute(a) => Some(a),
            _ => None,
        })
    }
}

// ---- statements -----------------------------------------------------------------

/// A statement list.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Span (empty for an empty body).
    pub span: Span,
    /// Statements in order.
    pub stmts: Vec<Stmt>,
}

impl Block {
    /// The labels and line numbers defined directly in this block.
    pub fn labels(&self) -> impl Iterator<Item = &LabelDef> {
        self.stmts.iter().filter_map(|s| match &s.kind {
            StmtKind::Label(l) => Some(l),
            _ => None,
        })
    }
}

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    /// Span.
    pub span: Span,
    /// What it is.
    pub kind: StmtKind,
}

/// A label or line number definition (§5.4.1.1). Line numbers and labels
/// are statements of their own: `10 x = 1` is `Label(10)` followed by the
/// assignment.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelDef {
    /// Span.
    pub span: Span,
    /// The label.
    pub label: LabelRef,
}

/// A reference to a label or line number.
#[derive(Debug, Clone, PartialEq)]
pub enum LabelRef {
    /// `name`
    Name(Ident),
    /// A line number; `None` when it does not fit `u64`.
    LineNumber {
        /// Span.
        span: Span,
        /// Value.
        value: Option<u64>,
    },
}

impl LabelRef {
    /// The span.
    pub fn span(&self) -> Span {
        match self {
            LabelRef::Name(i) => i.span,
            LabelRef::LineNumber { span, .. } => *span,
        }
    }

    /// Whether two references denote the same label (names compare
    /// case-insensitively, line numbers numerically).
    pub fn same_target(&self, other: &LabelRef) -> bool {
        match (self, other) {
            (LabelRef::Name(a), LabelRef::Name(b)) => a.name == b.name,
            (LabelRef::LineNumber { value: a, .. }, LabelRef::LineNumber { value: b, .. }) => {
                a.is_some() && a == b
            }
            _ => false,
        }
    }
}

/// The assignment keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignKind {
    /// `x = v` or `Let x = v`
    Let,
    /// `LSet x = v`
    LSet,
    /// `RSet x = v`
    RSet,
}

/// An `Exit` target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// `Exit Sub`
    Sub,
    /// `Exit Function`
    Function,
    /// `Exit Property`
    Property,
    /// `Exit For`
    For,
    /// `Exit Do`
    Do,
}

/// The action of `On Error`.
#[derive(Debug, Clone, PartialEq)]
pub enum OnErrorAction {
    /// `GoTo label` (a line number other than 0 also lands here)
    GoTo(LabelRef),
    /// `GoTo 0`
    GoToZero,
    /// `GoTo -1`
    GoToMinusOne,
    /// `Resume Next`
    ResumeNext,
}

/// The target of `Resume`.
#[derive(Debug, Clone, PartialEq)]
pub enum ResumeTarget {
    /// Plain `Resume`.
    Same,
    /// `Resume Next`
    Next,
    /// `Resume label`
    Label(LabelRef),
}

/// `While` or `Until`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCondKind {
    /// `While`
    While,
    /// `Until`
    Until,
}

/// A loop condition.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopCond {
    /// `While` or `Until`.
    pub kind: LoopCondKind,
    /// The condition.
    pub expr: Expr,
}

/// A `Case` item (§5.4.2.10).
#[derive(Debug, Clone, PartialEq)]
pub enum CaseItem {
    /// `Case expr`
    Value(Expr),
    /// `Case a To b`
    Range {
        /// Lower end.
        lower: Expr,
        /// Upper end.
        upper: Expr,
    },
    /// `Case Is <op> expr`
    Is {
        /// The comparison operator.
        op: BinaryOp,
        /// The operand.
        value: Expr,
    },
}

/// A `Case` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseClause {
    /// Span.
    pub span: Span,
    /// Empty for `Case Else`.
    pub items: Vec<CaseItem>,
    /// Whether this is `Case Else`.
    pub is_else: bool,
    /// Body.
    pub body: Block,
}

/// An `ElseIf` clause.
#[derive(Debug, Clone, PartialEq)]
pub struct ElseIf {
    /// Span.
    pub span: Span,
    /// Condition.
    pub cond: Expr,
    /// Body.
    pub body: Block,
}

/// `If` in block or single-line form (§5.4.2.8).
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    /// Condition.
    pub cond: Expr,
    /// `Then` body.
    pub then_body: Block,
    /// `ElseIf` clauses (block form only).
    pub else_ifs: Vec<ElseIf>,
    /// `Else` body.
    pub else_body: Option<Block>,
    /// Single-line form.
    pub single_line: bool,
}

/// `Select Case` (§5.4.2.10).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStmt {
    /// The tested expression.
    pub subject: Expr,
    /// Clauses in order.
    pub cases: Vec<CaseClause>,
}

/// `For counter = start To end [Step step]` (§5.4.2.3).
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    /// Loop variable.
    pub counter: Expr,
    /// Start value.
    pub start: Expr,
    /// End value.
    pub end: Expr,
    /// Step.
    pub step: Option<Expr>,
    /// Body.
    pub body: Block,
    /// Variables named after `Next` (several when `Next j, i` closes
    /// nested loops; the extra ones close enclosing loops).
    pub next_counters: Vec<Expr>,
}

/// `For Each item In collection` (§5.4.2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachStmt {
    /// Loop variable.
    pub item: Expr,
    /// The collection or array.
    pub collection: Expr,
    /// Body.
    pub body: Block,
    /// Variables named after `Next`.
    pub next_counters: Vec<Expr>,
}

/// `Do ... Loop` (§5.4.2.5-6).
#[derive(Debug, Clone, PartialEq)]
pub struct DoStmt {
    /// Condition after `Do`.
    pub pre: Option<LoopCond>,
    /// Condition after `Loop`.
    pub post: Option<LoopCond>,
    /// Body.
    pub body: Block,
}

/// File mode of `Open`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// `Input`
    Input,
    /// `Output`
    Output,
    /// `Append`
    Append,
    /// `Binary`
    Binary,
    /// `Random` (also the default when `For` is omitted)
    Random,
}

/// `Access` clause of `Open`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAccess {
    /// `Read`
    Read,
    /// `Write`
    Write,
    /// `Read Write`
    ReadWrite,
}

/// Lock clause of `Open`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenLock {
    /// `Shared`
    Shared,
    /// `Lock Read`
    LockRead,
    /// `Lock Write`
    LockWrite,
    /// `Lock Read Write`
    LockReadWrite,
}

/// `Open path [For mode] [Access a] [lock] As #n [Len = n]` (§5.4.5.1).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenStmt {
    /// File name expression.
    pub path: Expr,
    /// Mode; `None` when `For` was omitted.
    pub mode: Option<OpenMode>,
    /// Access.
    pub access: Option<OpenAccess>,
    /// Lock.
    pub lock: Option<OpenLock>,
    /// File number (without the `#`).
    pub file: Expr,
    /// Record length.
    pub record_len: Option<Expr>,
}

/// An item of a `Print`/`Write`/`Debug.Print` list.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputItem {
    /// An expression (including `Spc(n)` and `Tab(n)` calls).
    Expr(Expr),
    /// `;`
    Semicolon,
    /// `,`
    Comma,
}

/// A `ReDim` target.
#[derive(Debug, Clone, PartialEq)]
pub struct ReDimItem {
    /// Span.
    pub span: Span,
    /// The array being resized (a name or member access).
    pub target: Expr,
    /// The new bounds.
    pub bounds: Vec<Bound>,
    /// `As` clause.
    pub as_clause: Option<AsClause>,
}

/// An executable statement kind (§5.4).
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// A label or line number definition.
    Label(LabelDef),
    /// Local `Dim`/`Static` declaration.
    Dim(VarDecl),
    /// Local `Const`.
    Const(ConstDecl),
    /// `ReDim [Preserve] ...`
    ReDim {
        /// `Preserve`.
        preserve: bool,
        /// Targets.
        items: Vec<ReDimItem>,
    },
    /// `Erase a, b`
    Erase(Vec<Expr>),
    /// `[Let] target = value`, `LSet`, `RSet`.
    Assign {
        /// Which keyword.
        kind: AssignKind,
        /// Assigned location.
        target: Expr,
        /// Value.
        value: Expr,
    },
    /// `Set target = value`
    Set {
        /// Assigned location.
        target: Expr,
        /// Value.
        value: Expr,
    },
    /// `Call f(args)` or the implicit `f args` / `f(args)` statement.
    Call {
        /// The procedure expression (name or member access).
        callee: Expr,
        /// Arguments.
        args: Vec<Arg>,
        /// Written with the `Call` keyword.
        explicit: bool,
        /// The arguments were enclosed in parentheses.
        parenthesized: bool,
    },
    /// `If`
    If(Box<IfStmt>),
    /// `Select Case`
    Select(Box<SelectStmt>),
    /// `For`
    For(Box<ForStmt>),
    /// `For Each`
    ForEach(Box<ForEachStmt>),
    /// `Do`/`Loop`
    Do(Box<DoStmt>),
    /// `While ... Wend`
    While {
        /// Condition.
        cond: Expr,
        /// Body.
        body: Block,
    },
    /// `With`
    With {
        /// The object expression.
        object: Expr,
        /// Body.
        body: Block,
    },
    /// `GoTo`
    GoTo(LabelRef),
    /// `GoSub`
    GoSub(LabelRef),
    /// `Return`
    Return,
    /// `On Error ...`
    OnError(OnErrorAction),
    /// `On expr GoTo|GoSub l1, l2`
    OnGoTo {
        /// Selector.
        selector: Expr,
        /// `GoSub` rather than `GoTo`.
        is_gosub: bool,
        /// Targets.
        targets: Vec<LabelRef>,
    },
    /// `Resume`
    Resume(ResumeTarget),
    /// `Exit ...`
    Exit(ExitKind),
    /// `End`
    End,
    /// `Stop`
    Stop,
    /// `Open`
    Open(Box<OpenStmt>),
    /// `Close [files]`; empty closes all.
    Close(Vec<Expr>),
    /// `Print #file, items`
    Print {
        /// File number.
        file: Expr,
        /// Items.
        items: Vec<OutputItem>,
    },
    /// `Write #file, items`
    Write {
        /// File number.
        file: Expr,
        /// Items.
        items: Vec<OutputItem>,
    },
    /// `Input #file, targets`
    Input {
        /// File number.
        file: Expr,
        /// Variables.
        targets: Vec<Expr>,
    },
    /// `Line Input #file, target`
    LineInput {
        /// File number.
        file: Expr,
        /// Variable.
        target: Expr,
    },
    /// `Get #file, [record], [target]`
    Get {
        /// File number.
        file: Expr,
        /// Record number.
        record: Option<Expr>,
        /// Variable.
        target: Option<Expr>,
    },
    /// `Put #file, [record], [value]`
    Put {
        /// File number.
        file: Expr,
        /// Record number.
        record: Option<Expr>,
        /// Value.
        value: Option<Expr>,
    },
    /// `Seek #file, position`
    Seek {
        /// File number.
        file: Expr,
        /// Position.
        position: Option<Expr>,
    },
    /// `Lock #file [, range]`
    Lock {
        /// File number.
        file: Expr,
        /// First record or range start.
        from: Option<Expr>,
        /// Range end.
        to: Option<Expr>,
    },
    /// `Unlock #file [, range]`
    Unlock {
        /// File number.
        file: Expr,
        /// First record or range start.
        from: Option<Expr>,
        /// Range end.
        to: Option<Expr>,
    },
    /// `Width #file, width`
    Width {
        /// File number.
        file: Expr,
        /// Width.
        width: Option<Expr>,
    },
    /// `Name old As new`
    Name {
        /// Old path.
        old: Expr,
        /// New path.
        new: Expr,
    },
    /// `RaiseEvent name(args)`
    RaiseEvent {
        /// Event name.
        name: Ident,
        /// Arguments.
        args: Vec<Arg>,
    },
    /// `Debug.Print items`
    DebugPrint(Vec<OutputItem>),
    /// `Attribute ...` inside a procedure.
    Attribute(AttributeDecl),
    /// A conditional-compilation directive between statements.
    Directive(Directive),
    /// Text of an inactive conditional-compilation branch.
    Inactive,
    /// Source that failed to parse.
    Error,
}

// ---- expressions ----------------------------------------------------------------

/// An expression with its span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Span.
    pub span: Span,
    /// What it is.
    pub kind: ExprKind,
}

/// Radix of an integer literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    /// Decimal.
    Decimal,
    /// `&H`
    Hex,
    /// `&O` or `&`
    Octal,
}

/// A literal (§3.3, §5.6.5).
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// An integer literal. `value` is the unsigned magnitude as written
    /// (`None` when it exceeds `u64`); the type-selection rules of
    /// §3.3.2 are static semantics and not applied here.
    Integer {
        /// Magnitude.
        value: Option<u64>,
        /// Radix.
        radix: Radix,
        /// Suffix.
        suffix: Option<TypeSuffix>,
    },
    /// A floating point literal.
    Float {
        /// Value (`D` exponents handled).
        value: f64,
        /// The literal as written without its suffix, with a `D` exponent
        /// marker spelled `E`; kept so a `Currency` literal can be read
        /// exactly instead of through the `f64`.
        text: String,
        /// Suffix.
        suffix: Option<TypeSuffix>,
    },
    /// A string with the quotes removed and `""` collapsed.
    String(Vec<u8>),
    /// A date literal's text between the `#`s.
    Date(Vec<u8>),
    /// `True`
    True,
    /// `False`
    False,
    /// `Nothing`
    Nothing,
    /// `Empty`
    Empty,
    /// `Null`
    Null,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `Not`
    Not,
}

/// Binary operators (§5.6.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `\`
    IntDiv,
    /// `Mod`
    Mod,
    /// `^`
    Pow,
    /// `&`
    Concat,
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `Like`
    Like,
    /// `Is`
    Is,
    /// `And`
    And,
    /// `Or`
    Or,
    /// `Xor`
    Xor,
    /// `Eqv`
    Eqv,
    /// `Imp`
    Imp,
}

/// An argument of a call or index expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// Span (empty for a skipped argument).
    pub span: Span,
    /// The parameter name of `name := value`.
    pub name: Option<Ident>,
    /// The value; `None` for a skipped argument (`f(, 2)`).
    pub value: Option<Expr>,
}

/// An expression kind.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A literal.
    Literal(Literal),
    /// A simple name.
    Name(Ident),
    /// `( expr )`: kept because it forces by-value passing.
    Paren(Box<Expr>),
    /// Unary operation.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        operand: Box<Expr>,
    },
    /// Binary operation.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        lhs: Box<Expr>,
        /// Right operand.
        rhs: Box<Expr>,
    },
    /// `target.name`; `target` is `None` for `.name` inside `With`.
    Member {
        /// Object expression.
        target: Option<Box<Expr>>,
        /// Member name.
        name: Ident,
    },
    /// `target!name` dictionary access; `target` is `None` inside `With`.
    Dict {
        /// Object expression.
        target: Option<Box<Expr>>,
        /// Key name.
        name: Ident,
    },
    /// A procedure call or an index expression; which one is decided by
    /// name resolution, not by syntax.
    Call {
        /// The called or indexed expression.
        callee: Box<Expr>,
        /// Arguments.
        args: Vec<Arg>,
    },
    /// `New Type`
    New(TypeRef),
    /// `AddressOf proc`
    AddressOf(Box<Expr>),
    /// `TypeOf expr Is Type`
    TypeOfIs {
        /// Tested expression.
        expr: Box<Expr>,
        /// Type.
        ty: TypeRef,
    },
    /// `#n` in an argument list.
    FileNumber(Box<Expr>),
    /// `lower To upper` in an index or lock list.
    Range {
        /// Lower end.
        lower: Box<Expr>,
        /// Upper end.
        upper: Box<Expr>,
    },
    /// Missing or malformed expression.
    Error,
}
