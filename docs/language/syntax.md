# Syntax tree, parser and AST (`vba-syntax`, `vba-ast`)

`vba-syntax` turns `vba-lexer` tokens into a lossless concrete syntax tree
(CST) with a hand-written parser. `vba-ast` lowers that tree into a typed AST
for the semantic analyzer. Sources: MS-VBAL section 4 (module structure,
conditional compilation) and section 5 (declarations, statements,
expressions). Section numbers in the code are given at that granularity only;
finer numbers were not checked against the specification text.

## Interface

```rust
let parse = vba_syntax::parse(file_id, bytes, &profile, &ParseOptions::tolerant());
// parse.tree: SyntaxTree (owns the bytes), parse.diagnostics: Vec<Diagnostic>
let parsed = vba_ast::parse_module(file_id, bytes, &profile, &ParseOptions::strict());
// parsed.module: vba_ast::Module, parsed.tree, parsed.diagnostics (sorted)
```

## CST (`vba-syntax`)

- **Lossless.** Every byte belongs to exactly one token; trivia (whitespace,
  comments, line continuations) and `Newline`/`Colon` separators are tokens
  in the tree. `tree.text() == source` for every input, including erroneous
  and non-UTF-8 input (property-tested over random bytes, token soups,
  mutated and truncated fixtures, all profiles, both modes).
- **Own arena, not rowan.** rowan stores token text as `str`; VBA source is
  bytes in a Windows code page. The tree is a small arena (`SyntaxTree`) that
  owns the source and stores spans. Handles `SyntaxNode`/`SyntaxToken` are
  `Copy` references.
- **Trivia placement.** Trivia is emitted lazily into the enclosing node
  before the next token, never at the start or end of a node, so node spans
  start and end on real tokens. Statements never contain their terminating
  `Newline`/`Colon`; those belong to the enclosing `Block` or `Module`.
- **Node kinds** (`NodeKind`, shape documented per variant): module level
  `Module`, `VersionLine`, `HeaderBlock`, `OptionStmt`, `AttributeStmt`,
  `DefTypeStmt`, `ImplementsStmt`, `DeclareStmt`, `EventStmt`, `ConstStmt`,
  `VarDeclStmt`, `TypeStmt`, `EnumStmt`, `ProcDecl`, `ParamList`, `Param`,
  `AsClause`, `TypeRef`; statements `Block`, `LabelStmt`, `AssignStmt`,
  `SetStmt`, `CallStmt`, `IfStmt`, `IfLineStmt`, `SelectStmt`, `ForStmt`,
  `ForEachStmt`, `DoStmt`, `WhileStmt`, `WithStmt`, `GoToStmt`, `GoSubStmt`,
  `ReturnStmt`, `OnErrorStmt`, `OnGoToStmt`, `ResumeStmt`, `ExitStmt`,
  `EndStmt`, `StopStmt`, `ReDimStmt`, `EraseStmt`, `OpenStmt`, `CloseStmt`,
  `PrintStmt`, `WriteStmt`, `InputStmt`, `LineInputStmt`, `GetStmt`,
  `PutStmt`, `SeekStmt`, `LockStmt`, `UnlockStmt`, `WidthStmt`, `NameStmt`,
  `RaiseEventStmt`, `DebugPrintStmt`; expressions `LiteralExpr`, `NameExpr`,
  `ParenExpr`, `UnaryExpr`, `BinaryExpr`, `MemberExpr`, `DictExpr`,
  `CallExpr`, `NewExpr`, `AddressOfExpr`, `TypeOfIsExpr`, `FileNumberExpr`,
  `RangeExpr`, `ArgList`, `Arg`; plus `Error`, `ErrorExpr` and the
  conditional-compilation nodes below.
