/// The kind of an interior node of the concrete syntax tree.
///
/// Leaves are lexer tokens ([`vba_lexer::TokenKind`]). The shape of each
/// node (its child nodes and tokens, in order) is documented per variant.
/// Trivia and the `Newline`/`Colon` separators never belong to a statement
/// node: they are children of the enclosing [`NodeKind::Block`] or
/// [`NodeKind::Module`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    // ---- module structure (MS-VBAL §4) -----------------------------------
    /// The root. Children: optional `VersionLine`, optional `HeaderBlock`,
    /// then items (declarations, procedures, directives, inactive text,
    /// error nodes) separated by `Newline`/`Colon` tokens.
    Module,
    /// `VERSION 1.0 CLASS` line of an exported module (not in MS-VBAL;
    /// from the exported file format). Flat tokens.
    VersionLine,
    /// `BEGIN ... END` property block of an exported class or form. Flat
    /// tokens, nesting balanced; contents are not parsed.
    HeaderBlock,
    /// Source text of an inactive `#If` branch. Flat tokens, never
    /// parsed; lexed losslessly.
    InactiveText,
    /// `#If cond Then`. Flat tokens; the tree stores the evaluated value.
    CcIf,
    /// `#ElseIf cond Then`. Flat tokens; value stored when evaluated.
    CcElseIf,
    /// `#Else`.
    CcElse,
    /// `#End If`.
    CcEndIf,
    /// `#Const name = expr`. Flat tokens.
    CcConst,
    /// Source that could not be parsed. Flat tokens up to the end of the
    /// logical line (or a single token); a diagnostic accompanies it.
    Error,

    // ---- declarations ---------------------------------------------
    /// `Option Explicit | Base n | Compare x | Private Module`.
    OptionStmt,
    /// `Attribute [obj.]name = expr {, expr}`.
    AttributeStmt,
    /// `DefInt A-C, X`. Children: keyword, `DefRange`s, commas.
    DefTypeStmt,
    /// One letter or letter range of a `DefTypeStmt`.
    DefRange,
    /// `Implements TypeRef`.
    ImplementsStmt,
    /// `[vis] Declare [PtrSafe] Sub|Function name Lib "x" [Alias "y"]
    /// [ParamList] [AsClause]`.
    DeclareStmt,
    /// `[vis] Event name [ParamList]`.
    EventStmt,
    /// `[vis] Const ConstItem {, ConstItem}`.
    ConstStmt,
    /// `name [AsClause] = expr`.
    ConstItem,
    /// `Dim|Private|Public|Global|Static [Shared] [WithEvents]
    /// VarDeclarator {, VarDeclarator}`.
    VarDeclStmt,
    /// `name [ArgList-bounds] [AsClause]`.
    VarDeclarator,
    /// `[vis] Type name <members> End Type`.
    TypeStmt,
    /// `name [ArgList-bounds] AsClause` inside a `TypeStmt`.
    TypeMember,
    /// `[vis] Enum name <members> End Enum`.
    EnumStmt,
    /// `name [= expr]` inside an `EnumStmt`.
    EnumMember,
    /// A procedure: modifiers, `Sub|Function|Property Get|Let|Set`, name,
    /// optional `ParamList`, optional `AsClause`, `Block`, `End ...`
    ProcDecl,
    /// `( Param {, Param} )`.
    ParamList,
    /// `[Optional] [ByVal|ByRef] [ParamArray] name [()] [AsClause] [= expr]`.
    Param,
    /// `As [New] TypeRef [* expr]`.
    AsClause,
    /// A possibly qualified type name or a built-in type keyword.
    TypeRef,

    // ---- statements -----------------------------------------------
    /// A statement list. Children: statements, `Newline`, `Colon`, trivia,
    /// directives, inactive text.
    Block,
    /// `name:` or `10` or `10:`.
    LabelStmt,
    /// `[Let|LSet|RSet] lhs = expr`.
    AssignStmt,
    /// `Set lhs = expr`.
    SetStmt,
    /// `Call expr` or an implicit call `callee [ArgList]`.
    CallStmt,
    /// Block `If`: `If expr Then Block {ElseIfClause}
    /// [ElseClause] End If`.
    IfStmt,
    /// Single-line `If expr Then Block [Else Block]`.
    IfLineStmt,
    /// `ElseIf expr Then Block`.
    ElseIfClause,
    /// `Else Block`.
    ElseClause,
    /// `Select Case expr {CaseClause} End Select`.
    SelectStmt,
    /// `Case items Block` or `Case Else Block`.
    CaseClause,
    /// `Is <relop> expr` in a case clause.
    CaseIsItem,
    /// `For c = a To b [Step s] Block Next [c]`.
    ForStmt,
    /// `For Each v In e Block Next [v]`.
    ForEachStmt,
    /// `Do [While|Until e] Block Loop [While|Until e]`.
    DoStmt,
    /// `While e Block Wend`.
    WhileStmt,
    /// `With e Block End With`.
    WithStmt,
    /// `GoTo label` or the bare line number after `Then`/`Else`.
    GoToStmt,
    /// `GoSub label`.
    GoSubStmt,
    /// `Return`.
    ReturnStmt,
    /// `On Error GoTo label|0|-1` or `On Error Resume Next`.
    OnErrorStmt,
    /// `On expr GoTo|GoSub l1, l2`.
    OnGoToStmt,
    /// `Resume [Next | label]`.
    ResumeStmt,
    /// `Exit Sub|Function|Property|For|Do`.
    ExitStmt,
    /// `End`.
    EndStmt,
    /// `Stop`.
    StopStmt,
    /// `ReDim [Preserve] ReDimItem {, ReDimItem}`.
    ReDimStmt,
    /// `target(bounds) [AsClause]`.
    ReDimItem,
    /// `Erase expr {, expr}`.
    EraseStmt,
    /// `Open path [For mode] [Access ..] [lock] As #n [Len = n]`.
    OpenStmt,
    /// `Close [args]`.
    CloseStmt,
    /// `Print #n, OutputList`.
    PrintStmt,
    /// `Write #n, OutputList`.
    WriteStmt,
    /// `Input #n, vars`.
    InputStmt,
    /// `Line Input #n, var`.
    LineInputStmt,
    /// `Get #n, [rec], var`.
    GetStmt,
    /// `Put #n, [rec], expr`.
    PutStmt,
    /// `Seek #n, pos`.
    SeekStmt,
    /// `Lock #n [, range]`.
    LockStmt,
    /// `Unlock #n [, range]`.
    UnlockStmt,
    /// `Width #n, w`.
    WidthStmt,
    /// `Name old As new`.
    NameStmt,
    /// `RaiseEvent name [ArgList]`.
    RaiseEventStmt,
    /// `Debug.Print OutputList`.
    DebugPrintStmt,
    /// Print-style items: expressions separated by `;` and `,`.
    OutputList,

    // ---- expressions ------------------------------------------------
    /// `( Arg {, Arg} )` or, for implicit calls and file statements, the
    /// same without parentheses.
    ArgList,
    /// One argument slot: `[name :=] [expr]`; empty for a skipped argument.
    Arg,
    /// A literal token (number, string, date, `True`, `Nothing`, ...).
    LiteralExpr,
    /// A simple name token (identifier, foreign name, or a keyword that is
    /// also a function such as `Len` or `Date`).
    NameExpr,
    /// `( expr )`.
    ParenExpr,
    /// `op expr` with op in `+ - Not`.
    UnaryExpr,
    /// `expr op expr`.
    BinaryExpr,
    /// `[expr] . name`; without the leading expression it is a `With`
    /// member.
    MemberExpr,
    /// `[expr] ! name` dictionary access.
    DictExpr,
    /// `expr ArgList`: a procedure call or an index expression; the syntax
    /// cannot tell them apart.
    CallExpr,
    /// `New TypeRef`.
    NewExpr,
    /// `AddressOf expr`.
    AddressOfExpr,
    /// `TypeOf expr Is TypeRef`.
    TypeOfIsExpr,
    /// `# expr` file number.
    FileNumberExpr,
    /// `expr To expr` (array bounds, case ranges, lock ranges).
    RangeExpr,
    /// A missing or malformed expression (zero or one token).
    ErrorExpr,
}
