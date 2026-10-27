# Language server (`vba-lsp-core`, `vba-lsp`)

`crates/language/vba-lsp-core` is the editor-agnostic language intelligence: no
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
  tables of the analysis and the rules of docs/language/sema.md: procedure locals,
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

## Feature matrix

| Capability | Core API | LSP method | Status | Tests |
| --- | --- | --- | --- | --- |
| Completion: keywords, in-scope symbols | `View::completions` | `textDocument/completion` | done | `tests/completion.rs` |
| Completion: members of project types | `View::completions` | same, trigger `.` | done | `tests/project.rs` |
| Completion: members of library and host types | `View::completions` + `Workspace::set_type_provider` | same | done; `vba-lsp` loads `vba.typeLibraries` and `vba.hostLibraries` (see below), embedders install a provider; items carry `documentation` when the provider has some | `tests/library.rs`, `tests/host_library.rs` |
| Completion: names of described libraries | same | same | done: global functions, constants, objects, types after `As`, `Library.` qualifier | `tests/host_library.rs` |
| Hover: type and signature | `View::hover` | `textDocument/hover` | done; library members with a provider | `tests/navigation.rs`, `tests/project.rs`, `tests/library.rs` |
| Signature help | `View::signature_help` | `textDocument/signatureHelp` | done for procedures, library functions and provider-described members | `tests/completion.rs`, `tests/library.rs` |
| Go to definition | `Workspace::definition` | `textDocument/definition` | done, cross-module | `tests/project.rs`, `tests/navigation.rs` |
| Find references | `Workspace::references` | `textDocument/references` | done, cross-module | same |
| Document symbols | `View::document_symbols` | `textDocument/documentSymbol` | done, hierarchical | `tests/outline.rs` |
| Rename | `Workspace::prepare_rename`, `rename` | `textDocument/prepareRename`, `rename` | done, scope-aware with a re-analysis safety check | `tests/project.rs`, `tests/project_properties.rs` |
| Semantic tokens | `View::semantic_tokens` | `textDocument/semanticTokens/full`, `range` | done | `tests/outline.rs` |
| Folding ranges | `View::folding_ranges` | `textDocument/foldingRange` | done | `tests/outline.rs` |
| Formatting: indentation | `View::format`, `format_range` | `textDocument/formatting`, `rangeFormatting` | done, idempotent | `tests/formatting.rs`, `tests/properties.rs` |
| Formatting: reserved-word casing | `FormatOptions::keyword_case` | formatting option `keywordCase` (boolean, default off) | done, idempotent, letter case only | `tests/keyword_case.rs`, `apps/vba-lsp/tests/stdio.rs` |
| Workspace symbols, selection ranges, diagnostics | see Features | matching methods | done | `tests/project.rs`, `apps/vba-lsp/tests/stdio.rs` |

### Library and host types

`Workspace::set_type_provider(Some(Arc<dyn TypeProvider + Send + Sync>))`
installs the embedding application's description of type libraries and host
object models (the `vba_sema::TypeProvider` trait). The provider is used by
the project analysis, so declarations such as `Dim d As Scripting.Dictionary`
resolve (no `SEM004`), and by the editor features: after `.` on a receiver of
a library type the members come from `interface_members` (the provider
must list them, `StaticLibrary::as_interface` does), hover on a member shows
its declaration-style signature and owner, and calls to a member get signature
help with the active parameter (including named arguments and `ParamArray`).
The type of a member's result is followed, so `d.Item(1).` completes through
the provider again. A provider that does not enumerate members yields hover
and signature help but no member completion.

The `vba-lsp` executable builds its provider from the `vba.typeLibraries`
setting (below): a `vba_typelib::sema::TypelibProvider` over every loaded
library, which is `Send + Sync` and enumerates the members of interfaces,
dispinterfaces and (through their default interface) coclasses. Embedders can
use the same adapter with `Workspace::set_type_provider`.

## Features