- **Precedence** (`prec.rs`, tightest last): `Imp` < `Eqv` < `Xor` < `Or` <
  `And` < `Not` < comparison (`= <> < > <= >= Like Is`) < `&` < `+ -` <
  `Mod` < `\` < `* /` < unary `- +` < `^`. All binary operators are left
  associative (`2^3^2` = 64). `-2^2` is `-(2^2)`, `2^-2` parses, `Not a = b`
  is `Not (a = b)`, `x = y = z` is `x = (y = z)` as assignment of a
  comparison.
- **Call versus index.** `a(1)` is one `CallExpr`; only name resolution can
  tell a call from an index. A `(` preceded by whitespace in statement
  position (`Foo (a), b`) starts the first argument of an implicit call, which
  matters because the parenthesised argument is passed by value.
- **Statements that depend on context** are decided by lookahead: block `If`
  versus single-line `If` (is anything but a comment after `Then`?), label
  versus statement (identifier plus `:` at the start of a physical line, or a
  line number), `Name old As new` versus a call or assignment to something
  called `Name`, `Line Input`, `Width #`, `Debug.Print`, `Print #n,`.
  `Else If` on one line opens a nested block `If` that needs its own `End If`;
  `ElseIf` is a clause. `Next j, i` closes both loops (the `For` that lists
  several counters records the extra closes for the enclosing loops).
  Contextual words that the lexer does not reserve (`Property`, `Step`,
  `Lib`, `Alias`, `Explicit`, `Base`, `Compare`, `Text`, `Binary`, `Error`,
  `Access`, `Module`, ...) are matched by spelling where the grammar expects
  them.
- **Exported modules.** A leading `VERSION 1.0 CLASS` line and a balanced
  `BEGIN ... END` block (class `MultiUse`, form designer properties) are kept
  as `VersionLine` / `HeaderBlock` flat tokens; their contents are not parsed
  and lexer diagnostics inside them (`{GUID}`, `"x.frx":0000`) are dropped.
  This header is part of the exported file format, not MS-VBAL.

### Conditional compilation (section 4)

A pre-pass (`cc.rs`) over all tokens finds `#If`, `#ElseIf`, `#Else`,
`#End If` and `#Const` at the start of a physical line, evaluates conditions
and records which token ranges are inactive.

- Conditions use the profile's predefined constants (`VBA6`, `VBA7`, `Win16`,
  `Win32`, `Win64`, `Mac` via `vba-profile`), project constants passed in
  `ParseOptions::constants`, and `#Const` definitions seen in active code.
  Undefined names are Empty (false). Supported: integer, float, hex/octal and
  string literals, `True`/`False`/`Empty`/`Null`, parentheses, unary `- + Not`,
  `+ - * / \ Mod ^ &`, comparisons, `And Or Xor Eqv Imp`. Function calls and
  other constructs are reported as `SYN042`.
- Directive lines become flat nodes `CcIf`, `CcElseIf`, `CcElse`, `CcEndIf`,
  `CcConst` (siblings of the surrounding items, not a nested tree), so a
  branch may contain just a procedure header or half of a `Type`. The
  evaluated value of `#If`/`#ElseIf` is stored on the node (`value()`); a
  `#ElseIf` after a taken branch is not evaluated.
- Inactive branches are kept as one `InactiveText` node of flat tokens. They
  are lexed (so the tree stays lossless) but not parsed; lexer diagnostics
  inside them are dropped. Directives nested inside inactive text are part of
  that text, but their nesting is tracked so `#End If` pairs correctly, and a
  `#Const` in inactive text defines nothing.
- Errors: `#Else`/`#ElseIf`/`#End If` without `#If` (`SYN040`), missing
  `#End If` (`SYN041`), bad expression or missing `Then` (`SYN042`).

### Version gating

The lexer reserves `PtrSafe`, `LongPtr`, `LongLong` (VBA 7), and
`Implements`, `Event`, `RaiseEvent`, `WithEvents`, `AddressOf` (not in the
Access 95 profile) per generation. On top of that the parser reports
(`SYN010`): `PtrSafe` before VBA 7; `LongPtr` before VBA 7; `LongLong` unless
the profile supports it (VBA 7 and 64-bit target); `Declare` without
`PtrSafe` on a 64-bit VBA 7 target. In generations that lack a keyword the
word is an ordinary identifier, so for example `Event Foo()` in the Access 95
profile fails as an ordinary statement.

### Diagnostics and recovery

Codes: `SYN001` unexpected token, `SYN002` expected X, `SYN003` missing
closer, `SYN004` misplaced construct, `SYN005` tolerated deviation,
`SYN010` version gate, `SYN040`-`SYN042` conditional compilation. Lexer
diagnostics (`LEX...`) are included.

- **Statement-level resync.** On an error the rest of the logical line (up to
  `Newline`, `:` or, in a single-line `If`, `Else`) becomes an `Error` node
  and parsing continues with the next statement. Only one diagnostic is
  reported per bad line. A failed expression becomes an empty `ErrorExpr`.
- **Block structure.** Every open construct declares the terminators it
  accepts. A terminator that belongs to an outer construct (for example
  `End Sub` while an `If` is open) ends the inner block with a `SYN003`
  "missing `End If`" diagnostic; a terminator nobody accepts is an `Error`
  node. A procedure header at the start of a line closes everything that is
  open, so a missing `End Sub` does not swallow the next procedure.
- **Progress guarantee.** Every loop either consumes a token or is forced to
  wrap one token in an `Error` node.
- **Nesting limit.** Blocks, statements and expressions nest at most 200
  deep (`MAX_DEPTH`); deeper input is reported (`SYN004`) and the rest of the
  line skipped instead of overflowing the stack.
