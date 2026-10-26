# p-code decoder: status

Crate: `crates/formats/vba-pcode` (library only, no CLI wiring). Scope reached:
phase 1 (structure) and phase 2 (opcode decoder and disassembler) of the gates in
[pcode.md](pcode.md). Phase 3 (other versions and 64-bit) is not started. A lowering of the
verified subset to `vba-ir` exists in `crates/language/vba-pcode-ir`; its coverage on
the local project (99 of the 139 standard-module procedures with a body, 71 %, and 90
of the 205 procedures with a body of class, document and designer modules, 44 %) is
measured in [pcode-phase3.md](pcode-phase3.md) sections 4 and 8.

Specifications: [pcode-format.md](pcode-format.md), [pcode-opcodes.md](pcode-opcodes.md),
[pcode.md](pcode.md) and the public MS-OVBA text. Tags as in those documents: `[spec]`
stated by the documents, `[obs]` observed on VBA 7.0 x86 material (33 modules, source
and cache side by side), `[unverified]` implemented from the text without material.
Nothing from the observed material is committed; the tests that need it are `#[ignore]`d
and return early when it is absent. Observed corrections to the specifications are
recorded in the specifications themselves, not here.

## What is implemented

| Piece | Where | Status |
|---|---|---|
| Layout selection from the `_VBA_PROJECT` version word and `PROJECTSYSKIND` (`Layout::select`) | `layout.rs` | VBA 7.0 x86 (word `0x0097`) verified; VBA5, VBA6, other VBA7 words and x64 `[unverified]`. Version `0xFFFF`, 16-bit, Macintosh and unknown platform kinds are typed errors; big-endian module streams are a typed error |
| Project-stream identifier walk (`parse_identifiers`, `ProjectCache`) | `project_stream.rs` | verified on the target |
| Module-stream header, table locations (`TableLocations`) | `module.rs` | VBA6/7 x86 verified; x64 and VBA5 `[unverified]` |
| Line table (`LineTable`) and per-line code slots | `lines.rs` | verified |
| Region map (`ModuleCache`): gap-free tiling of the whole cache with a named kind per region | `cache.rs` | verified (target layout) |
| Reference list and member table sections | `sections.rs` | verified (layouts in pcode-format.md 4.4) |
| Records: function, argument, variable, user-defined type, type descriptor, declaration, object entry | `records.rs` | target layout verified; x64 offsets `[unverified]` |
| Reachable-record map of the indirect table (`RecordMap`) | `recordmap.rs` | verified (target layout) |
| Identifier resolution (`Resolver`, `Name`) and the confirmed built-in table | `names.rs` | user identifiers verified; built-ins: see below |
| Canonical opcode table (264 entries) with operand shapes | `opcode.rs` | generated from the opcode document |
| Opcode translation to canonical numbering | `layout.rs` | x86 VBA6/7 verified; x64, VBA5 `[unverified]`; VBA3 not implemented |
| Line decoder (`decode_line`) | `decode.rs` | verified |
| Typed disassembly and its text format (`Disassembly`, `Disassembler`) | `disasm.rs` | verified |

Every parser is bounds-checked and returns `PcodeError`; hostile input is covered by
property tests (`tests/hostile.rs`) and the layout tests use hand-built bytes
(`tests/common/synth.rs`, `tests/synthetic.rs`).

### Text format

`Disassembly` renders one instruction per row,
`L<line> +<offset> <LABEL> [mod=<n>] <operands>`, preceded by a three-line header.
The format is documented in the module comment of `disasm.rs` and pinned by a test on
synthetic bytes.

## Metrics on the observed material (33 modules, 691,204 cache bytes)

Phase 1.

| Metric | Result |
|---|---|
| Caches that decompose into a gap-free sequence of regions | 33 of 33 |
| Bytes in regions whose structure is interpreted | 669,393 (96.84 %) |
| Bytes in delimited regions with uninterpreted content | 21,811 (3.16 %), 555 to 1,041 per module |
| Bytes in no identified structure | 0 |
| Lowest interpreted share of a module | 40.9 % (the smallest module, 1,102 bytes) |
| Code area: bytes claimed by no line / by two lines | 0 / 0 in every module |
| Line count (cache vs. logical source lines after the attribute lines) | 33 of 33 modules agree |
| Procedure headers: cache lines carrying `PROC_HEADER` vs. source lines that are `Sub`/`Function`/`Property`/`Declare` headers | 430 vs. 430, the same line indexes in 33 of 33 modules |
| Procedure records: name agrees with the header line / kind agrees | 430 of 430 / 430 of 430 |
| Argument records: name agrees with the header line | 627 of 627 |
| Variable records: name agrees / intrinsic type id agrees with the `As` token | 1,094 of 1,094 / 642 of 642 |
| Identifier table: entries announced vs. read | 2,512 vs. 2,512 (the walk ends 1,107 bytes before the end of the 51,580-byte stream) |

"Interpreted" is a region-level notion: the region is located, delimited and its
internal structure is known to the degree listed in the implemented table. It is not a
claim that every byte inside is understood. The honest content-level figures:

* The indirect table is 36 % of the cache (246,888 bytes). Records reachable from the
  code (functions 430, arguments 701, variables 1,094, user types 24, type descriptors
  174) cover 78,176 bytes, 31.7 % of it. They do not overlap and do not conflict in
  kind. The remaining 68 % of the table (large runs and many 16-byte filler records) is
  not understood.
* The object table entries are located but only partly interpreted (pcode-format.md 6.1).
* The line records have two fields (4 bytes at +0, 2 bytes at +6) of unknown meaning.
* The fixed-size blocks listed under unknowns below are delimited but opaque.