| Feature | Notes |
| --- | --- |
| Diagnostics | Lexer and syntax diagnostics of the tolerant parse, version gating per profile, `SEM###` diagnostics of the project analysis (mapped to the right document), plus pluggable providers. Related labels and notes are kept. |
| Document symbols | Procedures (Sub, Function, Property), `Declare`, `Event`, `Type` (+ fields), `Enum` (+ members), `Const`, variables; procedure locals as children. |
| Folding | Procedures, types, enums, `If`, `Select Case`, `Case` clauses, loops, `With`, `#If` branches, runs of 2+ comment lines. |
| Semantic tokens | Full and range. Keywords, operators, literals, comments, type names, declared names by class, library functions (`defaultLibrary`), labels, `#` directives, inactive code (`comment` + `inactive`). Multi-line tokens are split per line. Unresolved names are left unclassified. |
| Hover | Names bound by the project: declared type of variables, parameters and fields, signature of procedures (all members of a property group), constant values, enum member values, UDT and enum declarations, `Me`, the declaring module; late-bound members say so. Keywords (one-line description), library functions (signature with types, optional markers, module, `$` form). Names the project does not bind use the syntactic declaration text. |
| Completion | Context-dependent keywords (statement start, module level, after `End`, `Exit`, `On`, `Option`, `As`, ...), in-file names visible at the cursor, public names of other modules (standard-module members, public types and enums, module names; only types and class modules after `As`/`New`), library functions, `VBA.`/`Strings.`... qualifiers, labels after `GoTo`. After `.`: the members of the receiver when its type is known: public members of a class module (all members inside that class, `Me.`), fields of a user-defined type, public members of a standard module (`Utils.`), enum members, module names after the project name, and the object of an enclosing `With` (`.`). Members of library types when a type provider is installed (see above). Nothing for `Variant`/`Object` receivers. Type names respect the profile (`LongPtr`, `LongLong`). Nothing inside comments and strings. |
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
property-tested over random documents. Options: `tabSize`, `insertSpaces`, and
`keywordCase` (boolean, default off; sent as an extra formatting property).
With `keywordCase`, reserved words are rewritten to their canonical spelling
(`elseif` becomes `ElseIf`); only the case of letters changes, never a word
after `.` or `!`, an attribute name, inactive text, comment or string. Names
that merely resemble keywords and words with a type suffix are left as
written. Idempotence and "only letter case and indentation differ" are
property-tested (`tests/keyword_case.rs`).

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
           "moduleKinds": { "*.txt": "class", "Sheet1.vba": "document" },
           "typeLibraries": ["C:/libs/scrrun.dll", "libs/Shapes.tlb"],
           "hostLibraries": ["libs/app-objects.json"],
           "moduleHosts": { "Form_Orders": "Form_Orders",
                            "Report_Sales": { "class": "Report_Sales", "eventSource": "Report" } } } }
```

`profile` is one of the `vba-profile` identifiers, `encoding` is `utf-8` or a
code page (`cp1252`, `932`), `constants` are project `#Const` values,
`semantic` switches the project analysis on (default) or off, `projectName`
is the project name for `Project.Module.Name`, and `moduleKinds` maps a URI,
file name or `*.ext` pattern to `standard`, `class`, `document` or
`designer`. `typeLibraries` is a list of
file paths and `hostLibraries` a list of file paths or inline description
objects (see below). Absent
settings take defaults; invalid entries are reported with `window/logMessage`
and ignored. A change of `encoding` applies to documents opened afterwards.

Errors are explicit: requests the server does not implement get
`MethodNotFound` (-32601) naming the method; requests for unopened documents
or with malformed parameters get `InvalidParams` (-32602); refused renames
and unsupported semantic requests get `RequestFailed` (-32803) with the
reason; requests after `shutdown` get `InvalidRequest` (-32600). `exit`
returns code 0 after `shutdown`, 1 otherwise.

### Forms: `vba.moduleHosts`

`moduleHosts` maps a module name to the class of a host library that is that
module's `Me` (a string, or `{ "class": name, "eventSource": prefix }`; the
prefix of the handlers of the class's own events is `Form` by default). In
such a module the controls, properties and methods of that class complete as
bare names and after `Me.`, with signatures and documentation; `Me.txtName.`
and `txtName.` complete the members of the control's class; hover and
signature help work on them (`Requery T`, `Me.Requery(T)`); and the
diagnostics are those of the analysis: a control of another form is `SEM019`
or `SEM002`/`SEM003` naming the class, handlers are checked against the
described events. An embedder whose form design changes installs the new
description (`Workspace::set_type_provider`, or a changed `hostLibraries`
setting); `Config::module_hosts` is the in-process equivalent of the
setting. Entries for modules that are not open are ignored; a class the
libraries do not describe is `SEM004` on the module.

