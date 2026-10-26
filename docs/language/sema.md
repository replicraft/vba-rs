# Semantic analysis (`vba-sema`)

`vba-sema` turns the parsed modules of a project into a bound, typed program
plus deterministic diagnostics. It is the layer between `vba-ast` and
`vba-ir`. Sources: MS-VBAL section 5 (declarations, statements, expressions)
and section 2 (naming and scoping); section numbers are given at that
granularity only, finer numbers were not checked against the specification
text. Rules that come from documented VBA behaviour instead of the
specification are marked **doc**, rules we chose without a source are marked
**unverified**. Nothing here has been compared with a real VBA host.

## Interface

```rust
let analysis = vba_sema::analyze_sources(&modules, &profile, &provider, &Options::default());
// or: vba_sema::analyze_project(&vba_project, &profile, &provider)
analysis.program      // Program: modules, procs (bound bodies), globals, consts, UDTs, enums, declares, events
analysis.diagnostics  // parser + AST + SEM### diagnostics, sorted
analysis.sources      // SourceMap for vba_diagnostics::render_plain
```

Inputs are `SourceModule { name, kind, source }` (standard, class, document,
designer) or a `vba_project::VbaProject`. Modules are parsed with
`vba_ast::parse_module` (strict by default, project conditional-compilation
constants are passed through). External libraries are injected with the
`TypeProvider` trait; a built-in `VBA` library (constants such as `vbCrLf`,
`Err`, `Debug`, `Collection`, `CreateObject`, `MsgBox`, ...) is consulted after
the provider, and the `vba-stdlib` registry resolves intrinsic functions
(`Left`, `Left$`, `VBA.Left`, ...).

`TypeProvider` has a "does not know" default for every method:
`resolve_type`, `member`, `default_member`, `describes_members`,
`interface_members`, `events`, `global`. `StaticLibrary` is a table-driven
implementation for tests and small hosts.

## The model

- **Symbols.** Each module has one case-insensitive symbol table (`Name`
  compares ASCII case-insensitively and keeps its spelling): variables,
  constants, procedures, property groups (`Get`/`Let`/`Set` merged under one
  name), `Declare`s, enums and their members, UDTs, events. A duplicate in the
  same scope is `SEM001`.
- **Types** (`Type`): intrinsic scalars, `FixedString(n)`, `Variant`,
  `Object`, `Class(module)`, `Udt`, `Enum`, `Library(lib, name)`, arrays with
  optional rank, `Nothing`, `Any`, and `Error` (poison after a diagnostic).
  `LongPtr` resolves to `Long`/`LongLong` from the profile's bitness;
  `LongLong` needs a 64-bit VBA7 profile.
- **Bound bodies.** `BStmt`/`BExpr`/`BPlace` trees: every name is an id, every
  call is classified (`Proc`, `Declare`, `Intrinsic`, or a late-bound
  `BLate`), every implicit conversion is an explicit `Convert`, arguments are
  `Value`/`Ref`/`Missing` in parameter order, `With` is a hidden local (or an
  inlined block for user-defined types), `Static` locals are hoisted into
  module variables owned by the procedure, labels are ids.
- **Late binding is explicit.** Member access on `Variant`/`Object`, default
  members of unknown receivers and `!` access become `BLate` with the
  `IDispatch` flavour (`Get`, `PropertyPut`, `PropertyPutRef`) and named
  arguments. A member of a *library* type that the provider cannot describe is
  also bound late but reported as `SEM005` (info), never silently.

## Resolution rules implemented

Simple names (section 5.6): procedure locals/parameters/local constants, then
the module's own symbols, then public symbols of the other standard modules
(ambiguity between modules is `SEM015`), then module names (a class module
with `VB_PredeclaredId` or a document/designer module is its default
instance), then the library layer: `vba-stdlib` intrinsics, built-in VBA
names, provider globals. Public types and enums of class modules are visible
project-wide. Qualified names: `Module.Name`, `Project.Module.Name`,
`Enum.Member`, `VBA.Left`, and member access on typed values. An unresolved
name is `SEM002` under `Option Explicit` and otherwise an implicit local
(type from the suffix or `Def<Type>`); an unresolved name that is called or used
as a statement is always `SEM003`. `Option Base`, `Option Compare`
(`Database` is treated as `Text`), `Option Private Module`, `DefInt..`.