Phase 2.

| Metric | Result |
|---|---|
| Lines with code | 9,226 (all lines that have code decode); 0 lines stop decoding |
| Instructions decoded | 23,559 |
| Statement consistency score (definition below) | 9,175 of 9,226 = **99.45 %** (lenient); 9,090 = 98.53 % (strict, no unresolved name operand) |
| Operand level | names 11,549 of 11,549; string literals 1,088 of 1,088; comments 2,642 of 2,646; integer literals 1,969 of 1,970 |
| Opcodes seen | 135 of 264 canonical opcodes; 129 never occur in the observed material (listed by the local test) |
| Opcodes with a decoder | 264 of 264 (operand shapes from the opcode document); the 129 unseen ones have not been exercised |
| Unresolved operands | built-in name indices with no confirmed spelling (see below); 0 identifier indices out of range |

Consistency score. A *statement* is a logical source line (continuations joined,
leading `Attribute` lines dropped) with at least one significant token or a comment.
It is consistent when (forward) every comment operand equals the source comment, every
string operand equals a string literal of the line, every integer operand equals a
numeric literal of the line, and every resolved name operand equals an identifier or
keyword token of the line; and (reverse) the comment, every string literal and every
identifier that is a project identifier occurs among the operands. The strict score
additionally requires that no name operand is unresolved. Failures left: 5 statements
with a forward failure (a synthetic zero argument of the graphics `Line` statement and
four comments on continued lines) and 50 reverse misses (type names of module-local user
types, library-qualified type names, and two comment forms), all listed by the local
test.

## Built-in names

`names::CONFIRMED_BUILTINS` holds 19 index/spelling pairs; every other built-in index is
`Name::UnknownBuiltin` and is never guessed. Evidence kind for all entries: in the
observed material every instruction or type field using the index sits on a source line
that contains the spelling as a token, and no other token is shared by all such lines
(index 44 is also constrained by the alphabetical order of its neighbours, because the
keywords `As` and `Dim` share its lines). The test
`phase2_decode_and_consistency` re-checks every use (all agree). Observations per entry
range from 1 to 107. The indices fall in the lower part of the table and in
alphabetical order, as the format document expected; no confirmed entry lies in the
range where the VBA7 adjustment of format 5.2 applies, so that adjustment stays
unverified (pcode-format.md 5.2). 11 name operands and a number of type names use unconfirmed indices.

## Unknowns

* The built-in name table beyond the 19 entries; library-qualified type names; the type
  name of user types declared in the same module (the descriptor has no name).
* Contents of the opaque blocks: the header between the offset fields and the
  declaration table (30 bytes), the block after the declaration table (module info,
  173 or 189 bytes plus module-dependent extras), the object-table header (138 bytes),
  the block after the object table (10 or 90 bytes plus extras), the indirect-table
  header (10 bytes), the 126-byte block after the indirect table, the 60-byte
  pre-line-table block, the 8 bytes after the code, and 68 % of the indirect table.
* The four unknown line-record bytes at +0 and the two at +6; the word at +2 and the 10
  bytes after the line records.
* The meaning of the member-table record dwords, of the extra word of `VAR_DECL`
  (values 12 to 34 seen), of flag 0x40 of arguments, and of the argument records
  without a resolvable name (74 in the sample; they are the result records of
  functions and property getters in class-like modules, pcode-format.md 6.6).
* 129 opcodes were never seen (including `PROC_CONTEXT`, `PROC_HEADER_ALT`, event,
  `Implements`, preprocessor, file I/O and most operators on `Variant` forms); their
  operand shapes come from the document only. The stack semantics of every opcode are
  those of the opcode document (inferred); nothing here verifies them.
* Text and string handling uses the project code page bytes as stored; names are
  presented with one character per byte.

## What blocks phase 3 and phase 4

1. Test material. Only VBA 7.0 x86 material was available. Phase 3 needs at least a VBA 7.1 32-bit, a 64-bit and a VBA 6 sample, each
   with its source, to verify the x64 offsets, the opcode translations and the
   identifier adjustments marked `[unverified]`.
2. The built-in name table. Without it, calls to many library members cannot be named;
   the 19 confirmed entries are not enough to disassemble arbitrary programs. More
   material (or an authoritative list) is needed; deriving it from limited material is slow
   because most built-ins have dedicated opcodes.
3. Semantics for lowering. The decoder yields an operand-complete instruction stream,
   but the stack effects and the order in which the expression instructions are
   emitted are inferred. Lowering to `vba-ir` needs a differential test of the
   lowered program against the source-derived one, which in turn needs the 129 unseen
   opcodes exercised by purpose-made projects (the test-material list in pcode.md).
4. Declarations. Arrays (bounds), fixed-length strings, enums, `Declare` details beyond
   the entry point, event and `Implements` records, and the module-local user type
   names are not resolved. The indirect table remainder may hold part of this. The
   value of a constant is located (the expression in front of its `VAR_DECL`,
   [pcode-opcodes.md](pcode-opcodes.md) 4.18); the lowering generates the ones that are a
   single literal.
5. References and members. Early-bound member resolution needs the type libraries; the
   stream carries names only (no dispatch ids), so it belongs to the host/automation
   layer.
6. Compiled-only projects (`.accde`) have not been seen; the cache is assumed to have
   the same structure without source.

## Running the tests

```text
cargo test -p vba-pcode                                      # synthetic and property tests
VBA_PCODE_PROJECT=<path to an extracted project stream set> \
  cargo test -p vba-pcode --test real_project -- --ignored --nocapture
```

The second command prints the numbers above when the environment variable names a
local project file; without it the tests return early.