### Host libraries (`vba.hostLibraries`)

The object model of the embedding application, described once with
`vba-host-model` ([host-model](../architecture/host-model.md)) and generated
as JSON by the application (`HostLibrary::to_json_string()`), is the second
source of types. Each entry of `vba.hostLibraries` is a path to such a file or
the description object itself (convenient in `initializationOptions` when the
application starts the server). Each one is parsed, validated and installed
as one provider that is consulted before the type libraries. A file that
cannot be read, is not the format, has another `version`, or describes an
invalid library (duplicate names, dangling types...) is reported with
`window/logMessage` naming the problem and skipped; the others still load,
files are read through the 64 MiB cap, at most 64 entries are used, and
changing the setting reloads everything.

With a host library the server offers, from the description alone: member
completion with signatures and documentation (`CompletionItem.documentation`,
hidden members left out), completion of global functions, constants,
enumeration members and predeclared objects in expressions and of classes and
enumerations after `As`/`New`, completion after `Library.`, hover (signature
and doc text) for members, global names and types, signature help for members
and global functions, typing of predeclared objects (`Application.` completes
through return types), and the `SEM###` diagnostics (unknown member, argument
count, argument and `ByRef` type, read-only assignment, unknown type).
`vba-lsp/tests/host_libraries.rs` runs this over the real protocol.

### Type libraries (`vba.typeLibraries`)

Each path is read as a raw type library (`.tlb`, `.olb`; MSFT or SLTG, chosen
by the file's magic) or as a PE image (`.dll`, `.ocx`, `.exe`), whose
`TYPELIB` resources are all loaded. Relative paths are relative to the
server's working directory. The libraries are combined into one provider
installed in the workspace; library types then resolve (no `SEM004`), and
member completion, hover and signature help work as described above.

Loading is defensive and never stops the server: a file larger than 64 MiB is
refused (reading is capped, so an endless file cannot exhaust memory), at most
64 paths and 256 libraries are used, a missing, unreadable or malformed file
is skipped and the others still load, and a panic inside the reader is caught
and reported like any other error. Every problem is sent as a
`window/logMessage` warning naming the file; a summary (`loaded N type
libraries`) is logged at info level. The setting is read from
`initializationOptions` and every time the configuration is applied
(`workspace/didChangeConfiguration`, `workspace/configuration`); each time the
files are read again and the previous libraries are replaced, so editing a
library on disk takes effect on the next configuration change, and removing
the setting (or an empty list) unloads them. Diagnostics are republished for
all open documents afterwards.

Go to definition on a library member answers `null` (the libraries carry no
source locations); it is not an error.

## Not implemented

- Host object models: without `vba.hostLibraries`, `vba.typeLibraries` (or an
  embedder's provider) the analysis knows no library types (`SEM004`/`SEM005` appear for them) and
  members are not completed. The server does not discover libraries by itself
  (no registry, no `Reference` lines of a project): they are listed in the
  setting, and the host's own object model must be supplied as a type library
  file. A receiver of a library type is neither late bound nor renamable, and
  library members have no definition (nothing to jump to). Document and designer modules have no
  host members. Members of a provider type that cannot enumerate its members
  (`interface_members` returns `None`) are not offered for completion.
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
new diagnostic code afterwards). `apps/vba-lsp/tests/libraries.rs`: type
libraries written by the test itself (the synthetic MSFT and SLTG writers of
the `vba-typelib` tests and a minimal PE image; no binary fixtures are
committed) are loaded by the real binary from `initializationOptions` and
`workspace/didChangeConfiguration`: member completion, hover, signature help,
null definition, diagnostics before and after, unloading, and bad files
(missing, garbage, truncated, a directory, a wrong setting type) that are
reported while a good file still loads.