Calls and indexing: a name followed by arguments is classified by what it
resolves to: procedure/property (argument matching), array (rank checked,
subscripts converted to `Long`), `Variant`/`Object` (late default-member or
runtime index), class with `Attribute x.VB_UserMemId = 0` (default member),
library type (provider default member). Argument matching supports
positional, named, skipped optional, `Optional` and `ParamArray`; `ByRef`
arguments pass variables, elements and fields by reference and make a
temporary for expressions, parenthesised values and type-converted copies
(`Variant` parameters). `SEM026` is the `ByRef argument type mismatch`
error.

Assignment: `Let` of an object variable needs `Set` (`SEM007`); an object on
the right of `Let` yields its default member (`rvalue`); `Set` needs object
targets and values; `Property Let/Set/Get` are chosen from the group
(including write-only properties and indexed properties); the function name
inside its own function is the result slot (reading it is a recursive call).

Typing: operator result types follow the VBA operator tables (**doc**):
`Variant` operand gives `Variant`; mixed integer widths promote to the wider;
`/` and `^` give `Double` (`Single/Single` gives `Single`); `\` and `Mod` give
`Integer`/`Long`/`LongLong`; `Currency` combined with `Double` is `Double`;
`Date - Date` is `Double`; logical operators are bitwise on the wider integer
width (`Boolean` stays `Boolean`). Literals: unsuffixed decimal integers are
`Integer`, then `Long`, then `Double` (never `LongLong`; **doc**), `&H`/`&O`
literals are 16-bit `Integer` up to `&HFFFF` and 32-bit `Long` up to
`&HFFFFFFFF`, suffixes force the type. Literal conversions and negation are
folded.

Constants (`Const`, enum members, array bounds, `Optional` defaults, fixed
string lengths): folded with `vba-value` operators, lazily and
cycle-safe (`SEM010`), converted to the declared type; folding is not applied
to ordinary expressions.

Run-time provided intrinsics (called as `Callee::Intrinsic`, implemented by
the VM, not present in the `vba-stdlib` registry): `CreateObject`, `GetObject`,
`MsgBox`, `InputBox`, `DoEvents`, `Beep`, `Erl`, `Command`, `Shell`, the file
system functions and statements (`FreeFile`, `EOF`, `LOF`, `Loc`, `Seek`,
`FileLen`, `FileDateTime`, `GetAttr`, `SetAttr`, `CurDir`, `Dir`, `Kill`,
`MkDir`, `RmDir`, `ChDir`, `ChDrive`, `FileCopy`), and the `Mid`
statement, which is bound as a call of `MidStatement(ref target, start,
length|missing, value)` (`MidBStatement` for `MidB`).

## Declaration-level checks

| Code | Check |
| --- | --- |
| SEM001 | duplicate declaration (module, procedure, parameter, label scope, module names) |
| SEM002 / SEM003 | undeclared (Option Explicit) / undefined name |
| SEM004 | unresolved library type (not a project type, not provided) |
| SEM005 | info: library member / host member bound late because it cannot be verified |
| SEM006 | type mismatch (assignment, operands, conditions) |
| SEM007 | `Set`/`Let` misuse, missing `Property Let/Set` |
| SEM008 | argument list errors (count, unknown/duplicate named, missing required) |
| SEM009 | not callable / not a Sub in value position / not indexable |
| SEM010 | constant expression required / evaluation error / cycle |
| SEM011 | `Implements` incomplete, mismatched, duplicated, self, wrong module kind |
| SEM012 | `Property Get/Let/Set` pairing (arity, parameter types, value type) |
| SEM013 | label undefined / duplicate (line numbers are labels) |
| SEM014 | `Exit` without a matching construct |
| SEM015 | ambiguous name across modules |
| SEM016 | `Option` misplaced, repeated or invalid for the module kind |
| SEM017 | `Event`/`RaiseEvent`/`WithEvents`/handler problems |
| SEM018 | `For`/`For Each` control variable, `Next` mismatch |
| SEM019 / SEM028 | no such member / member not accessible |
| SEM020 | `Me`, `.x`, `!x` or `With` without an object |
| SEM021 | array rules (rank, bounds order, `ReDim` of fixed arrays, `Erase`) |
| SEM022 | `Declare` problems (Private in classes, empty `Lib`) |
| SEM023 | `Class_Initialize`/`Class_Terminate` signature |
| SEM024 | `Optional`/`ParamArray` rules |
| SEM025 | not assignable (constant, call result) |
| SEM026 | `ByRef` argument type mismatch |
| SEM027 | UDT/enum declaration (recursion, value range) |
| SEM099 | construct not supported (explicit; `DefDec`, `AddressOf` in profiles without it) |

`PtrSafe` on 64-bit targets and its availability by generation are diagnosed
by the parser (`SYN010`) from the same profile, so they are not repeated.

`Implements` (class interfaces): every public member of the interface class
(procedures, and public variables as `Property Get` plus `Let`/`Set`) needs a
procedure `Interface_Member` of the same kind, parameter types and result
type; the pairing is recorded in `ModuleInfo::implements` for dispatch. For a
library interface the provider must enumerate members (`interface_members`),
otherwise an info diagnostic says completeness is not verified.

Events: `Event` only in class modules; `WithEvents` variables must be class or
library types; a procedure `Var_Event` is a handler when `Var` is a
`WithEvents` variable of the module, its event must exist (project class
events, or the provider's `events`) and its parameter types must match.

## Not implemented / known gaps

- Document and designer modules: members of the host base class and controls
  (`Text1.Text`, `Show`, `Me.Caption`) cannot be verified without the form
  description, so they are bound late with `SEM005`. A bare, unqualified,
  undeclared name in such a module is still an ordinary implicit variable /
  `SEM002`.
- String literals are decoded as UTF-8 if valid, otherwise as Latin-1; the
  module code page is not applied. `Option Compare Database` compares like
  `Text`. Date literals that `vba-value` cannot parse stay as `DateText`.
- Operator result types and the `Currency`/`Single` mixing rules are the
  documented tables, not verified against a host (**unverified**).
- `Foo (x)` in a call statement is treated as an argument list (so `x` is by
  reference); real VBA passes it by value.
- `Static` locals are hoisted before local constants exist, so a `Static`
  array cannot use a local `Const` as a bound.
- No control-flow analysis: unreachable code, use before assignment, a
  missing `Return` for `GoSub`, `Resume` outside a handler and `Next` lists
  that close several loops (`Next j, i`) beyond the first name are not checked.
- `Implements` is not transitive; interface events and the Friend/Instancing
  attributes are not modelled; `Public Const`/`Type` in class modules are not
  rejected.
- Only the default `Err`, `Debug`, `Collection` and the listed constants are
  built in; everything else comes from the provider or is reported.
- `With` on a user-defined type re-evaluates the variable expression at each
  use instead of binding a reference.
- Profile gating covers `LongPtr`, `LongLong`, `Implements`, `Events`,
  `AddressOf`; the Access95 profile has no additional rules.

## Tests

`tests/diagnostics.rs` is a table of 46 small projects with the exact
`CODE@line` list; `tests/typing.rs` checks operator and literal types and call
classification on the bound tree; `tests/semantics.rs` checks the model
(scopes, properties, visibility, host modules, library events/interfaces,
profiles); `tests/corpus.rs` analyses a realistic multi-module project
(standard module with Enum/UDT/arrays/error handling, an interface class,
an implementing class with events, a `WithEvents` consumer, Win32 API
declares) for both 32- and 64-bit profiles; `tests/fuzz.rs` runs random token
soups, structured statement soups and mutated fixtures through
parse -> sema (no panic, deterministic output).
