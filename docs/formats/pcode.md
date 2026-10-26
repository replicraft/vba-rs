# Compiled VBA ("p-code") research notes

Status: phase 0 (observation only). Nothing here is executable yet, and no
crate exists for it. This document records what is known about the module
performance cache and what is still hypothesis.

Tags: `[spec]` stated by the public MS-OVBA text; `[obs]` observed directly in a
real project that carries both source and cache; `[hyp]` hypothesis, not
substantiated. Facts without a tag are not added.

## Scope

Goal of the research line: execute compiled VBA so that compiled-only projects
(`.accde`, `.mde`, and documents whose source was stripped) can run on the
shared `vba-vm`. See [compiled-projects.md](../runtime/compiled-projects.md) for what the
project model does today (store, copy and re-embed the cache as opaque bytes;
never interpret it).

Priority target: VBA 7.0, 32-bit, as found in Access 2010 (the version of the
reference project below). VBA 7.1 and 64-bit variants come next, per fixture
availability; layout statements are not assumed to carry over between them.

Out of scope for phase 0: any opcode table, any decoder, any interpreter, any
database container reader.

Test material: extracted projects are local-only and never committed, and no
bytes derived from them (hex dumps, identifier lists) are committed either. This
document contains structure, counts and offsets only.

## Reference project used for observation

One real Access project with full source and a cache side by side.

* 33 modules (11 standard, 22 class), all with source and cache, so the
  source pipeline is usable as an oracle: source decompresses from
  `MODULEOFFSET` onward in every module `[obs]`.
* Code page 1252, LCID 1033. `PROJECTSYSKIND` is 1 (32-bit Windows) `[obs]`.
* References include `stdole`, `ADODB`, `VBIDE`, an Outlook library and DAO, with
  paths indicating an Office 2010 era installation `[obs]`.
* `_VBA_PROJECT` starts with the reserved constant `CC 61`, then the 16-bit
  version field, here `0x0097` `[spec: layout; obs: value]`. The specification
  calls the value implementation-specific; mapping it to a VBA release is
  `[hyp]` (VBA 7.x, consistent with the Office 2010 paths and with the
  conditional-compilation name `Win64` appearing in the project-level cache).
* Every layout statement below was observed in this one project; it must be
  re-verified against other versions and 64-bit projects before it is trusted for
  them.

### Which version made it

Evidence found inside the project, strongest first:

1. References pointing into an `Office14` installation, a DAO reference to the
   ACE DAO library under `OFFICE14`, and an Outlook type library at version 9.4
   (the Outlook 2010 library) `[obs]`. Together: Office 2010 (14.0), 32-bit on a
   64-bit OS. High confidence.
2. `PROJECTSYSKIND` = 1 (32-bit Windows) `[obs]`.
3. The conditional-compilation name `Win64` in the project-level cache, which
   exists only from VBA 7 on `[obs]`; Office 2010 shipped VBA 7.0 `[hyp]`.
4. The `_VBA_PROJECT` version word `0x0097` `[obs]`: this is the field that
   identifies the compiler release which produced the cache, so it is the one that
   matters for decoding. Its mapping to a release stays `[hyp]` until a project
   from an independently known Office build confirms it.
5. Database level: file format "ACE 2007", subversion 0, 4096-byte pages, and a
   database property `AccessVersion` = `09.50` `[obs]`. These describe the file
   format and are long-lived markers, so they confirm "Access 2007 format or
   later" but do not distinguish releases.

Caveat: the cache is produced by whichever Access last compiled the project, which
can differ from the one that created the file. The version word, not the file
properties, decides how to decode it.

Sizes `[obs]`:

* Source lengths 350 to 60,292 characters; cache lengths (`MODULEOFFSET`
  values) 1,102 to 95,625 bytes; total cache about 691 KB.
* `_VBA_PROJECT` is 51,580 bytes (a large project-level cache, not the usual
  few bytes of header).
* Streams named `__SRP_<hex>`: exactly two per module, plus two for the
  project. Module stream names are scrambled by the host application.

