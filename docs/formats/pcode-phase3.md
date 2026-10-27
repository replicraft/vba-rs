# Compiled code, phase 3: evidence, indirect table, lowering

Status of the third research step on the compiled-code cache. Companion to
[pcode-status.md](pcode-status.md) (phases 1 and 2), [pcode-format.md](pcode-format.md)
and [pcode-opcodes.md](pcode-opcodes.md). Nothing is wired into the facade or the CLI.

Material: sections 1 to 8 describe the first local project (VBA 7.0, 32-bit, 33 modules
with source and cache), which remains the baseline. Tests that need it read its path
from `VBA_PCODE_PROJECT`, are `#[ignore]`d and return early when it is unset. Section 9
describes 46 further projects of VBA 6 and 7, 32-bit and 64-bit; their tests read a
directory from `VBA_PCODE_CORPUS` and behave the same. Only numbers are recorded here.

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

Result of the first project: 86 opcodes Verified, 49 Inferred (seen, not verified), 129
Unseen (no change in the seen total of 135). After the 46 further projects (section 9):
86 Verified, 81 Inferred, 97 Unseen; the 32 newly seen opcodes decode to the exact line
length everywhere they occur, their meaning is not verified. Left inferred although seen: `ERR_HANDLER_SET`,
`CONVERT_TO_TYPE`, member access (`READ_DOT` family), `WITH`, `For Each`, `New`,
arrays (`ReDim`), `Set`, graphics and file statements.

## 4. Lowering (`crates/language/vba-pcode-ir`)

`lower_project(project_stream, sys_kind, modules)` returns an `IrProgram` that passes
`vba_ir::verify` plus one report per procedure: `Lowered`, or `Unsupported { opcode,
procedure, line, reason }` naming the first thing outside the subset. Procedures that are
not lowered have a body that raises an error and must not be run; a procedure calling
one is itself unsupported. Only the verified layouts (`vba_pcode::VERIFIED_LAYOUTS`: VBA 6
and 7, 32-bit and 64-bit; section 9) are accepted.

The IR refers to a `vba_sema::Program`. That program is obtained by analysing a
declaration-only text generated from the cache records (headers, parameters, local
declarations; empty bodies); no source text is read. The bodies are then replaced by the
lowered ones. `vba-sema` is therefore a dependency (the IR types need it).

Subset (class-like modules and module-level names: section 8; types, objects and members:
section 10): standard modules; `Sub`/`Function` with plain scalar parameters (including
`Optional` ones, with or without a default value) and locals (`ByVal`/`ByRef`); every
opcode verified; literals (Integer, Long, Double, String, Boolean/Null/Empty, Date,
hexadecimal Integer and Long), arithmetic, comparison, logic, concatenation, `Like`,
unary operators; assignment with the same conversions as the source lowering; block and
single-line `If`/`ElseIf`/`Else`; `For` with `Step`; `Do`/`Loop` in all forms;
`While`/`Wend`; `Select Case` with values, ranges and `Is =`/`Is <>`; `Exit`
for/function/sub; calls to other procedures of the project (including recursion,
`ByRef` arguments and omitted optional arguments) and to library functions by resolved
name (`Abs Int Sgn Len InStr` with 2, 3 or 4 arguments, `CLng`, `CStr`, plus names
resolved through the library registry and the run-time functions such as `MsgBox`);
library constants (`vbCrLf`, ...); `Debug.Print`; `On Error Resume Next`,
`On Error GoTo label`, labels, `Resume label` and property reads on `Err`.

Rules added in the second round, each pinned by the differential run (section 5) and by
synthetic caches, not by a bare reading of the opcode document:

* Default values of `Optional` parameters sit on the header line in front of the
  `PROC_HEADER` instruction, after a `CONST_EXPR_WITH_CALL` marker, in reverse argument
  order. Option bit 0x400 of the argument record marks an argument that has one: the
  number of such arguments equals the number of expressions on 430 of the 430 header
  lines. (The argument-count byte of the function record is not used for this: it has a
  flag in bit 0x80 on six headers.)
* Opcodes that the opcode table still lists as inferred but that the lowering accepts
  in one form: `ERR_HANDLER_SET` with modifier 1 (`Resume Next`) or 0 (`GoTo label`),
  `ERR_RESUME` with modifier 0, `LABEL_NAMED`, `READ_DOT` on the `Err` object,
  `CONST_INT16_HEX`, `CONST_DATE`, `CONVERT_TO_TYPE` with modifier 3 (`CLng`) or 8
  (`CStr`). The opcode table is unchanged; the acceptance list is in `lower.rs`.
* A 32-bit hexadecimal literal is a `Long` whatever its value (a literal with up to four
  digits and no `&` suffix is an `Integer` and has the 16-bit form).
* Under `On Error`, a simple statement resumes right after itself; the header of an
  `If`, `For`, `Do` or `While` resumes at the start of its body; a `Select Case`
  resumes at its first case body; the statement of a `Resume` or `Exit` has no resume
  point. This mirrors the source lowering; a statement marker starts a new statement of
  its own.
* Modifier bit 0x10 of a local `VAR_DECL` only adds a layout word (seen on declarations
  whose `As` is aligned with extra spaces); it does not make the declaration anything
  but a plain `Dim`.
* `InStr` with three arguments is `start, string1, string2`, the way the source binding
  reads three arguments.

