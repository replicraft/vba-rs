# p-code decoder: status

Crate: `crates/formats/vba-pcode` (library only, no CLI wiring). Scope reached:
phase 1 (structure) and phase 2 (opcode decoder and disassembler) of the gates in
[pcode.md](pcode.md), and phase 3 for VBA 6 and 7 on 32-bit and 64-bit Windows
(verified on 46 further projects, [pcode-phase3.md](pcode-phase3.md) section 9; VBA 5
and VBA 3 have no material and stay unverified). A lowering of the
verified subset to `vba-ir` exists in `crates/language/vba-pcode-ir`; its coverage on
the first local project (99 of the 139 standard-module procedures with a body, 71 %, and 90
of the 205 procedures with a body of class, document and designer modules, 44 %) is
measured in [pcode-phase3.md](pcode-phase3.md) sections 4 and 8, and on the 46 other
projects in section 9.

Specifications: [pcode-format.md](pcode-format.md), [pcode-opcodes.md](pcode-opcodes.md),
[pcode.md](pcode.md) and the public MS-OVBA text. Tags as in those documents: `[spec]`
stated by the documents, `[obs]` observed in compiled projects with the source of the
same modules side by side (a first project of VBA 7.0 x86 with 33 modules, then 46
projects of VBA 6 and 7, 32-bit and 64-bit), `[unverified]` implemented from the text
without material. Nothing from the observed material is committed; the tests that need
it are `#[ignore]`d and return early when it is absent. Observed corrections to the specifications are
recorded in the specifications themselves, not here.

## What is implemented

| Piece | Where | Status |
|---|---|---|
| Layout selection from the `_VBA_PROJECT` version word and `PROJECTSYSKIND` (`Layout::select`) | `layout.rs` | the bitness comes from `PROJECTSYSKIND` alone; verified for the (family, bitness, version word) combinations of `VERIFIED_LAYOUTS`: VBA6 x86 `0x79`, `0x88`; VBA7 x86 `0x97`, `0xAF`, `0xB2`; VBA7 x64 `0x9A`, `0xA6`, `0xB2`, `0xB5`. VBA5 and combinations not seen (for example `0xB5` on x86) are `[unverified]`. Version `0xFFFF`, 16-bit, Macintosh and unknown platform kinds are typed errors; big-endian module streams are a typed error, and so is a module whose platform word disagrees with the layout |
| Project-stream identifier walk (`parse_identifiers`, `ProjectCache`) | `project_stream.rs` | verified (entries announced = entries read in all 46 further projects, versions `0x79` to `0xB5`) |
| Module-stream header, table locations (`TableLocations`) | `module.rs` | VBA6/7, x86 and x64 verified; VBA5 `[unverified]` |
| Line table (`LineTable`) and per-line code slots | `lines.rs` | verified |
| Region map (`ModuleCache`): gap-free tiling of the whole cache with a named kind per region | `cache.rs` | verified (gap-free tiling of 492 of 492 further modules, all layouts) |
| Reference list and member table sections | `sections.rs` | verified (layouts in pcode-format.md 4.4) |
| Records: function, argument, variable, user-defined type, type descriptor, declaration, object entry | `records.rs` | VBA6/7, x86 and x64 verified |
| Reachable-record map of the indirect table (`RecordMap`) | `recordmap.rs` | verified (record sizes for VBA6/7, x86 and x64; the shape classifier `IndirectMap` still covers VBA7 x86 only) |
| Identifier resolution (`Resolver`, `Name`) and the confirmed built-in table | `names.rs` | user identifiers verified for VBA6 (offset 0), VBA7 x86 (4) and x64 (7); built-ins: see below |
| Canonical opcode table (264 entries) with operand shapes | `opcode.rs` | generated from the opcode document |
| Opcode translation to canonical numbering | `layout.rs` | x86 VBA6/7 and x64 verified; VBA5 `[unverified]`; VBA3 not implemented |
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

