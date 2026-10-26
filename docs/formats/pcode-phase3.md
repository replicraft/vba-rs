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

Subset (class-like modules and module-level names: section 8): standard modules; `Sub`/`Function` with plain scalar parameters (including
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
* Other versions and bitness: see section 7.

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