Coverage on the local project, before and after the second round.

| Metric on the local project | Round 1 | Round 2 |
|---|---|---|
| Procedures in the caches | 430 | 430 |
| In standard modules | 201 (62 are `Declare`) | 201 (62 are `Declare`) |
| Lowered, verifier-clean | 63 (45 % of the 139 standard-module procedures with a body) | **95 (68 %)** |
| Unsupported: procedure of a class-like module | 229 | 229 |
| Unsupported: `Declare` | 62 | 62 |
| Unsupported: local of a type that is not a plain scalar (objects, user types, arrays, fixed-length strings) | 8 | 14 |
| Unsupported: name that is not a local, a procedure or a library name (module-level variables, objects) | 9 | 9 |
| Unsupported: parameter that is not a plain scalar | 7 | 7 |
| Unsupported: `On Error` | 18 | 0 |
| Unsupported: `Optional`/`ParamArray` parameter | 14 | 1 (`ParamArray`) |
| Unsupported: `Static` locals | not separated | 2 |
| Unsupported: `Mid$` assignment, `ReDim`, `Option Base`, `Set`, `LBound`, indexing, unknown callee, assignment to a module-level name | 13 | 11 |

The second-round numbers were measured with
`VBA_PCODE_PROJECT=<file> cargo test -p vba-pcode-ir --test real_project -- --ignored --nocapture`.
Procedures that moved from one reason to another between the rounds (for example an
`Optional` procedure that then needs an object-typed local) are counted under the reason
that stops them now.

## 5. Differential tests

Local project (`tests/differential.rs`): the lowered cache program and a program
compiled from the original text of the same procedures run on `vba-vm` with
deterministic argument sets (up to 6 per procedure, diagonal selection from per-type
pools; for procedures with `Optional` parameters also the sets with each run of
trailing optional arguments omitted), comparing result, `Debug.Print` output and error.

| Metric | Round 1 | Round 2 |
|---|---|---|
| Procedures lowered / executed | 63 / 63 | 95 / 95 |
| Runs | 306 (50 end in a run-time error on both sides, identically) | 509 (98 end in a run-time error on both sides, identically) |
| Mismatches | 0 | 0 |

The harness was checked for sensitivity twice: swapping the operand order of the binary
operators, and dropping the `Resume Next` points of simple statements (20 mismatches
reported at once).

Synthetic (`tests/synthetic.rs`, no project needed): 17 tests assemble caches from the
documented layouts and run them with hand-worked expectations; they cover the
operators and operand order, comparison and logic, text, `If`/`ElseIf`/`Else` and
single-line `If`, `For`/`Step`/`Exit For`, all `Do`/`While` forms, `Select Case`,
calls/recursion/`ByRef`, `Debug.Print`, library calls, typed overflow (error 6),
`Optional` parameters (with and without defaults, omitted at the call and at the run
call, `IsMissing`), `On Error Resume Next` (statement, `If` condition and `Select Case`
subject failures), `On Error GoTo` with labels, `Err.Number` and `Resume label`, library
constants and run-time functions, `InStr` with three arguments, hexadecimal and `Date`
literals, `CLng`/`CStr`, unsupported reporting, and a property test that mutated and
truncated caches never panic.

## 6. What blocks the rest

(Round 2. The state after the object, type and member round is in section 10.)

Counts are procedures of standard modules (the 229 class-like ones and the 62 `Declare`s
are separate, structural reasons).

* Locals and parameters of object, user-defined, array and fixed-length string types
  (14 + 7 + 1 procedures): the cache gives the type reference but not the bounds, the
  fixed length or the module-local user type name (`Unknowns` in
  [pcode-status.md](pcode-status.md)).
* Module-level variables and constants (9 + 2): done for scalars in section 8 (the value of a
  constant is the expression in front of its `VAR_DECL`).
* `Static` locals (2), `Mid$` assignment (`SUBSTR_ASSIGN`, 2), `Set` (1), `ReDim` (1),
  `Option Base` (1), `LBound` (1), indexing of a variable (1), `ParamArray` (1): each is
  one opcode family whose operand layout is inferred, with a single use in the sample.
* Class-like modules (229): lowered in section 8; what is left there is in 8.4.
* Other versions and bitness: verified in section 9; the same subset is lowered there.

## 7. Test material that would still help

Other versions and bitness were verified in section 9. What is still without material:

* VBA 5 and VBA 3 projects (version word below 0x6B); big-endian (Macintosh) caches;
  version words not listed in `VERIFIED_LAYOUTS` (for example a 32-bit build writing
  0xB5, or a 2013 32-bit build).
* Small purpose-made modules, each exercising one family with its source: `On Error`
  forms, `Optional` with defaults, module `Const`/`Static`/`Dim`, arrays, `Date` and
  `Currency` literals, 64-bit integer literals, `Case Is`, `Do`/`Loop` with `Exit Do`,
  all `CONVERT_TO_TYPE` modifiers, `With`, `For Each`, `New`, `Set`, `Property`
  procedures, a procedure with `PROC_CONTEXT`.
* Raw index 0xC3 and above of the built-in names, to settle the adjustment of
  pcode-format.md 5.2.
* A compiled-only project (no source) to confirm that the cache has the same structure
  when the source part is empty.