After the corrections of phase 3 (type references scaled by the pointer size, entry 0
valid, `obj` operands scaled on x86, more built-in names) the same project gives 9,190
of 9,226 statements consistent (99.61 %; strict 9,114, 98.79 %), 5 forward failures and
35 reverse misses; the structure, record and differential numbers are unchanged
(differential: 189 of 430 procedures lowered, 0 mismatches).

## Built-in names

`names::CONFIRMED_BUILTINS` holds 29 index/spelling pairs (19 from the first project, 10 from the VBA6 and 64-bit material, pcode-format.md 5.3); every other built-in index is
`Name::UnknownBuiltin` and is never guessed. Evidence kind for all entries: in the
observed material every instruction or type field using the index sits on a source line
that contains the spelling as a token, and no other token is shared by all such lines
(index 44 is also constrained by the alphabetical order of its neighbours, because the
keywords `As` and `Dim` share its lines). The test
`phase2_decode_and_consistency` re-checks every use (all agree). Observations per entry
range from 1 to 107. The indices fall in the lower part of the table and in
alphabetical order, as the format document expected; no confirmed entry lies in the
range where the VBA7 adjustment of format 5.2 applies, so that adjustment stays
unverified (pcode-format.md 5.2): the only operand seen at or above it in all the material is a single member name. In the 46 further projects every built-in name operand that is not one of the 29 entries is left as `UnknownBuiltin`; none of them contradicts an entry.

## Unknowns

* The built-in name table beyond the 29 entries; library-qualified type names (the object
  entry has bit 0 of the word at +2 set and no names that were found).
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
* 97 opcodes were never seen in any layout (129 in the first project alone; the 32 more
  seen in the 46 further projects include `Implements`, events, `Debug.Assert`,
  conditional compilation and `ReDim` of members). Unseen include `PROC_CONTEXT`,
  `PROC_HEADER_ALT`, the three 8-byte literals, `Single`, `Currency` and octal literals,
  the `Bang` families, `Rem`, most file I/O, graphics and `Print` list parts, computed
  `GoTo`/`GoSub`, `Stop`; their operand shapes come from the document only. The stack semantics of every opcode are
  those of the opcode document (inferred); nothing here verifies them.
* Text and string handling uses the project code page bytes as stored; names are
  presented with one character per byte.

## What blocks phase 3 and phase 4

1. Test material. The first project (VBA 7.0 x86) was extended by 46 projects of VBA 6
   and 7, 32-bit and 64-bit (pcode-phase3.md section 9), which verified the x64 and
   VBA6 offsets, opcode translations and identifier adjustments. Still without
   material: VBA5 and VBA3, big-endian (Macintosh) caches, 32-bit builds with version
   words other than `0x97`, `0xAF`, `0xB2`, 64-bit builds with other words than `0x9A`,
   `0xA6`, `0xB2`, `0xB5`, the 8-byte integer literals and `PROC_CONTEXT`, and the
   built-in adjustment above index `0xC3`.
2. The built-in name table. Without it, calls to many library members cannot be named;
   the 29 confirmed entries are not enough to disassemble arbitrary programs. More
   material (or an authoritative list) is needed; deriving it from limited material is slow
   because most built-ins have dedicated opcodes.
3. Semantics for lowering. The decoder yields an operand-complete instruction stream,
   but the stack effects and the order in which the expression instructions are
   emitted are inferred. Lowering to `vba-ir` needs a differential test of the
   lowered program against the source-derived one, which in turn needs the 97 unseen
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
VBA_PCODE_CORPUS=<directory of project files or documents> \
  cargo test -p vba-pcode --test corpus -- --ignored --nocapture
VBA_PCODE_CORPUS=<directory> cargo test -p vba-pcode-ir --test corpus --test differential \
  -- --ignored --nocapture
```

The second command prints the numbers above when the environment variable names a
local project file; without it the tests return early.
