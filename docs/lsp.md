# Language server (`vba-lsp-core`, `vba-lsp`)

`crates/tooling/vba-lsp-core` is the editor-agnostic language intelligence: no
protocol transport, no async runtime, no dependency on `lsp-types`.
`apps/vba-lsp` is the executable `vba-lsp`, a thin stdio server (`lsp-server`
+ `lsp-types`, synchronous) that maps requests onto the core. Everything is
built on the one shared parser, `vba-syntax`, in tolerant mode, so features
work on broken code.

## Core design

- `Workspace` holds open `Document`s and the `Config` (VBA profile, default
  encoding, project `#Const` values). `Workspace::view(uri)` returns a `View`
  with all features for the current version. Unopened documents are an
  `Error::NotOpen`; requests the implementation cannot answer return
  `Error::Unsupported`/`Error::Invalid` with a message.
- **Text.** A document is a byte buffer in a declared encoding (UTF-8 for
  editor buffers by default; `Config::encoding` or
  `Workspace::open_with_encoding` select a Windows code page). Incremental
  edits are applied to the bytes: the edit range is converted to a byte span
  with the line index, the replacement text is encoded to the buffer
  encoding (characters the code page cannot represent become `?`).
- **Positions** are LSP positions: zero-based line, UTF-16 column (`text.rs`,
  `TextMap`). Columns are converted per character, decoding according to the
  encoding: UTF-8 (4-byte sequences are two units, invalid bytes one unit
  each), single-byte code pages (one unit per byte), and the double-byte
  code pages 932, 936, 949 and 950 (lead byte plus trail byte >= 0x40 is one
  unit). A column inside a surrogate pair or a double-byte character rounds
  down to the character start; columns and lines past the end clamp. Lines
  end with CR, LF or CRLF.
- **Parse cache.** The parse of a version is computed on first use and cached
  in the document (`Arc<Analysis>`); any edit or configuration change drops
  it. The syntactic scope index is built lazily next to it.
- **Semantic diagnostics hook.** `SemanticDiagnostics` is a trait
  (`diagnose(&SemanticInput) -> Vec<vba_diagnostics::Diagnostic>`);
  providers added with `Workspace::add_diagnostics_provider` are merged into
  `View::diagnostics()` without any API change. The `vba-sema` integration
  below does not use it (it needs all documents, not one tree); the hook stays
  for single-document providers.

## Project analysis (`vba-sema`)

Every open document is one module of one `vba_sema::analyze_sources` run
(`project.rs`). The core stays transport-free; `Workspace` owns the cache.

- **Module name and kind.** The name is `Attribute VB_Name`, else the file
  stem of the URI. The kind comes from, in order: `Config::module_kinds`
  (patterns: full URI, file name, or `*.ext`; first match wins), the file
  extension (`.bas` standard, `.cls` class, `.frm` designer), the export
  header (`VERSION 1.0 CLASS` is a class, any other header block a designer),
  and is standard otherwise. Document modules (host sheets) need an explicit
  `module_kinds` entry.
- **Debounce and cache.** Any open, edit, close or configuration change bumps
  a workspace generation; the project is analysed lazily by the first
  semantic request after a bump and reused by every request until the next
  one (`Workspace::semantic_runs` counts runs; tested). Editors that send
  several changes in a burst therefore cost one analysis. `vba-lsp` also
  defers publishing diagnostics while more client messages are queued.
- **Diagnostics.** Only `SEM###` diagnostics of the analysis are used (the
  tolerant parse already reports syntax and lexer findings); each goes to the
  document whose module the diagnostic's file id names, so an error in one
  module is reported in that module even when another module's edit caused it.
  Byte spans are converted with that document's text map, i.e. to UTF-16
  columns for UTF-8 buffers and per the code page for others. Related labels
  are kept. `Config::semantic = false` turns the whole integration off.
- **Binding** (`resolve.rs`). An identifier token is bound to a declaration
  (`Decl`: procedure group, `Declare`, variable, constant, type, field, enum,
  enum member, event, parameter/local/`Static`, module) with the declaration
  tables of the analysis and the rules of docs/sema.md: procedure locals,
  own module, public members of other standard modules (ambiguity binds
  nothing), module names; type positions; `Mod.Name`, `Enum.Member`,
  `Project.Mod.Name`; member access through the *declared* type of the
  receiver (`Me`, variables, fields, properties, function results, array
  elements, `New T`, the object of the enclosing `With`). Working from
  declared types instead of bound expressions keeps `x.` completable while
  the statement is still incomplete. A member of a `Variant`/`Object`
  receiver is **late bound**; anything else the project does not know
  (library types, unresolved names) is unknown.

## Resolution: project first, syntactic walk as fallback