How Access stores the project: the VBA storage hierarchy (`VBA/`, `PROJECT`,
`PROJECTwm`) is held as rows of a system table, one row per storage or stream,
with the stream bytes in a long-value column. The stream bytes are exactly the
stream contents: `dir` decompresses as an MS-OVBA container and all module
sources decompress `[obs; spec: MS-OVBA 2.4.1]`. Reading that table belongs to
the database container layer, not to this project. Whether `.accde` files use the
same layout with empty source parts is unchecked (no fixture).

## Findings on the module stream and cache

All `[obs]` unless marked. Cache means bytes `[0, MODULEOFFSET)` of the module
stream; source starts at `MODULEOFFSET` `[spec]`.

Confidence: high = holds in all 33 modules; medium = holds in the large
majority, with understood exceptions; low = pattern seen, not tested broadly.

| # | Finding | Confidence |
|---|---------|-----------|
| 1 | All module caches begin `01 16 01 00 06`; bytes 5..8 (a 32-bit little-endian field) vary by module and grow with module complexity; meaning unknown. | high (constant), field meaning unknown |
| 2 | The dword at cache offset 29 equals `len(cache) - 6` in every module, so the cache ends in a 6-byte trailer after a last section, and the header holds at least six section-like dwords at offsets 9, 13, 17, 21, 25, 29. Offset 21 is `0xFFFFFFFF` for exactly the standard modules and a real offset for all class modules. | high for the offset-29 equation; hyp for "section table" |
| 3 | The dwords at 13 and 17 are identical for modules of the same kind and size class, so the header part up to the first variable offset is shaped by module kind, not by code size. | medium |
| 4 | Comment text is stored verbatim in the cache, without the leading apostrophe, as a record `E0 00 | u16 a | u16 len | bytes[len]`. 98.9% of source comments match this shape exactly; the rest are comments on continuation lines or are split differently (likely a line-splitting artefact). Field `a` is 0 in most records, otherwise small values, and `0xFFFF` in about 1%; meaning unknown. | high for the record shape; low for `a` |
| 5 | String literals appear verbatim (code page bytes): 89% of literals of length >= 3. The misses may be literals on continued lines or stored differently. | medium |
| 6 | Identifier names appear as length-prefixed ASCII, not UTF-16. | high |
| 7 | A name record `B6 00 | u16 len | name[len]`, followed for form/report event handlers by `20 00 FF FF 21 00`, matched 29 procedures with no false positives (precision 100%, recall 9%). In standard modules the same shape is followed by a second `B6 00 len` record carrying the module name, preceded by `51 00 54 00`. `B6 00` introduces a length-prefixed name (hyp: a name or reference operand). | medium for shape, low for semantics |
| 8 | The cache size correlates with the number of non-comment source lines (0.98) and the source token count (0.98). Fit: roughly 9.6 bytes per token plus about 2 bytes per comment byte. So the cache is a dense per-token encoding rather than a compact bytecode, and comment text may be stored more than once (hyp). | medium |
| 9 | Each module owns exactly two `__SRP_*` streams starting `72 55`; the first correlates with code size, the second is small. Their role is unknown. | high for the count; role unknown |
| 10 | The `_VBA_PROJECT` cache section contains the reference descriptions (UTF-16 type-library strings), conditional-compilation names and ASCII names of modules, declared procedures and `Declare` targets. | medium |

### Alignment experiment (reproducible locally)

Source oracle: for each module, decompress the source at `MODULEOFFSET`; split
lines; separate the comment from the code with a quote-aware splitter; extract
string literals; list procedure headers with a pattern match. Then:

* comment text vs comment records: 98.9%;
* literals present as bytes: 89%;
* procedure names present as bytes: roughly 40% overall (high in standard
  modules with public procedures, low in forms; cause unknown, hyp: names are
  inlined only where referenced, and unreferenced event handlers are keyed by an
  index);
* name records vs procedures: precision 100%, recall 9%.

Procedure counts come from a header-matching pattern and are a lower bound.

## What is hypothesised and not yet tested

* The cache is a flat, token-per-record encoding of each source line with
  inline operands (names, literals, comments) and one or more tables of offsets
  in the header. `[hyp]`
* Offsets in the header point inside the same cache (all observed values are
  below the cache length, except `0xFFFFFFFF` sentinels). `[hyp]`
