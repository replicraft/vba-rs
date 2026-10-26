# Compiled-only VBA projects

Some VBA projects carry no source text: `.mde`/`.accde` files, and any project
whose source was stripped. Their module streams hold only the
implementation-specific *performance cache* (compiled "p-code" state). This
document says what vba-rs detects, what it preserves, and what it deliberately
does not do.

## Policy

Preserve byte for byte; detect and report precisely; never invent or
reconstruct source; never execute compiled state.

## What is detected

A module stream (MS-OVBA 2.3.4.3) is `MODULEOFFSET` bytes of performance cache
followed by a compressed source container (2.4.1). Classification uses only
two facts: is there a container after the cache, and is the cache non-empty.

| `SourceAvailability` | Container | Cache | Meaning |
| --- | --- | --- | --- |
| `Present` | yes, non-empty text | any | normal module |
| `Empty` | yes, decompresses to nothing | any | a module with source whose text is empty (an empty class module still stores a container) |
| `CompiledOnly` | no | yes | compiled-only module |
| `AbsentNoCache` | no | no | no source and nothing proves it was ever compiled |

Project level (`ProjectSourceAvailability`): `FullSource` (every module
`Present` or `Empty`; also a project with no modules), `CompiledOnly` (at least
one module, none with source), `Mixed` (otherwise). `CompiledOnly` and
`AbsentNoCache` both count as lacking source.

### Limits of the heuristic

- The heuristic cannot see *why* a container is missing. `AbsentNoCache` may be
  a stripped module whose cache was also removed, a module of a project never
  compiled, or a module created in memory without text. It is reported as such
  and `Module::source_is_ambiguous()` is true.
- `Empty` with a non-empty cache is ambiguous too: it is what Office writes for
  an empty compiled module, but a tool could also leave a placeholder
  container while stripping. It is classified `Empty` (runnable, defines
  nothing) and flagged ambiguous; `vbars inspect` prints a note.
- A container that decompresses to text that is merely wrong (replaced source,
  mismatched cache) is not detectable: the cache is opaque and is not compared
  with the source.
- The `_VBA_PROJECT` version and cache are not used for classification. They
  are preserved, never interpreted.

## What is preserved

Reading and writing a compiled-only (or mixed) project without edits
reproduces the performance cache of every module, the `_VBA_PROJECT` stream
(version field and cache), `dir`, `PROJECT` and every other stream and storage
byte for byte, through `write_project`, `write_project_with_report` (empty
`changed_streams`), tolerant reading, and `replace_ooxml_project`. Metadata edits
that rewrite `dir` leave `_VBA_PROJECT` and all module streams untouched.
Container-level bytes (sector layout, timestamps) are not preserved, as for
every project. Tests: `crates/formats/vba-format/tests/compiled_only.rs`
(including a property test over arbitrary mixes of compiled-only, empty and
full modules).

## What is refused

- Supplying source for a module whose stored source is absent:
  `Error::UnsupportedEdit` naming the module (`ModelError::CompiledOnlyModule`
  from `set_module_source`). Source written next to an unrelated cache would
  silently mismatch it.
- In a project with a compiled-only module, editing another module under
  `StaleCache::Invalidate`: invalidation resets `_VBA_PROJECT` to force a
  recompile, which a module without source cannot survive. Use
  `StaleCache::Keep` knowingly or edit a source project instead.
- Running: `Session::builder().build_project` returns `Error::CompiledOnly`
  (project and module names); `Mixed` projects only with `allow_partial()`,
  where calls into the missing modules are `Error::CompiledOnly`.
- CLI: `vbars check`, `parse` and `run` exit 6. See [runtime.md](runtime.md).

## Deliberately not supported

Executing, decompiling or disassembling p-code. The performance cache is
undocumented (MS-OVBA calls it implementation-specific), its layout depends on
the Office/VBA version and bitness, and this project has no verification
fixtures (no compiled projects with known source and a known-good decoder
output). Any interpreter or decompiler written now would be unverifiable
guesswork presented as fact, and a wrong decompilation of code that is about to
be run is worse than a refusal.

## What an embedder should do

- Treat a compiled-only project as opaque: store, copy and re-embed it
  unchanged; check `VbaProject::source_availability()` before analysis.
- If a source project matching the compiled one is available, use that for
  analysis and execution, and keep the compiled project for distribution.
- Do not offer to "edit the macros" of a compiled-only project; there is
  nothing to edit.
- For ambiguous modules, show the user the state (`source_is_ambiguous`)
  rather than choosing for them.