## 8. Class-like modules and module-level names

The lowering now covers class, document and designer modules, and module-level
variables and constants of every module. Counts are for the local project; the
verification is the differential run of section 5, extended as described in
8.3.

### 8.1 What was added

* `ModuleInput` carries the module role (`ModuleKind`) instead of a flag. The
  shell is built with the real role, so the analysis binds members of the
  module on `Me`, properties, fields and `Class_Initialize`/`Class_Terminate`
  the way it does for source text.
* The shell declares the module-level variables and constants found in the
  cache (scalars; literal constants with the literal's own type) and the
  procedures as `Public` members, with `Property Get` and `Property Let`
  (`Property Set` stays unsupported). Declarations that cannot be generated
  (user types, objects, arrays, `WithEvents`, `New`, constants that are not a
  literal) are left out and reported; a name that refers to one does not
  resolve, so the procedure is unsupported.
* Names resolve like this: local, local constant, module variable or constant
  (own module, then public ones of standard modules), project procedure (own
  module first; members of other class-like modules are not reachable by a bare
  name), library names. A variable of a standard module is a static variable
  (`LoadGlobal`/`StoreGlobal`), of a class-like module a field of the instance
  (`Me` then `LoadField`/`StoreField`). `Me.name` reaches the module's own
  variables, procedures and properties; assignment to a property is a call of
  its `Property Let` with the value passed by value; a call of a member of the
  module carries `Me` as receiver.
* In a document or designer module a member the module does not declare is
  late bound on the host base object of the instance (`Me.txtName`,
  `Me.Caption`, `Me.ctl.Prop`, `Me.Refresh`), as the source binding does. A bare
  unresolved name is a host member only when it is called, used as a statement,
  or the receiver of a member access; a bare value read needs the name in
  `LowerOptions::host_members` (the list the embedding application also hands
  to the source analysis), otherwise the source binding reads an implicit
  variable or reports an undeclared one. A name the cache declares anywhere
  (a procedure or variable that is not part of the program) is never taken for
  a host member.
* `Class_Initialize`/`Class_Terminate` run when the run time creates and
  destroys an instance. A module whose one is not lowered offers none of its
  procedures (reported as such).
* A `Function` or `Property Get` of a class-like module has a result record at
  the end of its argument chain (pcode-format.md 6.6); it is not a parameter.
  A parameter or local whose name the identifier table cannot give keeps its
  identity under a placeholder spelling.
* Also lowered: `GoTo label`, `End`, `On Error GoTo 0`, omitted arguments
  (`F(a, , c)`), local `Const`. Opcodes accepted below `Verified`, with their
  form and verification level: pcode-opcodes.md 4.18.
* Two rules found by the differential run: the run time releases the objects
  held by temporaries when the statement changes, so the comparisons of a
  `Select Case` belong to the statement of the `Select Case` (the subject can be
  an object); and the value of an assignment to a member is evaluated before
  the receiver.

### 8.2 Coverage on the local project

| Metric | Before | After |
|---|---|---|
| Procedures in the caches | 430 | 430 |
| Standard-module procedures with a body (139) lowered | 95 (68 %) | 99 (71 %) |
| Class-like-module procedures with a body (205) lowered | 0 | 90 (44 %) |
| Lowered in total | 95 | **189** |
| Module-level declarations found / left out | not read | 471 / 60 |
| Differential: procedures executed | 95 | 99 + 90 |
| Differential: runs | 509 | 524 + 344 (170 end in a run-time error on both sides, identically) |
| Differential: whole-module sessions | none | 112 |
| Differential: mismatches | 0 | 0 |

### 8.3 Verification

The embedding application's choices are made in the test, not in the library:
forms and reports are document modules, and the names the source analysis
reports as undeclared in them are the host members. Both programs run on
fresh instances whose host base object accepts every member name, journals
every get, put and call with its arguments, and answers control-like names with
child objects; the journals are compared together with the result,
`Debug.Print` output and error. A session runs every lowered procedure of a
module one after the other on one VM and instance, so that variables written by
one are read by the next (4 rounds per module, alternating order).

The run was checked for sensitivity: evaluating the receiver before the value
of a member assignment gives 5 mismatches. Two stores were also removed to see
whether the local project exercises them: no lowered procedure of the
local project writes a module-level variable or a field and reads it back, so
those stores are covered by the hand-assembled caches only (`tests/classes.rs`:
a counter in a standard module, a class with fields, `Class_Initialize`, a
property pair, `Me.field` with a shadowing parameter).

Hand-assembled caches (`tests/classes.rs`, 16 tests): module-level variables
and constants of both kinds of module, the literal types of constants, a class
with fields, properties and `Me`, an initializer that is not lowered, the
result record, placeholder names, host-base members (reads, assignments, calls,
chains; bare names with and without the list), members the cache declares,
`GoTo`, omitted arguments, local constants, `End`, `On Error GoTo 0`.

### 8.4 What is left, class-like and standard

Counts are procedures with a body (139 standard, 205 class-like), the first
reason found is counted: 40 standard and 115 class-like procedures are left.

| Class-like | Standard | Reason |
|---|---|---|
| 31 | - | an instance runs `Class_Initialize` or `Class_Terminate`, which is not lowered (the initializer of the one property class of the project calls a procedure that works with library objects, its terminator calls a `Declare`) |
| 33 | - | calls a procedure of the module that is itself unsupported for the reasons below |
| 27 | 14 | a local of object, user-defined or array type: `Database`, `Workspace`, `QueryDef`, `TableDef`, `Control`, `Collection` (library types, which the analysis cannot resolve without a type library), Win32 record types used with `Declare`s, and arrays whose descriptor is not decoded |
| 4 | 8 | a parameter of such a type, or `ParamArray` |
| 4 | 9 | a name outside the program: a module-level object of a project class (`mc`), `Screen` (library object), a `ListBox` variable, `Erl` |
| 16 | 9 | an opcode family with one to four uses: `Set` (3 + 1), `Option Base` (4 + 1), `Bang` access (2), `LBound`/`UBound` (2 + 1), `ReDim` (1), `Mid$` assignment (2), `Static` locals (1 + 2), numeric labels (1), a further `CONVERT_TO_TYPE` target, a text-compare intrinsic (1), indexing of a variable (1), assignment to a property without a `Let` (1) |

The object, array and user-type locals asked for in the status document are
blocked by library and `Declare` dependencies in this project rather than by the
lowering: every procedure with such a local of a library type calls the library,
and the Win32 record types only occur in procedures that call `Declare`s, which
are not lowered. Arrays (bounds, `ReDim`, indexing, `LBound`/`UBound`) would
unlock a handful of procedures at most and need the array type descriptor
decoded first.

## 9. Other versions and 64-bit

Material: 46 project files (public documents in the binary and the open document
formats, and standalone project storages), with the source of every module in the
same file. Counted per file: 492 modules, 2,693
procedures. They were read with the container reader of `vba-format`; the tests are
`vba-pcode/tests/corpus.rs`, `vba-pcode-ir/tests/corpus.rs` and
`vba-pcode-ir/tests/differential.rs` (the last two read the same directory). Several
files carry the same small project (the same macro in a binary and an open-format
workbook, or the same source built by two builds); counts are per file.

### 9.1 Layouts

The decoder runs unchanged apart from the corrections listed in 9.4. For each layout,
every module parsed and tiled gap-free, the source line count equalled the cache line
count, every line with code decoded to its exact length, and the checks of
[pcode-status.md](pcode-status.md) (name operands, literals, record names, kinds, types,
visibility) agreed; "consistent" is the share of source statements for which every
decoded operand is on the source line and every project identifier and literal of the
line is among the operands.

| Version word | Platform kind | Files | Modules | Procedures | Lines with code | Statements consistent |
|---|---|---|---|---|---|---|
| 0x79 (VBA 6) | 1 (32-bit) | 3 | 9 | 6 | 39 | 39 (100 %) |
| 0x88 (VBA 6) | 1 | 3 | 9 | 7 | 74 | 74 (100 %) |
| 0x97 | 1 | 4 | 34 | 77 | 1,030 | 1,030 (100 %) |
| 0xAF | 1 | 5 | 10 | 12 | 256 | 253 (98.8 %) |
| 0xB2 | 1 | 2 | 11 | 3 | 10 | 10 (100 %) |
| 0x9A | 3 (64-bit) | 2 | 8 | 2 | 6 | 6 (100 %) |
| 0xA6 | 3 | 7 | 12 | 7 | 21 | 21 (100 %) |
| 0xB2 | 3 | 5 | 35 | 15 | 549 | 549 (100 %) |
| 0xB5 | 3 | 15 | 364 | 2,564 | 29,122 | 28,366 (97.4 %) |
| total | | 46 | 492 | 2,693 | 31,107 | 30,348 (97.6 %) |

All 55,975 name operands, 5,617 string operands and 4,623 integer operands agree with
the source; 3,185 variable names, 2,487 argument names, 2,083 intrinsic and 869 named
types, and the kind, `Declare`-ness, visibility, `Friend`, `Static` and `PtrSafe` of
2,693 procedures agree. The statements that are not consistent are the library-qualified
type names, the `Implements` interface names (not in the operands), lines stored as
text because the compiler did not take them (a `Declare` without `PtrSafe` on x64,
the other branch of `#If`) and 4 forward failures.

A pair of builds of one source (25 modules, 1,021 lines with code; version words 0x97 on
32-bit and 0xB5 on 64-bit) decodes to the same opcode and modifier on every line, with
equal literals and equal argument, variable and procedure information.

### 9.2 Lowering and differential execution

`lower_project_with_options` accepts all 46 projects (the verified layouts); the corpus
differential runs the lowered procedures and the procedures of the source on `vba-vm`
exactly as in section 5 (argument sets, one fresh instance per run for class-like
modules, a recording host mock, whole-module sessions).

| Version word, platform kind | Files | Procedures | Lowered | Executed (std + class) | Runs (std + class) | Sessions | Mismatches |
|---|---|---|---|---|---|---|---|
| 0x79, 1 | 3 | 6 | 1 | 0 + 1 | 0 + 1 | 4 | 0 |
| 0x88, 1 | 3 | 7 | 2 | 1 + 1 | 1 + 1 | 8 | 0 |
| 0x97, 1 | 4 | 77 | 12 | 2 + 10 | 6 + 14 | 32 | 0 |
| 0xAF, 1 | 5 | 12 | 3 | 0 + 3 | 0 + 3 | 12 | 0 |
| 0xB2, 1 | 2 | 3 | 3 | 3 + 0 | 3 + 0 | 8 | 0 |
| 0x9A, 3 | 2 | 2 | 2 | 0 + 2 | 0 + 2 | 8 | 0 |
| 0xA6, 3 | 7 | 7 | 7 | 0 + 7 | 0 + 7 | 28 | 0 |
| 0xB2, 3 | 5 | 15 | 7 | 3 + 4 | 9 + 4 | 28 | 0 |
| 0xB5, 3 | 15 | 2,564 | 242 | 24 + 218 | 84 + 319 | 360 | 0 |
| total | 46 | 2,693 | 279 | 33 + 246 | 103 + 351 | 488 | 0 |

The coverage is low in the large projects for reasons that are not about layout (as in
section 8.4): locals, parameters and return types of object, library and class types (about 1,100 of
2,693), module-level variables of a user-defined type read through a name that is not
lowered (346 procedures), `Set` (99), host objects used without being declared (a
bare `Application`, a sheet collection), `Declare` (20), `Property Set`, `With`, and
`Implements`.

The first project still gives 189 of 430 procedures lowered, 524 + 344 runs, 112 sessions
and 0 mismatches. It had 120 mismatches at the start of this step, all in
procedures with `On Error Resume Next` followed by `Resume` without an error: the
source lowering (`vba-ir`) had since kept the statement after such a `Resume`
reachable so that `Resume Next` skips error 20, and the cache lowering had not. It does now.

The corpus differential had 20 mismatches in its first run, for a different reason:
the name of another module of the project (a form, `frmConsole.Show`) was lowered as a
member of the host base object while the source side binds it to the module. A name that is
the name of a module is now never a host member, and the procedure is not lowered.

(Round 3, section 10, raises the 46-project figures above from 279 to 842 lowered
procedures and the first project from 189 to 192 of 430, still with 0 mismatches.)

### 9.3 What is verified now, and what is not

Verified (the numbers above): the module header and the tables for VBA 6 and VBA 7, 32-bit
and 64-bit; the record layouts of function, argument, variable, type descriptor, user type,
object entry and declaration records on both bitnesses; the opcode numbering of
both; the identifier adjustment for VBA6 (0), VBA7 x86 (4) and x64 (7); 29 built-in names;
the line table, including the cases where the code area has gaps; the lowering and its
differential checks on all of the above.

Not verified: VBA5 and VBA3 (no material), big-endian caches, version words not in
`VERIFIED_LAYOUTS`, `PROC_CONTEXT` and its second operand on x64, the 8-byte integer
literals, the built-in adjustment above 0xC3, 97 opcodes never seen, and the `Static`,
`ParamArray` and `Optional` forms on x64 beyond what the differential run covers. (The
first word of `Implements` and the library-qualified type names were resolved in
round 3, section 10.1.)

### 9.4 Corrections made to the earlier text and code

* Bitness is the platform kind, not the version word: `0xB2` is written by both
  builds, `0xD7`, `0xA3` and `0xD9` were not seen.
* The declaration table length is at 0x3F on x64 as well (not 0x43).
* x64 function record: the option, count and flag bytes are 6 bytes later than the
  widening of the leading fields; the argument `next` pointer shifts with the other
  fields; the function record has 88 bytes.
* The identifier adjustment has no step above 0xBE; x64 subtracts 7.
* Type descriptor and `obj` operands are the entry index times the pointer size (4
  or 8), and index 0 is a valid entry (the former rule of "byte offset, a multiple of 10"
  was wrong on x86, too).
* The result record name sentinel is 0xFFFF on x64 (0xFFFE on x86).
* `Implements` has two words, not four.
* `Attribute` lines have no cache line wherever they stand.
* The module platform word at offset 2 equals PROJECTSYSKIND.
* `LongLong` has type id 20; `LongPtr` on x64 is id 20 with the pointer flag.

## 10. Types, objects and members (round 3)

Goal of this step: raise the share of compiled procedures that lower to IR and agree
with the source-derived program on `vba-vm`. Starting point on the 46 files: 2,693
procedures decode, 279 lower (10.4 %). The first measurement ranked the reasons
(procedures counted once, by the first reason found, `tests/corpus.rs`): types of
parameters, locals and results that are not plain scalars (about 1,100), a module-level
variable of a user-defined type (346), `Set` (99), host objects that are used without
being declared, `Declare` (20), and a long tail. This section records what changed,
what the differential run confirms, and what is left.

### 10.1 What the cache gave

**Library-qualified type names.** An object-table entry with bit 0 of the word at +2 set
does not hold a name; its word at +6 is the slot of a record in a list that follows the
object table (pcode-format.md 6.1a). The record holds two identifier references: the
qualifier and the type name. For 176 qualified variable types the two agree with the
source tokens in 176 of 176 cases; 101 `New`/`TypeOf` operands and 358 argument and
return types resolve the same way. The qualifier is a name, not a library: a type
qualified with the name of the project itself is a type of the project (the lowering takes
the project name as a setting of the embedding application,
`LowerOptions::project_name`); every other qualifier is a library the cache does not
describe, and the type stays unresolved. This is the only place where the cache says
which library, and it does not say what the library contains.

**`Implements`.** The first word of the statement leads to a record that starts with a
type descriptor of the interface (pcode-format.md 6.2a): 226 of 226 statements resolve
(206 to a class of the project, 20 to a library-qualified type). The shell declares
`Implements` for the interfaces that are classes of the project.

**Arrays.** An array type descriptor has kind byte 0x1B and the element type at +6
(pcode-format.md 6.7). The decoder used to read it as a simple type and so returned the
element type as the type of the variable (118 variable descriptors of the `0xB5` files).
That was a defect of the decoder; arrays are now `TypeRef::Array` and the lowering
refuses them. The bounds of a declared array are the constant expressions in front of the
declaration; the descriptor does not hold them.

**Enums and user-defined types.** `TYPE_DECL` with MOD 1 is a `Type`, MOD 3 an `Enum`;
the scope is in the record. A `Type` field is a `VAR_DECL` between `TYPE_DECL` and
`TYPE_DECL_END`; an enum member is `INVOKE_SIMPLE name` (no value) or its value
expression followed by `WRITE_SIMPLE name`, and the block ends with `ENUM_DECL_END`
(an enum used to swallow the declarations after it). Values and constants are rendered
from the expression instructions (literals, names, the arithmetic and logic operators,
`Nothing`) instead of single literals.

**Class default instances.** A class module whose name is used as the receiver of a
member form (`Name.Member`) has a default instance; the attribute that says so
(`VB_PredeclaredId`) has no line in the cache, so the shell declares it for every class
that is used that way. The same holds for the enumerator attribute of a class (the
lowering does not enumerate classes) and for the default member (the lowering refuses a
value assignment of an object).

### 10.2 What the lowering covers now

The shell (the declaration-only program the IR refers to) declares, from the cache
records: enums with their values, user-defined types with fields of any supported type,
classes (`Implements` included), and the types `Object`, `Collection`, and the enums of
the VBA library (`VbVarType`, ...). Parameters, results, locals and module-level
variables may have any of these types. A type that does not resolve (a library type, a
qualifier that is not the project, an array) leaves the declaration out, and with it every
procedure that needs it.

Bodies. `Set` (`SET_KEYWORD_MARKER`, `WRITE_REF_*`), `New`, `Nothing`, `Is`, `TypeOf ... Is`,
the member forms on every receiver of section 10.3, named arguments (also for late-bound
calls, which keep the names), properties with parameters, `Property Set`,
`With`, `For Each` over collections, `Debug.Assert`, conditional-compilation
directives, the temporaries a `ByRef Variant` parameter needs, `Err.Raise` and the other
members of the built-in objects (early bound with the member id), `VBA.`-qualified
and sub-library-qualified names, and the enum, module, project and class-name
qualifiers.

A variable of another type passed to a `ByRef Variant` parameter is passed as a copy in
a temporary that is converted back when the call returns (MS-VBAL 5.3.1.11). The
lowering had passed it by value; the differential run on the first project showed that
as mismatches in one procedure (the run was already failing when this step started, the
source lowering having changed since the previous measurement). The temporaries are
added to the locals of the procedure.

### 10.3 Evidence

Differential run (`tests/differential.rs`, as in section 5), now also with objects:
parameters of class type receive `Nothing` or a new instance (only of a class whose
initializer and terminator are lowered), `Object` parameters `Nothing` or the host
object, enums the values 0 to 3 and -1. A run that stops at the step limit on both
sides counts as agreeing up to that point (the number of steps per iteration depends on
the shape of the blocks, which the two lowerings do not share). Procedures the source
analysis rejects are not compared and are counted; none is left.

| Version word, platform kind | Files | Procedures | Lowered before | Lowered after | Executed (std + class) | Runs (std + class) | Sessions | Mismatches |
|---|---|---|---|---|---|---|---|---|
| 0x79, 1 | 3 | 6 | 1 | 1 | 0 + 1 | 0 + 1 | 4 | 0 |
| 0x88, 1 | 3 | 7 | 2 | 2 | 1 + 1 | 1 + 1 | 8 | 0 |
| 0x97, 1 | 4 | 77 | 12 | 14 | 2 + 12 | 6 + 24 | 32 | 0 |
| 0xAF, 1 | 5 | 12 | 3 | 3 | 0 + 3 | 0 + 3 | 12 | 0 |
| 0xB2, 1 | 2 | 3 | 3 | 3 | 3 + 0 | 3 + 0 | 8 | 0 |
| 0x9A, 3 | 2 | 2 | 2 | 2 | 0 + 2 | 0 + 2 | 8 | 0 |
| 0xA6, 3 | 7 | 7 | 7 | 7 | 0 + 7 | 0 + 7 | 28 | 0 |
| 0xB2, 3 | 5 | 15 | 7 | 7 | 3 + 4 | 9 + 4 | 28 | 0 |
| 0xB5, 3 | 15 | 2,564 | 242 | 803 | 45 + 758 | 203 + 1,724 | 752 | 0 |
| total | 46 | 2,693 | 279 (10.4 %) | **842 (31.3 %)** | 54 + 788 | 222 + 1,766 | 880 | 0 |

Before this step: 33 + 246 procedures executed, 103 + 351 runs, 488 sessions. The gain
is almost all in the `0xB5` x64 files, which hold the large projects; the small files
were already lowered as far as their content allows, and the 32-bit and VBA 6 files
contain little object code. Of the 842 compared procedures, 725 completed at least one
run without a run-time error on the source side (and so on both); the 439 of 1,988 runs
that end in an error end in the same error on both sides (typically `Nothing` or an empty
host object used as a receiver). The first project: 192 of 430 procedures lowered (189
before), 526 + 345 runs, 112 sessions, 0 mismatches.

Opcode forms, counted in the compared procedures that completed a run (instructions /
procedures): `SET_KEYWORD_MARKER` 273 / 192, `WRITE_REF_SIMPLE` 170 / 136, `WRITE_REF_DOT`
103 / 64, `NEW_OBJECT` 61 / 60, `READ_DOT` 354 / 209, `READ_DOT_PARAMS` 47 / 30,
`INVOKE_DOT` 69 / 50, `WRITE_DOT` 115 / 70, `COMPARE_OBJECT_IDENTITY` 37 / 31,
`CONST_NULL_OBJECT` 86 / 40, `ARG_NAMED` 7 / 6, `FOR_COLLECTION` 6 / 6, `DEBUG_ASSERT`
4 / 4, `WRITE_DOT_PARAMS` 7 / 1, `TYPE_TEST` 1 / 1, the `With` forms 1 / 1,
`PP_MARKER` 4 / 1. The table of pcode-opcodes.md 4.18 lists each accepted form. Forms
without a run in the 46 files rely on the assembled caches of `tests/objects.rs` alone:
the other `*_WITH_DOT` forms, `WRITE_REF_DOT_PARAMS`, and `Property Set` with arguments.
The opcode table is unchanged (86 Verified, 81 Inferred, 97 Unseen): an opcode becomes
Verified when its operand shape, order and stack effect are confirmed, and what the
differential run confirms here is the lowering of a form, not every use of the opcode.

What the differential run cannot check: it compares behaviour on the inputs it
tries. A procedure whose runs all end in the same error is only checked up to that
error, and the argument pools are small (up to six sets per procedure). Early- versus
late-bound dispatch of library members is not in the cache, so a library type is
lowered only when a provider describes it (the built-in `Collection`, `Err`, `Debug`);
the argument conversions of those calls follow the source binding and are checked only by
the runs that use them.

### 10.4 What is left

Refused procedures by the reason found after following calls and instance blocks to the
procedure that is refused for its own reason (1,851 procedures):

| Procedures | Reason |
|---|---|
| 1,464 (79 %) | a library type, library object or host member that the cache names but does not describe: parameters, locals and results of such types (about 320), `New` and `TypeOf` of a library class (about 110), a module-level variable of a user-defined type that has a library field (453), host objects and members (`Application`, sheet collections, the base class of a document module: about 260), names of other projects. These need a provider for the libraries (their classes, members and dispatch ids); with one, the same cache would lower as early-bound calls |
| 180 (10 %) | arrays, the largest items being `ReDim` of a field (45), locals and parameters of array type (about 50), `UBound`/`LBound` (21), `Array(...)` (20) and `ParamArray` (9); the descriptor is decoded (10.1), the element access, the bounds and the `ReDim` forms are not lowered |
| 53 (3 %) | an assignment to a name that is not declared (implicit variables) |
| 154 (8 %) | `Declare` (20), strings outside ASCII (31; the code page of the project is not an input of the lowering), `Resume` forms (20), file numbers (12), `Set` with a source that is not a known object or object types that do not convert (21), `CONVERT_TO_TYPE` targets not seen (9), `As New` (4), and a long tail of one to seven procedures |

The share of reasons that no lowering can remove without library descriptions is the
measure of how far a cache-only lowering can go: about four fifths of what is left.
Two items were left on purpose even though the cache has the data: fields and
variables of an unresolved library type are not declared as `Object` (it would change
early-bound calls into late-bound ones), and a class is not enumerated by `For Each`
(its enumerator attribute has no line in the cache).

### 10.5 Opcodes never seen, and what they need

Of the 264 canonical opcodes, 167 occur in the 46 files and 97 do not (86 Verified, 81
Inferred). From their names and the opcode document, the 97 are reachable only from
constructs that do not occur in the material (the right-hand column is the probable
construct, not an observation, and none of the 97 has been seen to decode):

| Opcodes (canonical numbers) | Construct that would produce them |
|---|---|
| `BOOL_IMPLICATION` (0), `BOOL_EQUIVALENCE` (1) | the `Imp` and `Eqv` operators |
| `INTRINSIC_TRUNCATE` (24), `INTRINSIC_LENGTH_BYTES` (28), `INTRINSIC_CURRENT_DIR` (125), `INTRINSIC_DIR_LISTING` (126), `INTRINSIC_ERROR_TEXT` (129), `INTRINSIC_FORMAT_VALUE` (130), `INTRINSIC_NEXT_FILE_NUMBER` (131), `INTRINSIC_SEARCH_BYTES_A/B/C` (135-137), `INTRINSIC_SUBSTRING` (139), `INTRINSIC_SUBSTRING_BYTES` (140), `INTRINSIC_REPEAT_FROM_CODE/TEXT` (143, 144) | calls of the library functions that have dedicated opcodes (`Fix`, `LenB`, `CurDir`, `Dir`, `Error$`, `Format`, `FreeFile`, `InStrB`, `Mid`, `MidB`, `String$`/`Space$`); each has a plain call form in the material, the dedicated opcode did not occur |
| `READ_BANG_PARAMS` (38), `WRITE_BANG_PARAMS` (45), `WRITE_REF_BANG_PARAMS` (52), `READ_WITH_BANG` (54), `READ_WITH_BANG_PARAMS` (56), `WRITE_WITH_BANG` (58), `WRITE_WITH_BANG_PARAMS` (60), `WRITE_REF_WITH_BANG` (62), `WRITE_REF_WITH_BANG_PARAMS` (64) | the dictionary-style `!` access with arguments, and inside `With` |
| `WRITE_DEFAULT_INDEXED` (42), `WRITE_REF_DEFAULT_INDEXED` (49), `WRITE_REF_WITH_DOT_PARAMS` (63) | assignment through the default member of an object (`obj(i) = v`), `Set` of an indexed member inside `With` |
| `READ_TARGET_NAME` (31), `LINE_BEGIN` (72), `FRAME_RESERVE` (241), `PROC_CONTEXT` (90), `PROC_CONTEXT_END` (104), `PROC_HEADER_ALT` (151), `SELF_REF` (192), `SELF_REF_IMPLICIT` (193), `WRITE_SIMPLE_AMBIGUOUS` (261), `RESERVED_A/B` (127, 128), `INVALID_SENTINEL` (263), `IMMEDIATE_END` (108) | structures the compiler emits only for particular procedures (frame setup, procedures with a context), for lines that were not compiled, or never in a project file; the document says they exist, no sample shows when |
| `ADDRESS_OF_DOT` (74) | `AddressOf` applied to a member |
| `CASE_IS_LESS` (78), `CASE_IS_GREATER_EQUAL` (79), `CASE_IS_LESS_EQUAL` (80) | `Case Is <`, `Is >=`, `Is <=` (the other three `Case Is` forms occur) |
| `BLOCK_ELSEIF_BRANCH_TYPE_TEST` (102), `IF_BLOCK_OPEN_TYPE_TEST` (158), `SWITCH_CASE_CLASS` (238), `SWITCH_BEGIN_TYPE` (239), `FOR_COLLECTION_TYPED` (148) | combined `TypeOf` conditions, `Select Case TypeOf`, a typed `For Each`; the material has the two-instruction forms |
| `DRAW_CIRCLE` (85), `DRAW_POINT` (225), `DRAW_SCALE` (235) | the legacy graphics statements |
| `FILE_CLOSE_ALL` (87), `FILE_LOCK` (187), `FILE_UNLOCK` (244), `FILE_SEEK` (236), `FILE_WRITE_RECORD` (226), `FILE_WRITE_CHANNEL` (249), `RENAME_FILE` (200) | file statements (`Reset`, `Lock`, `Unlock`, `Seek`, `Write #`, `Name ... As`) |
| `OUT_COMMA` (214), `OUT_STATEMENT_END` (215), `OUT_NEWLINE` (219), `OUT_SEMICOLON` (221), `OUT_SPACES` (222), `OUT_TAB` (223), `OUT_TAB_THEN_COMMA` (224), `READ_LIST_OPEN/CLOSE/ITEM` (160-162) | `Print`/`Write` list parts (`Spc`, `Tab`, a trailing separator) and `Input` lists |
| `CONVERT_TO_TYPE_VARIANT` (89), `CONST_CURRENCY` (169), `CONST_INT64_DECIMAL/HEX/OCTAL` (174, 177, 181), `CONST_INT16/INT32_OCTAL` (179, 180), `CONST_FLOAT32` (182), `CONST_SMALL_INT` (184) | `CVar`; `Currency`, 64-bit, octal and `Single` literals |
| `DEFTYPE_RANGE` (92), `YIELD_EVENTS` (96), `ERR_RAISE_STMT` (115), `HALT_STOP` (242), `COMMENT_REM_KEYWORD` (231), `LET_KEYWORD_MARKER` (164), `RIGHT_ALIGN_ASSIGN` (234), `SUBSTR_BYTES_ASSIGN` (199), `PP_ELSEIF` (254) | `DefInt` and the other `Def` statements, `DoEvents`, the `Error n` statement, `Stop`, `Rem`, `Let`, `RSet`, `MidB` as a statement, `#ElseIf` |
| `BRANCH_SUBROUTINE` (153), `BRANCH_RETURN` (233), `BRANCH_COMPUTED_SUBROUTINE` (205), `BRANCH_COMPUTED_GOTO` (206) | `GoSub`, `Return`, `On ... GoSub`, `On ... GoTo` |
| `EVENT_FIRE_DOT` (118), `EVENT_FIRE_WITH_DOT` (119) | `RaiseEvent` on another object |
| `REDIM_DOT_TYPED` (196), `REDIM_DOT_TYPED_WITH` (197), `REDIM_NEW_MARKER` (259) | `ReDim` of a member of a typed object, `ReDim ... As New` |

Sample material that would turn these into evidence: modules that use each construct once,
with their source (the list in section 7 stays, with `Imp`/`Eqv`, `GoSub`, the legacy
graphics and file statements, `Print` lists, `Def` statements and `Currency`/`Single`/
64-bit literals added), compiled with each of the verified layouts. Nothing in this
list is needed to read the procedures of the 46 files: every instruction of every line
with code decoded to its exact length.