Definition, references and rename ask the project binding first
(`Workspace::{definition, references, prepare_rename, rename}`, which return
locations in any open document). A definite binding is answered across
modules. A name that is unknown to the project, or a late-bound member, falls
back to the name-based walk over the syntax tree (`scope.rs`, the `View`
methods of the same names, single document); the walk is also what resolves
labels, which are not part of the project model. The walk knows declarations
of procedures (property pairs share one symbol), `Declare`s, events, module
and local variables and constants, parameters, user types, enums and enum
members, and labels, with case-insensitive matching, type suffixes ignored,
innermost scope first. It does not resolve member access except
`EnumName.Member`, has no types or other modules, and does not see inactive
`#If` branches. Semantic-token classification and the document outline still
use only the walk.

Rename:

- **Late-bound members are never renamed.** The cursor on one gives
  `Unsupported`. Renaming a member of a class module (procedure, property,
  public variable, event) is also refused while the workspace contains
  late-bound uses of the same name (those cannot be found); renaming a
  parameter is refused while any named argument `name:=` with that name
  exists (it cannot be attributed to the procedure).
- A new name must be a plain identifier (no reserved word, suffix, non-ASCII,
  at most 255 bytes) and must not clash with a declaration in the same scope or
  module, a module name, or a name that is public in another module (ambiguity);
  it must not be hidden by a local of a procedure that uses the target, nor
  capture an existing use in the procedure of a local being renamed.
- Last check: the edits are applied to copies of all documents, the project
  is analysed again, and the rename is refused when any diagnostic code
  appears more often in a module than before (this catches interface
  implementations, event handlers named after the renamed variable and
  everything the table checks miss). The same check guards the syntactic
  fallback.
- Edits only replace identifier tokens (property-tested: token streams are
  identical except for the renamed identifiers; no new diagnostic codes).
- Not covered: handler procedures named after a renamed `WithEvents`
  variable (`Var_Event`) and `Implements` members (`Iface_Member`) are not
  renamed along; the final check usually refuses the rename. Modules cannot be
  renamed.

## Features

| Feature | Notes |
| --- | --- |
| Diagnostics | Lexer and syntax diagnostics of the tolerant parse, version gating per profile, `SEM###` diagnostics of the project analysis (mapped to the right document), plus pluggable providers. Related labels and notes are kept. |
| Document symbols | Procedures (Sub, Function, Property), `Declare`, `Event`, `Type` (+ fields), `Enum` (+ members), `Const`, variables; procedure locals as children. |
| Folding | Procedures, types, enums, `If`, `Select Case`, `Case` clauses, loops, `With`, `#If` branches, runs of 2+ comment lines. |
| Semantic tokens | Full and range. Keywords, operators, literals, comments, type names, declared names by class, library functions (`defaultLibrary`), labels, `#` directives, inactive code (`comment` + `inactive`). Multi-line tokens are split per line. Unresolved names are left unclassified. |
| Hover | Names bound by the project: declared type of variables, parameters and fields, signature of procedures (all members of a property group), constant values, enum member values, UDT and enum declarations, `Me`, the declaring module; late-bound members say so. Keywords (one-line description), library functions (signature with types, optional markers, module, `$` form). Names the project does not bind use the syntactic declaration text. |
| Completion | Context-dependent keywords (statement start, module level, after `End`, `Exit`, `On`, `Option`, `As`, ...), in-file names visible at the cursor, public names of other modules (standard-module members, public types and enums, module names; only types and class modules after `As`/`New`), library functions, `VBA.`/`Strings.`... qualifiers, labels after `GoTo`. After `.`: the members of the receiver when its type is known: public members of a class module (all members inside that class, `Me.`), fields of a user-defined type, public members of a standard module (`Utils.`), enum members, module names after the project name, and the object of an enclosing `With` (`.`). Nothing for `Variant`/`Object` receivers and library types. Type names respect the profile (`LongPtr`, `LongLong`). Nothing inside comments and strings. |
| Signature help | Parenthesised and implicit (`Foo a, b`) calls of in-file procedures and library functions; named arguments (`name:=`); `ParamArray`; nested calls. Object members are not resolved. |
| Definition, references, rename | Across modules, see above. Rename offers `prepareRename`. |
| Workspace symbols | Modules, procedures, `Declare`s, variables, constants, types and fields, enums and members, events of all open documents (not locals). Matching is case-insensitive: exact, prefix, substring, then subsequence, best first. |
| Selection ranges | Token, enclosing syntax nodes, whole document. |
| Formatting | Document and range. Re-indents block structure only (see below). |

### Formatter

Only the leading whitespace of a line can change. The indent of a line is the
nesting depth of its first token in the tree (blocks plus `Case` clauses, +1
for `Type`/`Enum` members); `Case` lines are one level inside `Select`;
labels and `#If` directives are flush left; comment-only lines take the depth
of the next code line unless that line closes a block, in which case they stay
with the preceding body. Not touched: token text, blank lines, trailing
whitespace, continuation lines, lines with line numbers, `VERSION`/`BEGIN`
headers, inactive conditional text, lines that start with an error token.
Idempotence, whitespace-only changes and an unchanged token stream are
property-tested over random documents. Options: `tabSize`, `insertSpaces`.

