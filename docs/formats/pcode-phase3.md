# Compiled code, phase 3: evidence, indirect table, lowering

Status of the third research step on the compiled-code cache. Companion to
[pcode-status.md](pcode-status.md) (phases 1 and 2), [pcode-format.md](pcode-format.md)
and [pcode-opcodes.md](pcode-opcodes.md). Nothing is wired into the facade or the CLI.

Material: the same single local project as before (VBA 7.0, 32-bit, 33 modules with
source and cache). Tests that need it read its path from `VBA_PCODE_PROJECT`, are
`#[ignore]`d and return early when it is unset. Only numbers are recorded here.

## 1. Built-in names

`names::CONFIRMED_BUILTINS` gained one pair (20 now). An index is accepted only when
every use sits on a source line that contains the spelling and no other token is shared
by all uses; the accepted one has 9 uses.

| Metric | Before | After |
|---|---|---|
| Built-in name operands (instructions and names inside records) | 396 | 396 |
| Resolved to a spelling | 383 (96.7 %) | 392 (98.99 %) |
| Left as `UnknownBuiltin` | 13 | 4 |

The four left are indexes with a single use each. For three of them exactly one token
also fits the alphabetical position between the confirmed neighbours; that is not
repeated evidence and they stay unknown. The adjustment of format 5.2 for indexes above
0xC3 is still unexercised.

## 2. Indirect table

`IndirectMap` classifies the whole table by what is reachable from code, by the first
word and pointer fields of a record, and by runs of pointers.

| Metric (246,888 bytes, 33 modules) | Before | After |
|---|---|---|
| Records reachable from code | 78,176 B (31.7 %) | 78,168 B (31.7 %) |
| Further records recognised by shape | 0 | 68,888 B (27.9 %) |
| Pointer arrays and fill | 0 | 57,212 B (23.2 %) |
| Understood in total | 31.7 % | 82.7 % |
| Unknown | 68.3 % | 42,620 B (17.3 %) |
| Per module, understood | not measured | min 14.3 %, median 83.0 %, max 90.0 % |

Shapes found: a 24-byte symbol record with the layout of a variable record but no name
(name word `0xFFFE` in 2,542 of 2,735; 2,730 recognised, 65,520 B), 103 named
variable-layout records, 1 argument record, 108 type descriptors, 371 arrays of
pointers and `0xFFFFFFFF` (57,024 B) and 2 runs of fill. Evidence that the shape
findings are not coincidences: 2,929 of 2,942 are the target of a pointer-looking dword
elsewhere in the same table. The links used are `Function+4` (next function),
`Function+32` (a per-procedure table of pointers), `Variable+4` and `UserType+4/+8`.
The reachable figure moves by 8 bytes because two descriptors overlap in one module.
Left unknown: the larger records that begin `0x8002`/`0x8302`/`0xA082`, the constant
text records referenced from variable records, and the headers of the pointer tables.

## 3. Opcode evidence

`Opcode::evidence()` returns `Verified`, `Inferred` (seen, not verified) or `Unseen`.
Verified means operand shape, operand order and stack effect were confirmed by
replaying the instructions of a statement on an expression stack and comparing the
result with the source expression parsed under the language operator precedence (or,
for markers without stack effect, by agreement between the lines carrying the opcode
and the keyword starting the source line).

| Check | Compared | Agree |
|---|---|---|
| Assignment right-hand sides and target | 1,098 | 1,097 |
| Block `If` conditions | 237 | 237 |
| `ElseIf` | 11 | 11 |
| Single-line `If` conditions | 28 | 27 |
| `Do`/`Loop`/`While` conditions | 20 | 20 |
| `Select Case` subjects | 2 | 2 |
| `For` headers (variable, start, end, step in order) | 35 | 35 |
| `Case` lists (values, ranges, `Is <>`/`Is =`) | 317 | 317 |
| `Debug.Print` lists with separators | 15 | 15 |
| Call statements (argument count, `Call` keyword) | 137 | 137 |
| Marker opcodes on line keywords (41 predicates) | 41 | 41, no unexplained disagreement |

Residual disagreements are accounted for: one `StrComp` operand not modelled by the
check, and a 32-bit hexadecimal literal whose stored value is 65,535 where the
language gives -1 for the same digits. New facts: the word of `LBound`/`UBound` is the
number of dimension arguments after the array (the instruction pops it plus one);
`InStr` has one opcode per argument count (2, 3, 4).

Result: 86 opcodes Verified, 49 Inferred (seen, not verified), 129 Unseen (no change
in the seen total of 135). Left inferred although seen: `ERR_HANDLER_SET`,
`CONVERT_TO_TYPE`, member access (`READ_DOT` family), `WITH`, `For Each`, `New`,
arrays (`ReDim`), `Set`, graphics and file statements.

## 4. Lowering (`crates/language/vba-pcode-ir`)

