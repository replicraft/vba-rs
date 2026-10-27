# vba-project: the VBA project model

Crate: `crates/model/vba-project`. It is the in-memory domain model of a VBA
project. It contains no binary-format knowledge and no dependency on
`vba-format`; format crates depend on it, never the reverse.

## What it models

- `VbaProject`: name, description, code page, platform (`SysKind`), LCIDs,
  compat version, help file and context, library flags, version, conditional
  compilation constants, project CLSID, declared `Protection`, host extenders,
  editor workspace records, `references` and `modules`.
- `Module`: name, `ModuleKind` (standard, class, document, designer), source
  text with its original line endings, `has_source`, description, help
  context, read-only and private flags, and an optional export `file_header`.
- `Reference`: `Registered`, `Project` and `Control` (the latter optionally
  carrying the original type library identifier). Library identifiers are kept
  as text and are not validated.
- `Protection`: the restrictions the file declares. `locked_for_viewing()`,
  `is_hidden()` and `notice()` summarize them. They describe the file and are
  **not security**: module source is stored unencrypted in a protected
  project, and the model enforces nothing.
- Source availability (`SourceAvailability`, `ProjectSourceAvailability`):
  `Module::source_availability`, `source_is_ambiguous`,
  `performance_cache_len`; `VbaProject::source_availability` and
  `modules_without_source`. `set_module_source` on a compiled-only module
  fails with `ModelError::CompiledOnlyModule`. See
  [compiled-projects.md](../runtime/compiled-projects.md).
- `ProjectVersion` (`VbaProject::project_version`): version-related facts: the
  `_VBA_PROJECT` version word, `SysKind`, code page, LCID, per-reference
  library facts (`ReferenceHint`) and `compiled_only`, plus a deliberately
  tiny `profile_hint()` table whose entries are hypotheses. It lives in this
  crate because it is model-level information; the word is read from the
  preserved `_VBA_PROJECT` bytes under `VBA_PROJECT_STREAM_KEY` (the only
  format-specific knowledge it has), so the dependency still points from
  `vba-format` to `vba-project` and never back. Profile names are plain
  strings; this crate does not depend on `vba-profile`. See
  [vba-format.md](vba-format.md) for the limits.
- Editing helpers on `VbaProject`: `module`, `module_mut` (case-insensitive,
  as VBA compares names), `add_module` (rejects duplicates), `remove_module`,
  `set_module_source`.

## Carrying data the model does not understand: `Preserved`

Every `VbaProject` and `Module` has a `preserved: Preserved` field, an ordered
map from string keys to raw bytes. The model never interprets it. A format
crate fills it when reading and consumes it when writing, so bytes it does not
(or cannot) model survive an edit. Rules:

- Keys are namespaced by the writer of the data, for example `ovba:dir`.
  `vba-format` documents its keys in `vba_format::keys`.
- Cloning a project clones its preserved data. The model does not know whether
  an entry is still valid after an edit; the format crate that wrote it
  decides (for example by comparing the model with what it originally parsed).
- The model offers `insert`, `get`, `remove`, `contains`, `iter`,
  `iter_prefixed`, `remove_prefixed`, `len`, `is_empty`, `total_bytes`.

## Standalone sources

`Module::from_source_file(file_name, text)` builds a module from the text of a
`.bas`, `.cls` or `.frm` file:

- the kind comes from the extension;
- the name comes from `Attribute VB_Name = "..."`, else the file stem;
- for `.cls` and `.frm`, the export header (`VERSION` line and the balanced
  `BEGIN`/`Begin` ... `END`/`End` block) is split into `file_header`;
- `Module::to_source_file` joins header and source again, byte-identically.

Not supported: the binary `.frx` companion of a form, decoding of file bytes
(callers pass text), validation of the header contents, and parsing of the VBA
language itself.

## Hostile input

The model itself reads nothing from bytes. Its one text reader is
`Module::from_source_file`: the header scan is linear in the text and does not recurse, the
`Begin`/`End` depth counter cannot go below zero, and every failure is a
`StandaloneError`. `crates/model/vba-project/tests/hostile.rs` feeds it random fragments
(unbalanced blocks, byte-order marks, NULs, non-ASCII, empty and odd file names) and a
200000-level nested header. The stream and container readers that fill the model are in
`vba-format` (see its "Hostile input" section).

## Not in scope

The model describes a project; it does not enforce `Protection`, validate
identifiers or library ids, or execute code.