## Server (`vba-lsp`)

Capabilities: incremental text sync, hover, completion (`.`), signature help
(`(`, `,`), definition, references, rename with prepare, document symbols
(hierarchical when the client supports it, otherwise flat), folding,
selection ranges, semantic tokens (full, range), formatting, range
formatting, workspace symbols (flat `SymbolInformation`, at most 500);
position encoding UTF-16. Definition, references and rename are
cross-document (rename returns a `WorkspaceEdit` with one entry per touched
file). Diagnostics are pushed for **every** open document whenever something
changed (a semantic error can move between modules when one is edited), only
for documents whose diagnostics or version changed, and once the client has no
further message queued, so a burst of edits is analysed once. Closing a
document clears its diagnostics.

Configuration, from `initializationOptions` or `workspace/configuration`
(section `vba`, pulled when the client supports it) and
`workspace/didChangeConfiguration`:

```json
{ "vba": { "profile": "vba7.1-excel-win64", "encoding": "utf-8",
           "constants": { "DEBUGGING": 1 },
           "semantic": true, "projectName": "VBAProject",
           "moduleKinds": { "*.txt": "class", "Sheet1.vba": "document" } } }
```

`profile` is one of the `vba-profile` identifiers, `encoding` is `utf-8` or a
code page (`cp1252`, `932`), `constants` are project `#Const` values,
`semantic` switches the project analysis on (default) or off, `projectName`
is the project name for `Project.Module.Name`, and `moduleKinds` maps a URI,
file name or `*.ext` pattern to `standard`, `class`, `document` or
`designer`. Absent
settings take defaults; invalid entries are reported with `window/logMessage`
and ignored. A change of `encoding` applies to documents opened afterwards.

Errors are explicit: requests the server does not implement get
`MethodNotFound` (-32601) naming the method; requests for unopened documents
or with malformed parameters get `InvalidParams` (-32602); refused renames
and unsupported semantic requests get `RequestFailed` (-32803) with the
reason; requests after `shutdown` get `InvalidRequest` (-32600). `exit`
returns code 0 after `shutdown`, 1 otherwise.

## Not implemented

- Host object models and COM libraries: the analysis runs without a type
  provider, so library types are unknown (`SEM004`/`SEM005` appear for them),
  members of library types are not completed, and a receiver of a library type
  is neither late bound nor renamable. Document and designer modules have no
  host members.
- Hover and completion use *declared* types: the type of an arbitrary
  expression (`(a + b).`, a call result chain through library objects) is not
  computed; the default member of a class and `!` access are not followed.
- Named arguments (`name:=`) are not bound to parameters (not found by
  references, not renamed; they block a parameter rename). Labels and
  event-handler/interface naming conventions are outside the project binding.
- Workspace analysis covers open documents only; there is no workspace folder
  scan, file watching or on-disk project (`.vbp`, export folder) loading, and
  no incremental sema: any change re-analyses the project (cached per
  generation, run on the request thread).
- Code actions, quick fixes, code lens, inlay hints, call hierarchy, document
  highlight, linked editing, on-type formatting, delta semantic tokens;
  semantic-token classes from the project binding; dynamic registration; pull
  diagnostics; cancellation; identifier case normalisation on format;
  non-UTF-16 position encodings; code pages other than the single-byte ones
  that `encoding_rs` knows and 932/936/949/950 (others fall back to one byte
  per column). Inactive `#If` branches have no symbols.

## Tests

Per feature, table-driven on realistic snippets (`tests/outline.rs`,
`navigation.rs`, `completion.rs`, `formatting.rs`, `positions.rs`); the
project features on a three-module project (standard module, class with an
export header, main module) in `tests/project.rs` (diagnostics per document,
caching, UTF-16 and code-page columns, module kinds, cross-module
definition/references/rename, late-bound refusal, clashes, hover, member
completion, workspace symbols); property
tests (`tests/properties.rs`: no panics on arbitrary edits and probes across
all features, formatter laws, position round-trips and monotonicity for
arbitrary bytes in several encodings, incremental edits against a string
model); protocol tests that spawn the `vba-lsp` binary and speak JSON-RPC over
real stdio (`apps/vba-lsp/tests/stdio.rs`, including a two-module project:
cross-file definition, references, rename, a semantic diagnostic, workspace
symbols, `Lib.` completion and a diagnostic that moves to another module when
one is edited). `tests/project_properties.rs`: no panics on arbitrary edits,
opens and closes across multi-document workspaces probing every semantic
feature, and rename results that only replace identifier tokens (token streams
equal outside the edits, edits ordered and disjoint, one name per rename, no
new diagnostic code afterwards).