`lower_project(project_stream, sys_kind, modules)` returns an `IrProgram` that passes
`vba_ir::verify` plus one report per procedure: `Lowered`, or `Unsupported { opcode,
procedure, line, reason }` naming the first thing outside the subset. Procedures that are
not lowered have a body that raises an error and must not be run; a procedure calling
one is itself unsupported. Only the verified layout (VBA 7.0 x86) is accepted.

The IR refers to a `vba_sema::Program`. That program is obtained by analysing a
declaration-only text generated from the cache records (headers, parameters, local
declarations; empty bodies); no source text is read. The bodies are then replaced by the
lowered ones. `vba-sema` is therefore a dependency (the IR types need it).

Subset: standard modules; `Sub`/`Function` with plain scalar parameters and locals
(`ByVal`/`ByRef`); every opcode verified; literals (Integer, Long, Double, String,
Boolean/Null/Empty), arithmetic, comparison, logic, concatenation, `Like`, unary
operators; assignment with the same conversions as the source lowering; block and
single-line `If`/`ElseIf`/`Else`; `For` with `Step`; `Do`/`Loop` in all forms;
`While`/`Wend`; `Select Case` with values and ranges; `Exit` for/function/sub; calls to
other procedures of the project (including recursion and `ByRef` arguments) and to
library functions by resolved name (`Abs Int Sgn Len InStr`, plus names resolved
through the library registry); `Debug.Print`.

| Metric on the local project | Result |
|---|---|
| Procedures in the caches | 430 |
| In standard modules | 201 (62 are `Declare`) |
| Lowered, verifier-clean | 63 (45 % of the 139 standard-module procedures with a body) |
| Unsupported: procedure of a class-like module | 229 |
| Unsupported: `Declare` | 62 |
| Unsupported: `On Error` (`ERR_HANDLER_SET`) | 18 |
| Unsupported: `Optional`/`ParamArray` parameter | 14 |
| Unsupported: name that is not a local, procedure or library function | 9 |
| Unsupported: local of non-scalar type / non-plain declaration | 8 / 7 |
| Unsupported: parameter that is not a plain scalar | 7 |
| Unsupported: other (single occurrences: `Set`, `ReDim`, `Option Base`, `CInt`-family, hex literals, indexing, unknown callee) | 13 |

## 5. Differential tests

Local project (`tests/differential.rs`): the lowered cache program and a program
compiled from the original text of the same procedures run on `vba-vm` with
deterministic argument sets (up to 6 per procedure, diagonal selection from per-type
pools), comparing result, `Debug.Print` output and error.

| Metric | Result |
|---|---|
| Procedures lowered / executed | 63 / 63 |
| Runs | 306 (50 end in a run-time error on both sides, identically) |
| Mismatches | 0 |

The harness was checked for sensitivity by swapping the operand order of the binary
operators: it reports mismatches immediately.

Synthetic (`tests/synthetic.rs`, no project needed): 12 tests assemble caches from the
documented layouts and run them with hand-worked expectations; they cover the
operators and operand order, comparison and logic, text, `If`/`ElseIf`/`Else` and
single-line `If`, `For`/`Step`/`Exit For`, all `Do`/`While` forms, `Select Case`,
calls/recursion/`ByRef`, `Debug.Print`, library calls, typed overflow (error 6),
unsupported reporting, and a property test that mutated and truncated caches never
panic.

## 6. What blocks the rest

* `On Error`: needs the semantics of `ERR_HANDLER_SET` verified and the
  statement resume points of the IR reproduced (18 procedures, mostly pure date
  functions).
* Optional parameters: the default value location in the argument record is unknown
  (14 procedures).
* Module-level variables and constants: the cache gives the declaration but the value
  of a constant is not located (9+ procedures).
* Class-like modules: 229 procedures need instances, members and the object model.
* `CONVERT_TO_TYPE` and the other inferred opcodes need more statements that exercise
  each modifier.
* Arrays, fixed-length strings and user types in declarations (bounds are not
  located; id 29 carries no name).
* Hexadecimal literals: how the Integer/Long distinction is stored for values below
  0x10000.

## 7. Test material that would unblock other versions and bitness

* A project compiled by VBA 7.1 32-bit, one by VBA 7.1 64-bit and one by VBA 6, each
  with the matching source: to verify the `[unverified]` offsets, the opcode
  translations and the identifier adjustment. The lowering refuses every layout that is
  not the verified one until then.
* Small purpose-made modules, each exercising one family with its source: `On Error`
  forms, `Optional` with defaults, module `Const`/`Static`/`Dim`, arrays, `Date` and
  `Currency` literals, `Case Is`, `Do`/`Loop` with `Exit Do`, all `CONVERT_TO_TYPE`
  modifiers, `With`, `For Each`, `New`, `Set`, `Property` procedures.
* A compiled-only project (no source) to confirm that the cache has the same structure
  when the source part is empty.
