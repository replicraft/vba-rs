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
  `View::diagnostics()` without any API change. A future `vba-sema` plugs in
  here.

## Name resolution is syntactic

Definition, references, rename, hover on declared names, semantic-token
classes and completion of declared names use a name-based walk over the
syntax tree (`scope.rs`). It is **not** semantic analysis, and full
resolution arrives with `vba-sema`. What it does: declarations of
procedures (property pairs share one symbol), `Declare`s, events, module and
local variables and constants, parameters, user types, enums and enum
members, and labels (a separate namespace, resolved from `GoTo`, `GoSub`,
`On Error GoTo`, `Resume`); case-insensitive matching with type suffixes
ignored; innermost scope first (procedure, then module). What it does not:

- Member access is never resolved except `EnumName.Member`. `obj.Member`,
  `With` blocks, type members (`Type` fields) and `Me.` are invisible, so
  renaming them is refused (`Unsupported`).
- No types, no other modules, no host objects, no implicit declarations
  (undeclared variables are not symbols), no `Option Explicit` checks.
- Inactive `#If` branches are not parsed and therefore have no symbols.
- A name that does not resolve here may still be valid elsewhere in the
  project; features then return nothing rather than guess.

Rename refuses a new name that is not a plain identifier (reserved words,
suffixes, non-ASCII, over 255 bytes) and one that would clash with a
declaration in the same scope or shadow/capture a use (module symbol vs.
local of a procedure that uses it).

## Features

| Feature | Notes |
| --- | --- |
| Diagnostics | Lexer and syntax diagnostics of the tolerant parse, version gating per profile, plus pluggable providers. Related labels and notes are kept. |
| Document symbols | Procedures (Sub, Function, Property), `Declare`, `Event`, `Type` (+ fields), `Enum` (+ members), `Const`, variables; procedure locals as children. |
| Folding | Procedures, types, enums, `If`, `Select Case`, `Case` clauses, loops, `With`, `#If` branches, runs of 2+ comment lines. |
| Semantic tokens | Full and range. Keywords, operators, literals, comments, type names, declared names by class, library functions (`defaultLibrary`), labels, `#` directives, inactive code (`comment` + `inactive`). Multi-line tokens are split per line. Unresolved names are left unclassified. |
| Hover | In-file declarations (declaration text), keywords (one-line description), library functions (signature with types, optional markers, module, `$` form). |
| Completion | Context-dependent keywords (statement start, module level, after `End`, `Exit`, `On`, `Option`, `As`, ...), in-file names visible at the cursor, library functions, enum members after `Enum.`, `VBA.`/`Strings.`... qualifiers, labels after `GoTo`. Type names respect the profile (`LongPtr`, `LongLong`). Nothing inside comments and strings. No type-based member completion. |
| Signature help | Parenthesised and implicit (`Foo a, b`) calls of in-file procedures and library functions; named arguments (`name:=`); `ParamArray`; nested calls. Object members are not resolved. |
| Definition, references, rename | See above. Rename offers `prepareRename`. |
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
formatting; position encoding UTF-16. Diagnostics are pushed on open, change,
close (cleared) and configuration change.

Configuration, from `initializationOptions` or `workspace/configuration`
(section `vba`, pulled when the client supports it) and
`workspace/didChangeConfiguration`:

```json
{ "vba": { "profile": "vba7.1-excel-win64", "encoding": "utf-8",
           "constants": { "DEBUGGING": 1 } } }
```

`profile` is one of the `vba-profile` identifiers, `encoding` is `utf-8` or a
code page (`cp1252`, `932`), `constants` are project `#Const` values. Absent
settings take defaults; invalid entries are reported with `window/logMessage`
and ignored. A change of `encoding` applies to documents opened afterwards.

Errors are explicit: requests the server does not implement get
`MethodNotFound` (-32601) naming the method; requests for unopened documents
or with malformed parameters get `InvalidParams` (-32602); refused renames
and unsupported semantic requests get `RequestFailed` (-32803) with the
reason; requests after `shutdown` get `InvalidRequest` (-32600). `exit`
returns code 0 after `shutdown`, 1 otherwise.

## Not implemented

Workspace-wide (multi-module) features; semantic diagnostics (hook only);
type-based completion, hover and member resolution; code actions, quick
fixes, code lens, inlay hints, call hierarchy, workspace symbols, document
highlight, linked editing, on-type formatting, delta semantic tokens;
dynamic registration; pull diagnostics; debounced or cancelled analysis
(requests run synchronously on the main loop); identifier case normalisation
on format; non-UTF-16 position encodings; code pages other than the
single-byte ones that `encoding_rs` knows and 932/936/949/950 (others
fall back to one byte per column).

## Tests

Per feature, table-driven on realistic snippets (`tests/outline.rs`,
`navigation.rs`, `completion.rs`, `formatting.rs`, `positions.rs`); property
tests (`tests/properties.rs`: no panics on arbitrary edits and probes across
all features, formatter laws, position round-trips and monotonicity for
arbitrary bytes in several encodings, incremental edits against a string
model); protocol tests that spawn the `vba-lsp` binary and speak JSON-RPC over
real stdio (`apps/vba-lsp/tests/stdio.rs`).