* `0xFFFF` words act as null references in several records. `[hyp]`
* The version word and the `01 16` prefix are a VBA release marker; the mapping
  will be derived by comparing projects from different Office versions. `[hyp]`
* Per-procedure tables exist with one entry per procedure. Twelve-byte records
  `u16 id | u16 0 | u32 offset | u32 0` seen after the header describe hidden
  members of document-class modules rather than user procedures (their count
  did not match the procedure count in the module examined). `[hyp]`

## Test material still needed

All as pairs (source version + compiled-only version), with the exact Office
build noted, and with known expected output (a small `Debug.Print`/`MsgBox`
transcript or a table, written down at creation time):

1. Windows + Office produced, minimal projects: an empty module, one `Sub`
   with `Debug.Print 1+2`, one of each statement kind (If/Select/For/Do/
   With/Call/GoTo/On Error), each literal type, each operator, each data type
   and `Declare` forms. Small enough to diff mentally.
2. The same project saved by (a) VBA 7.0 32-bit (Access 2010, the target),
   (b) VBA 7.1 32-bit, (c) VBA 7.1 64-bit, (d) VBA 6 (Office 2003/2007 era),
   and as (e) a project in a document (`vbaProject.bin` from an `.xlsm`) and
   one in Access. Note the exact Office build for each.
3. An `.accdb`/`.accde` pair of the same Access project (the compile-to-
   `accde` step strips the source), including at least one form with an event
   handler and one standard module.
4. Projects with referenced type libraries, plus the `.tlb`/`.olb` files
   (or their versions and GUIDs) so that member resolution can be studied.
5. For every fixture: a statement of license/provenance that allows local use
   (and ideally permits committing the file, which would let tests run in CI).
6. A project with `Option Explicit`, one with conditional compilation
   (`#If VBA7 Then`, `Win64`), one with classes (`Implements`, `WithEvents`,
   `Property` pairs) and one with a user-defined type and an enum.

Without Office the pairs in (1) and (2) cannot be generated here; this is the
main blocker for any decoder that can be validated.

## Planned crate layout

* `crates/formats/vba-pcode` (new): read-only access helpers over the
  performance cache: parse the cache header, expose the raw bytes of sections
  and records, scan for length-prefixed names and literals. No execution.
  Depends on `vba-project` and `vba-format` for module bytes and on nothing
  Windows specific. The skeleton is deliberately **not** created in phase 0:
  with only the single generic observation above, a crate would be an empty
  shell with an unstable surface, and adding it later costs one line in the
  lockfile.
* `crates/language/vba-pcode-ir` (new): lowers decoded records into the
  existing `vba-ir`, so the shared `vba-vm` executes both source-derived and
  cache-derived programs through one path.
* Execution stays on `vba-vm`; no second interpreter. Name resolution
  (references, type libraries) is shared with the source pipeline.
* A source-vs-cache differential harness (the oracle) is the first
  consumer: for fixtures with both, the cache-derived IR must agree with the
  source-derived IR modulo comments and formatting.

## Phase gates and success metrics

| Phase | Gate | Metric |
|-------|------|--------|
| 0 | Fixture access, container extraction path, first structural facts, docs | done when this document exists; at least one fixture with source and cache |
| 1 | Header and section map: parse the header, locate every section, enumerate records without interpreting them | 100% of module caches parse into a gap-free sequence of known-size regions (no unexplained bytes in sections, tolerated in an `unknown` bucket below a stated size); property test: re-encode the region map |
| 2 | Token/record decoder for expressions and statements on the reference project; opcode table derived by diffing against source | per-record round trip: decoded records re-render the source statements of the oracle (modulo whitespace and case) for >= 95% of statements in the reference project |
| 3 | Same on the other versions and on 64-bit (needs new test material) | same metric, on each added version |
| 4 | Lower decoded procedures to `vba-ir`, run a small program on `vba-vm` | outputs of the compiled-only project equal the source version on a conformance subset; errors raised for unsupported records, never silent |
| 5 | Compiled-only `.accde` with referenced libraries | the first real `.accde` fixture runs end to end |

Policy gates: no phase merges unless its metric is measured by a committed
test that uses only committed (non-extracted) synthetic or user-approved
fixtures; tests that need extracted projects stay ignored and are run locally.