- **Strict versus tolerant.** Both modes build the same tree.
  `ParseOptions::strict()` turns lexer strictness on (non-ASCII identifier
  bytes) and makes tolerated deviations errors instead of warnings:
  statements on the same line as a block `Else` (also `Else If`), and
  indented labels. `Parse::into_result()` fails on any error-severity
  diagnostic.

## AST (`vba-ast`)

- `Module { header, items }` with `Item` variants for options, attributes,
  `Def...`, `Implements`, `Declare`, `Event`, `Const`, variables, `Type`,
  `Enum`, procedures, conditional-compilation directives, inactive text and
  errors. Procedure bodies are `Block`s of `Stmt { span, kind: StmtKind }`;
  local `Dim`/`Static`/`Const` reuse the module-level declaration structs.
  Expressions are `Expr { span, kind: ExprKind }`.
- `Name` compares and hashes with ASCII case folding and keeps the original
  spelling (`to_string_lossy`, `as_bytes`). Non-ASCII bytes compare exactly
  because folding them needs the code page. `Ident` adds the type suffix
  (`s$` is name `s`, suffix String), whether it was a `[foreign name]`, and
  the keyword when a reserved word is used as a name (`Len`, `Date`).
- Line numbers and labels are `StmtKind::Label` statements (`10 x = 1` is a
  label followed by an assignment); `GoTo`, `GoSub`, `On Error GoTo`,
  `On ... GoTo` and `Resume` carry `LabelRef` (`Name` or `LineNumber`);
  `LabelRef::same_target` compares them, `Block::labels()` lists them.
- Call statements keep whether `Call` was written and whether the arguments
  were parenthesised; `Foo (a)` has a `Paren` argument, `Foo(a)` is a
  parenthesised call. Index and call expressions are both `ExprKind::Call`.
- Literals are decoded (`""` collapsed, hex/octal values, `D` exponents) but
  the type-selection rules for integer literals (MS-VBAL 3.3.2) are static
  semantics and are not applied.
- `lower(&tree, file)` accepts any CST, erroneous ones included. Every CST
  `Error` node and `ErrorExpr` yields `AST001`; mandatory parts that are
  absent are replaced by placeholders and reported as `AST002`;
  unconvertible literals `AST003`. `parse_module` returns parser and
  lowering diagnostics together.

## Tests

Table-driven valid and invalid snippets per production
(`tests/productions.rs`), precedence/associativity and ambiguity tests
(`tests/expressions.rs`), conditional compilation, version gating and
strict/tolerant (`tests/conditional.rs`), recovery (`tests/recovery.rs`),
`expect-test` snapshots (`tests/snapshots.rs`; refresh with
`UPDATE_EXPECT=1`, and `CARGO_WORKSPACE_DIR=$PWD` when the checkout sits
inside another cargo workspace), property tests (`tests/properties.rs` in
both crates), and four realistic fixture modules (standard module, class with
exported header, form code-behind, API declares with `PtrSafe`/`LongPtr`)
that must parse with zero diagnostics in tolerant and strict mode, with LF and
CRLF, on three profiles (`tests/corpus.rs`).

## Not implemented / known gaps

- Compatibility with the real VBE parser is unmeasured: the tests check the
  grammar as understood from MS-VBAL and the fixtures are hand-written, not
  taken from real projects. Section numbers are not cited below section
  level.
- Conditional compilation: only line-start directives; a directive inside a
  parameter list or an expression continued over lines is not supported. The
  directive condition is stored as flat tokens, not an expression tree.
  Function calls in conditions are rejected.
- No semantic checks: duplicate declarations, `Option Explicit`, order of
  `Option`/declarations/procedures, `Exit Function` in a `Sub`, label
  existence, constant-expression validity, literal type ranges.
- Statements: `?` as `Print` shorthand, `Foo (1) = 2` (assignment to a
  spaced-paren call), `Declare ... CDecl`, `Date$`/`Time$` as assignment
  targets beyond what the lexer produces, and block statements inside a
  single-line `If` are not handled specially. `Open ... For` modifiers are
  taken as free words and interpreted only in the AST. `Print`/`Write` require
  `#`. Statement keywords not in the lexer's table (`Kill`, `MkDir`,
  `Randomize`, ...) are ordinary implicit calls.
- Header: the content of the `BEGIN ... END` block is not parsed; the AST
  extracts only top-level `name = value` lines. Form designer binary data
  (`.frx`) is out of scope.
- Non-Latin identifier forms and non-ASCII whitespace follow the lexer's
  limits (`docs/language/lexer.md`).
- `Else` followed by a statement on the same line of a block `If` and
  indented labels are accepted with a warning because VBE behaviour was not
  verified; they may be accepted or rejected differently by the real editor.
