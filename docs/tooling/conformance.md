# Conformance

This file is generated from `tests/inventory.csv` and the programs under `tests/vba/` by
`VBA_BLESS=1 cargo test -p vba-rs --test conformance`; a test fails when it is stale.

The inventory lists the statements, declarations, operators, functions, objects and data types
that the Microsoft VBA language reference (and MS-VBAL) enumerates. Each entry's status is derived from
the programs that exercise it: `implemented` means every program for the entry passes, `partial` that some
pass and some are recorded known deviations, `unsupported` that the construct is rejected or not run,
`not-started` that no program exists yet. **A status measures only what the listed programs check; it
is not a claim of full compatibility.** An entry with a handful of programs is far from fully
verified, and `unverified` provenance means the expected value is the implementation's own behavior.

Provenance of the expected values (the weakest among an entry's passing programs):

* `oracle`: confirmed by running the program in real VBA (environment pinned in `tests/oracle/ENVIRONMENTS.md`)
* `spec`: stated by MS-VBAL (a section is cited)
* `doc`: stated by the cited Microsoft language-reference page (example or remark)
* `derived`: derived by reasoning from documented semantics, not stated verbatim
* `unverified`: only the current behavior; no document backs it

## Totals

| Measure | Count |
| --- | ---: |
| Inventory entries | 708 |
| implemented | 510 |
| partial | 159 |
| unsupported | 6 |
| not-started | 33 |
| Conformance programs | 2085 |
| Programs that are known deviations (XFAIL) | 175 |

## Entries per category and status

| Category | Total | implemented | partial | unsupported | not-started |
| --- | ---: | ---: | ---: | ---: | ---: |
| statement | 76 | 55 | 19 | 2 | 0 |
| function-conversion | 8 | 6 | 2 | 0 | 0 |
| function-math | 13 | 13 | 0 | 0 | 0 |
| function-type-conversion | 13 | 11 | 2 | 0 | 0 |
| derived-math | 21 | 21 | 0 | 0 | 0 |
| function-string | 20 | 16 | 4 | 0 | 0 |
| function-datetime | 20 | 17 | 3 | 0 | 0 |
| function-financial | 13 | 13 | 0 | 0 | 0 |
| function-information | 11 | 10 | 1 | 0 | 0 |
| function-array | 3 | 3 | 0 | 0 | 0 |
| function-logic | 4 | 3 | 1 | 0 | 0 |
| function-formatting | 4 | 4 | 0 | 0 | 0 |
| function-file | 11 | 9 | 2 | 0 | 0 |
| function-interaction | 19 | 17 | 2 | 0 | 0 |
| operator-arithmetic | 8 | 0 | 8 | 0 | 0 |
| operator-concatenation | 1 | 0 | 1 | 0 | 0 |
| operator-comparison | 8 | 5 | 3 | 0 | 0 |
| operator-logical | 6 | 2 | 4 | 0 | 0 |
| operator-precedence | 1 | 1 | 0 | 0 | 0 |
| data-type | 18 | 18 | 0 | 0 | 0 |
| identifier-type-character | 7 | 7 | 0 | 0 | 0 |
| object | 13 | 10 | 0 | 3 | 0 |
| language-element | 38 | 26 | 11 | 1 | 0 |
| spec-environment | 5 | 4 | 1 | 0 | 0 |
| spec-lexical | 12 | 9 | 3 | 0 | 0 |
| spec-organization | 3 | 1 | 0 | 0 | 2 |
| spec-declarations | 21 | 11 | 10 | 0 | 0 |
| spec-procedures | 12 | 6 | 5 | 0 | 1 |
| spec-statements | 55 | 34 | 20 | 0 | 1 |
| spec-coercion | 18 | 9 | 8 | 0 | 1 |
| spec-expressions | 53 | 22 | 26 | 0 | 5 |
| spec-library | 193 | 147 | 23 | 0 | 23 |

## Entries per provenance

| Provenance | Entries |
| --- | ---: |
| oracle | 0 |
| spec | 441 |
| doc | 99 |
| derived | 39 |
| unverified | 96 |
| (no program) | 33 |

## Programs per area and provenance

| Area | Programs | oracle | spec | doc | derived | unverified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arrays | 31 | 0 | 7 | 10 | 3 | 11 |
| classes | 28 | 0 | 12 | 4 | 9 | 3 |
| collections | 23 | 0 | 7 | 14 | 0 | 2 |
| conversions | 39 | 0 | 28 | 4 | 2 | 5 |
| datatypes | 52 | 0 | 27 | 16 | 0 | 9 |
| datetime | 45 | 0 | 18 | 14 | 2 | 11 |
| derived-math | 21 | 0 | 0 | 21 | 0 | 0 |
| errors | 26 | 0 | 18 | 6 | 0 | 2 |
| events | 7 | 0 | 6 | 0 | 1 | 0 |
| file-io | 47 | 0 | 19 | 19 | 3 | 6 |
| financial | 13 | 0 | 0 | 2 | 11 | 0 |
| host | 34 | 0 | 3 | 15 | 10 | 6 |
| language | 18 | 0 | 6 | 4 | 0 | 8 |
| logic | 10 | 0 | 7 | 3 | 0 | 0 |
| math | 37 | 0 | 24 | 4 | 2 | 7 |
| operators | 65 | 0 | 59 | 0 | 5 | 1 |
| procedures | 34 | 0 | 23 | 2 | 4 | 5 |
| spec-coerce | 187 | 0 | 187 | 0 | 0 | 0 |
| spec-data | 274 | 0 | 274 | 0 | 0 | 0 |
| spec-decl | 155 | 0 | 155 | 0 | 0 | 0 |
| spec-lex | 145 | 0 | 145 | 0 | 0 | 0 |
| spec-lib-a | 96 | 0 | 96 | 0 | 0 | 0 |
| spec-lib-b | 61 | 0 | 61 | 0 | 0 | 0 |
| spec-lib-c | 58 | 0 | 58 | 0 | 0 | 0 |
| spec-lib-d | 74 | 0 | 74 | 0 | 0 | 0 |
| spec-ops | 131 | 0 | 131 | 0 | 0 | 0 |
| spec-proc | 214 | 0 | 214 | 0 | 0 | 0 |
| statements | 88 | 0 | 74 | 13 | 1 | 0 |
| strings | 72 | 0 | 33 | 16 | 8 | 15 |

## Gaps

Entries that are not `implemented`, with the reason recorded in the inventory.

| Id | Category | Name | Status | Notes |
| --- | --- | --- | --- | --- |
| `stmt.call` | statement | Call | partial |  |
| `stmt.dim` | statement | Dim | partial |  |
| `stmt.get` | statement | Get | partial |  |
| `stmt.input` | statement | Input # | partial |  |
| `stmt.let` | statement | Let | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `stmt.line-input` | statement | Line Input # | partial |  |
| `stmt.load` | statement | Load | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `stmt.lock-unlock` | statement | Lock, Unlock | partial |  |
| `stmt.lset` | statement | LSet | partial |  |
| `stmt.mid` | statement | Mid | partial |  |
| `stmt.on-error` | statement | On Error | partial |  |
| `stmt.open` | statement | Open | partial |  |
| `stmt.print` | statement | Print # | partial |  |
| `stmt.put` | statement | Put | partial |  |
| `stmt.redim` | statement | ReDim | partial | element assignment into an array held in a Variant (ReDim v(..); v(i) = x) is rejected as 'late binding' |
| `stmt.seek` | statement | Seek | partial |  |
| `stmt.set` | statement | Set | partial |  |
| `stmt.setattr` | statement | SetAttr | partial |  |
| `stmt.unload` | statement | Unload | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `stmt.width` | statement | Width # | partial |  |
| `stmt.write` | statement | Write # | partial |  |
| `func.hex` | function-conversion | Hex | partial |  |
| `func.str` | function-conversion | Str | partial |  |
| `func.cbyte` | function-type-conversion | CByte | partial |  |
| `func.cdbl` | function-type-conversion | CDbl | partial |  |
| `func.instrrev` | function-string | InStrRev | partial |  |
| `func.left` | function-string | Left | partial |  |
| `func.strconv` | function-string | StrConv | partial | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
| `func.string` | function-string | String | partial |  |
| `func.datepart` | function-datetime | DatePart | partial |  |
| `func.day` | function-datetime | Day | partial |  |
| `func.timeserial` | function-datetime | TimeSerial | partial |  |
| `func.isnumeric` | function-information | IsNumeric | partial |  |
| `func.partition` | function-logic | Partition | partial | Partition is not defined ('Sub or Function not defined') |
| `func.eof` | function-file | EOF | partial |  |
| `func.filelen` | function-file | FileLen | partial |  |
| `func.error` | function-interaction | Error | partial |  |
| `func.getallsettings` | function-interaction | GetAllSettings | partial | the registry-backed settings statements have no host service; registry-backed settings have no host service |
| `op.pow` | operator-arithmetic | ^ | partial |  |
| `op.neg` | operator-arithmetic | - (negation) | partial |  |
| `op.mul` | operator-arithmetic | * | partial |  |
| `op.div` | operator-arithmetic | / | partial |  |
| `op.intdiv` | operator-arithmetic | \ | partial |  |
| `op.mod` | operator-arithmetic | Mod | partial |  |
| `op.add` | operator-arithmetic | + | partial |  |
| `op.sub` | operator-arithmetic | - (subtraction) | partial |  |
| `op.concat` | operator-concatenation | & | partial |  |
| `op.eq` | operator-comparison | = | partial |  |
| `op.lt` | operator-comparison | < | partial |  |
| `op.like` | operator-comparison | Like | partial |  |
| `op.not` | operator-logical | Not | partial |  |
| `op.and` | operator-logical | And | partial |  |
| `op.or` | operator-logical | Or | partial |  |
| `op.imp` | operator-logical | Imp | partial |  |
| `obj.drive` | object | Drive | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.drives` | object | Drives | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.userform` | object | UserForm | unsupported | UserForm objects need a host that provides forms |
| `lit.string` | language-element | String literal | partial |  |
| `lit.null` | language-element | Null | partial |  |
| `expr.index` | language-element | Index expression | partial |  |
| `expr.typeof` | language-element | TypeOf ... Is expression | partial |  |
| `expr.addressof` | language-element | AddressOf expression | unsupported | AddressOf produces no callable native pointer without the Declare bridge |
| `expr.default-member` | language-element | Default member resolution | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `expr.implicit-conv` | language-element | Implicit conversion in expressions | partial |  |
| `lex.continuation` | language-element | Line continuation | partial |  |
| `cc.const` | language-element | Conditional compilation constants (#Const) | partial |  |
| `decl.byref-byval` | language-element | ByRef and ByVal | partial |  |
| `decl.withevents` | language-element | WithEvents | partial |  |
| `decl.dim-as-new` | language-element | Dim ... As New | partial |  |
| `vbal.2.5.1` | spec-environment | Automatic Object Instantiation | partial |  |
| `vbal.3.2.1` | spec-lexical | Physical Line Grammar | partial |  |
| `vbal.3.2.2` | spec-lexical | Logical Line Grammar | partial |  |
| `vbal.3.3.4` | spec-lexical | String Tokens | partial |  |
| `vbal.4.1` | spec-organization | Projects | not-started |  |
| `vbal.4.2.1` | spec-organization | Module Extensibility | not-started |  |
| `vbal.5.1` | spec-declarations | Module Body Structure | partial |  |
| `vbal.5.2.3.1` | spec-declarations | Module Variable Declaration Lists | partial |  |
| `vbal.5.2.3.1.1` | spec-declarations | Variable Declarations | partial |  |
| `vbal.5.2.3.1.2` | spec-declarations | WithEvents Variable Declarations | partial |  |
| `vbal.5.2.3.3` | spec-declarations | User Defined Type Declarations | partial |  |
| `vbal.5.2.3.4` | spec-declarations | Enum Declarations | partial |  |
| `vbal.5.2.3.5` | spec-declarations | External Procedure Declaration | partial |  |
| `vbal.5.2.3.6` | spec-declarations | Circular Module Dependencies | partial |  |
| `vbal.5.2.4.1.2` | spec-declarations | Default Instance Variables Static Semantics | partial |  |
| `vbal.5.2.4.3` | spec-declarations | Event Declaration | partial |  |
| `vbal.5.3.1` | spec-procedures | Procedure Declarations | partial |  |
| `vbal.5.3.1.2` | spec-procedures | Static Procedures | partial |  |
| `vbal.5.3.1.3` | spec-procedures | Procedure Names | not-started |  |
| `vbal.5.3.1.5` | spec-procedures | Parameter Lists | partial |  |
| `vbal.5.3.1.8` | spec-procedures | Event Handler Declarations | partial |  |
| `vbal.5.3.1.11` | spec-procedures | Procedure Invocation Argument Processing | partial |  |
| `vbal.5.4.1.1` | spec-statements | Statement Labels | partial |  |
| `vbal.5.4.2.1` | spec-statements | Call Statement | partial |  |
| `vbal.5.4.2.11` | spec-statements | Stop Statement | not-started |  |
| `vbal.5.4.3.1` | spec-statements | Local Variable Declarations | partial |  |
| `vbal.5.4.3.3` | spec-statements | ReDim Statement | partial |  |
| `vbal.5.4.3.5` | spec-statements | Mid/MidB/Mid$/MidB$ Statement | partial |  |
| `vbal.5.4.3.6` | spec-statements | LSet Statement | partial |  |
| `vbal.5.4.3.8` | spec-statements | Let Statement | partial |  |
| `vbal.5.4.4.1` | spec-statements | On Error Statement | partial |  |
| `vbal.5.4.5.1` | spec-statements | Open Statement | partial |  |
| `vbal.5.4.5.1.1` | spec-statements | File Numbers | partial |  |
| `vbal.5.4.5.3` | spec-statements | Seek Statement | partial |  |
| `vbal.5.4.5.4` | spec-statements | Lock Statement | partial |  |
| `vbal.5.4.5.5` | spec-statements | Unlock Statement | partial |  |
| `vbal.5.4.5.6` | spec-statements | Line Input Statement | partial |  |
| `vbal.5.4.5.7` | spec-statements | Width Statement | partial |  |
| `vbal.5.4.5.8` | spec-statements | Print Statement | partial |  |
| `vbal.5.4.5.9` | spec-statements | Write Statement | partial |  |
| `vbal.5.4.5.10` | spec-statements | Input Statement | partial |  |
| `vbal.5.4.5.11` | spec-statements | Put Statement | partial |  |
| `vbal.5.4.5.12` | spec-statements | Get Statement | partial |  |
| `vbal.5.5.1.1` | spec-coercion | Static semantics | partial |  |
| `vbal.5.5.1.2.1.1` | spec-coercion | Banker’s rounding | not-started |  |
| `vbal.5.5.1.2.2` | spec-coercion | Let-coercion to and from Boolean | partial |  |
| `vbal.5.5.1.2.4` | spec-coercion | Let-coercion to and from String | partial |  |
| `vbal.5.5.1.2.6` | spec-coercion | Let-coercion to and from resizable Byte() | partial |  |
| `vbal.5.5.1.2.9` | spec-coercion | Let-coercion to and from Error | partial |  |
| `vbal.5.5.1.2.11` | spec-coercion | Let-coercion from Empty | partial |  |
| `vbal.5.5.1.2.13` | spec-coercion | Let-coercion to and from a class or Object or Nothing | partial |  |
| `vbal.5.5.2.2.2` | spec-coercion | Set-coercion to and from non-object types | partial |  |
| `vbal.5.6.1` | spec-expressions | Expression Classifications | not-started |  |
| `vbal.5.6.2.1` | spec-expressions | Evaluation to a data value | partial |  |
| `vbal.5.6.2.2` | spec-expressions | Evaluation to a simple data value | partial |  |
| `vbal.5.6.2.3` | spec-expressions | Default Member Recursion Limits | not-started |  |
| `vbal.5.6.4` | spec-expressions | Expression Binding Contexts | not-started |  |
| `vbal.5.6.5` | spec-expressions | Literal Expressions | partial |  |
| `vbal.5.6.6` | spec-expressions | Parenthesized Expressions | partial |  |
| `vbal.5.6.7` | spec-expressions | TypeOf…Is Expressions | partial |  |
| `vbal.5.6.9.3` | spec-expressions | Arithmetic Operators | partial |  |
| `vbal.5.6.9.3.1` | spec-expressions | Unary - Operator | partial |  |
| `vbal.5.6.9.3.2` | spec-expressions | + Operator | partial |  |
| `vbal.5.6.9.3.3` | spec-expressions | Binary - Operator | partial |  |
| `vbal.5.6.9.3.4` | spec-expressions | * Operator | partial |  |
| `vbal.5.6.9.3.5` | spec-expressions | / Operator | partial |  |
| `vbal.5.6.9.3.6` | spec-expressions | \ Operator and Mod Operator | partial |  |
| `vbal.5.6.9.3.7` | spec-expressions | ^ Operator | partial |  |
| `vbal.5.6.9.4` | spec-expressions | & Operator | partial |  |
| `vbal.5.6.9.5` | spec-expressions | Relational Operators | partial |  |
| `vbal.5.6.9.6` | spec-expressions | Like Operator | partial |  |
| `vbal.5.6.9.8` | spec-expressions | Logical Operators | partial |  |
| `vbal.5.6.9.8.1` | spec-expressions | Not Operator | partial |  |
| `vbal.5.6.9.8.3` | spec-expressions | Or Operator | partial |  |
| `vbal.5.6.9.8.6` | spec-expressions | Imp Operator | partial |  |
| `vbal.5.6.10` | spec-expressions | Simple Name Expressions | partial |  |
| `vbal.5.6.12` | spec-expressions | Member Access Expressions | partial |  |
| `vbal.5.6.13` | spec-expressions | Index Expressions | partial |  |
| `vbal.5.6.13.1` | spec-expressions | Argument Lists | partial |  |
| `vbal.5.6.13.2` | spec-expressions | Argument List Queues | not-started |  |
| `vbal.5.6.16.1` | spec-expressions | Constant Expressions | partial |  |
| `vbal.5.6.16.2` | spec-expressions | Conditional Compilation Expressions | partial |  |
| `vbal.5.6.16.8` | spec-expressions | AddressOf Expressions | not-started |  |
| `vbal.6.1.2.3.1.2` | spec-library | CByte | partial |  |
| `vbal.6.1.2.3.1.5` | spec-library | CDbl | partial |  |
| `vbal.6.1.2.3.1.15` | spec-library | Error / Error$ | partial |  |
| `vbal.6.1.2.3.1.17` | spec-library | Hex / Hex$ | partial |  |
| `vbal.6.1.2.3.1.20` | spec-library | Str / Str$ | partial |  |
| `vbal.6.1.2.4.1.3` | spec-library | DatePart | partial |  |
| `vbal.6.1.2.4.1.6` | spec-library | Day | partial |  |
| `vbal.6.1.2.4.1.9` | spec-library | Month | partial |  |
| `vbal.6.1.2.4.1.11` | spec-library | TimeSerial | partial |  |
| `vbal.6.1.2.4.1.14` | spec-library | Year | partial |  |
| `vbal.6.1.2.5.1.3` | spec-library | EOF | partial |  |
| `vbal.6.1.2.5.1.6` | spec-library | FileLen | partial |  |
| `vbal.6.1.2.5.2.7` | spec-library | SetAttr | partial |  |
| `vbal.6.1.2.7.1.1` | spec-library | IMEStatus | not-started |  |
| `vbal.6.1.2.7.1.8` | spec-library | IsNumeric | partial |  |
| `vbal.6.1.2.8.1.3` | spec-library | Command | not-started |  |
| `vbal.6.1.2.8.1.4` | spec-library | CreateObject | not-started |  |
| `vbal.6.1.2.8.1.7` | spec-library | GetAllSettings | partial |  |
| `vbal.6.1.2.8.1.9` | spec-library | GetObject | not-started |  |
| `vbal.6.1.2.8.1.12` | spec-library | InputBox | not-started |  |
| `vbal.6.1.2.8.1.14` | spec-library | Partition | partial |  |
| `vbal.6.1.2.8.2.1` | spec-library | AppActivate | not-started |  |
| `vbal.6.1.2.8.2.5` | spec-library | SendKeys | not-started |  |
| `vbal.6.1.2.11.1.5` | spec-library | ChrB / ChrB$ | partial |  |
| `vbal.6.1.2.11.1.6` | spec-library | ChrW/ ChrW$ | partial |  |
| `vbal.6.1.2.11.1.15` | spec-library | InStrRev | partial |  |
| `vbal.6.1.2.11.1.18` | spec-library | LCase$ | not-started |  |
| `vbal.6.1.2.11.1.20` | spec-library | Left$ | not-started |  |
| `vbal.6.1.2.11.1.21` | spec-library | LeftB$ | not-started |  |
| `vbal.6.1.2.11.1.24` | spec-library | LTrim$ / RTrim$ / Trim$ | not-started |  |
| `vbal.6.1.2.11.1.26` | spec-library | Mid$ | not-started |  |
| `vbal.6.1.2.11.1.27` | spec-library | MidB$ | not-started |  |
| `vbal.6.1.2.11.1.30` | spec-library | Right / RightB | not-started |  |
| `vbal.6.1.2.11.1.31` | spec-library | Right$ | not-started |  |
| `vbal.6.1.2.11.1.32` | spec-library | RightB$ | not-started |  |
| `vbal.6.1.2.11.1.34` | spec-library | Space$ | not-started |  |
| `vbal.6.1.2.11.1.37` | spec-library | StrConv | partial |  |
| `vbal.6.1.2.11.1.38` | spec-library | String | partial |  |
| `vbal.6.1.2.11.1.39` | spec-library | String$ | not-started |  |
| `vbal.6.1.2.11.1.41` | spec-library | UCase | not-started |  |
| `vbal.6.1.2.11.1.42` | spec-library | UCase$ | not-started |  |
| `vbal.6.1.3.2.1.1` | spec-library | Clear | partial |  |
| `vbal.6.1.3.2.1.2` | spec-library | Raise | partial |  |
| `vbal.6.1.3.2.2.1` | spec-library | Description | not-started |  |
| `vbal.6.1.3.3.1.1` | spec-library | Load | not-started |  |
| `vbal.6.1.3.3.1.2` | spec-library | Unload | not-started |  |

## Known deviations

Programs whose documented expectation the implementation does not meet (kept failing on purpose).

| Program | Kind | Reason |
| --- | --- | --- |
| `collections/userform_not_provided` | unsupported | UserForm objects need a host that provides forms |
| `file-io/fso_drives_not_provided` | unsupported | FileSystemObject.Drives, GetDrive and DriveExists are not provided: a virtual file system has no drives to describe (error 438) |
| `host/load_unload_form` | unsupported | forms require a host that provides them |
| `language/addressof_without_native` | unsupported | AddressOf produces no callable native pointer without the Declare bridge |
| `spec-coerce/5_5_1_1_longlong_to_double_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: implicit LongLong narrowing is accepted and checked at run time |
| `spec-coerce/5_5_1_1_longlong_to_long_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: implicit LongLong narrowing is accepted and checked at run time |
| `spec-coerce/5_5_1_2_11_empty_to_object_object_required` | wrong | VBA behaviour kept where MS-VBAL text differs: Object = Variant without Set is rejected at analysis (SEM007), not error 424 at run time |
| `spec-coerce/5_5_1_2_13_variant_scalar_to_object_object_required` | wrong | VBA behaviour kept where MS-VBAL text differs: Object = Variant without Set is rejected at analysis (SEM007), not error 424 at run time |
| `spec-coerce/5_5_1_2_2_boolean_to_numeric` | wrong | VBA behaviour kept where MS-VBAL text differs: Byte = True is error 6 |
| `spec-coerce/5_5_1_2_4_number_to_string_small_fraction` | wrong | VBA behaviour kept where MS-VBAL text differs: small fractions format as 1E-05 |
| `spec-coerce/5_5_1_2_4_string_to_numeric` | wrong | not implemented: whitespace between a sign and the digits of a numeric string |
| `spec-coerce/5_5_1_2_6_byte_array_string_round_trip` | wrong | not implemented: Byte() to String * length coercion |
| `spec-coerce/5_5_1_2_9_error_to_other_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-coerce/5_5_1_2_9_error_to_string_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-coerce/5_5_2_2_2_set_empty_variant_to_object_object_required` | wrong | not implemented: error numbers of Set with a non-object Variant (13/424) differ |
| `spec-coerce/5_5_2_2_2_set_variant_scalar_to_object_object_required` | wrong | not implemented: error numbers of Set with a non-object Variant (13/424) differ |
| `spec-coerce/5_5_2_2_2_set_variant_scalar_to_variant_type_mismatch` | wrong | not implemented: error numbers of Set with a non-object Variant (13/424) differ |
| `spec-coerce/5_6_2_1_function_name_inside_function_is_return_value` | wrong | VBA behaviour kept where MS-VBAL text differs: reading the function name inside its own body is a recursive call |
| `spec-coerce/5_6_2_1_function_result_indexed_after_empty_arguments` | wrong | VBA behaviour kept where MS-VBAL text differs: F(1) passes 1 as an argument of F |
| `spec-coerce/5_6_2_1_variant_object_indexed_subscript_out_of_range` | wrong | not implemented: error 9 when indexing a Variant that holds a scalar or object |
| `spec-coerce/5_6_2_1_variant_scalar_indexed_subscript_out_of_range` | wrong | not implemented: error 9 when indexing a Variant that holds a scalar or object |
| `spec-coerce/5_6_2_2_parenthesized_object_without_default_fails` | wrong | not implemented: error 438 for (v) when v holds an object without a default member |
| `spec-coerce/5_6_5_literal_identifier_type_suffix_no_effect` | wrong | not implemented: type suffix on True/False/Empty/Null |
| `spec-coerce/5_6_7_typeof_is_literal_operand_invalid` | wrong | not implemented: static rejection of TypeOf on a non-variable operand |
| `spec-coerce/5_6_7_typeof_is_nothing_raises_91` | wrong | not implemented: error 91 for TypeOf applied to Nothing |
| `spec-coerce/5_6_9_3_1_unary_minus_date_overflow_variant` | wrong | not implemented: Double promotion on Date/Single overflow with a Variant operand |
| `spec-coerce/5_6_9_3_2_plus_date_arithmetic` | wrong | not implemented: Double promotion on Date/Single overflow with a Variant operand |
| `spec-coerce/5_6_9_3_3_minus_date_overflow_variant_double` | wrong | not implemented: Double promotion on Date/Single overflow with a Variant operand |
| `spec-coerce/5_6_9_3_4_multiply_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_5_divide_result_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_6_intdiv_mod_result_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_6_mod_negative_dividend_absolute_value` | wrong | VBA behaviour kept where MS-VBAL text differs: Mod keeps the sign of the dividend (-7 Mod 4 is -3) |
| `spec-coerce/5_6_9_3_7_power_zero_to_negative_invalid_call` | wrong | VBA behaviour kept where MS-VBAL text differs: 0 ^ negative is error 11 |
| `spec-coerce/5_6_9_3_binary_declared_and_effective_float_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_empty_operand_effective_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_error_operand_with_other_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-coerce/5_6_9_3_overflow_date_variant_gives_double` | wrong | not implemented: Double promotion on Date/Single overflow with a Variant operand |
| `spec-coerce/5_6_9_3_overflow_variant_single_widens_to_double` | wrong | not implemented: Double promotion on Date/Single overflow with a Variant operand |
| `spec-data/5_4_3_1_local_same_as_function_name` | wrong | VBA behaviour kept where MS-VBAL text differs: a local may share the name of the enclosing Function or Property Get |
| `spec-data/5_4_3_1_local_same_as_implicit_variable` | wrong | not implemented: conflict between a local declaration and an earlier implicit variable |
| `spec-data/5_4_3_1_local_same_as_property_get_name` | wrong | VBA behaviour kept where MS-VBAL text differs: a local may share the name of the enclosing Function or Property Get |
| `spec-data/5_4_3_3_redim_as_new_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: ReDim ... As New is accepted |
| `spec-data/5_4_3_3_redim_locked_by_byref_error10` | wrong | not implemented: error 10 for ReDim of an array locked by a ByRef argument |
| `spec-data/5_4_3_5_midb_replaces_bytes` | unsupported | the MidB statement |
| `spec-data/5_4_3_6_lset_udt_target_accepted` | unsupported | LSet/RSet between user-defined types |
| `spec-data/5_4_3_8_let_object_variable_without_default_error438` | wrong | VBA behaviour kept where MS-VBAL text differs: misuse of Let on objects and read-only properties is rejected at analysis, not at run time |
| `spec-data/5_4_3_8_let_property_get_only_variant_error451` | wrong | VBA behaviour kept where MS-VBAL text differs: misuse of Let on objects and read-only properties is rejected at analysis, not at run time |
| `spec-data/5_4_4_1_on_error_resets_err` | wrong | VBA behaviour kept where MS-VBAL text differs: On Error does not reset Err |
| `spec-data/5_4_5_10_input_date_without_hash_overflow` | wrong | not implemented: error 6 for Input into a Date from text without # |
| `spec-data/5_4_5_10_input_object_variable_invalid` | wrong | not implemented: static rejection of Object operands in Input/Put/Get |
| `spec-data/5_4_5_11_put_object_data_invalid` | wrong | not implemented: static rejection of Object operands in Input/Put/Get |
| `spec-data/5_4_5_11_put_variant_string_descriptor` | wrong | VBA behaviour kept where MS-VBAL text differs: a Variant String is written with its length descriptor |
| `spec-data/5_4_5_12_get_object_variable_invalid` | wrong | not implemented: static rejection of Object operands in Input/Put/Get |
| `spec-data/5_4_5_1_1_file_number_not_coercible_error52` | wrong | not implemented: error 52 for a non-numeric file number (13 is raised) |
| `spec-data/5_4_5_1_open_append_with_access_read_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_append_with_access_write_accepted` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_input_with_access_read_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_input_with_access_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_len_out_of_range_is_error` | wrong | not implemented: validation of Len above 32767 in Open |
| `spec-data/5_4_5_1_open_output_with_access_read_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_output_with_access_read_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_table_print_on_random_errors` | wrong | VBA behaviour kept where MS-VBAL text differs: Print on a Random file is allowed |
| `spec-data/5_4_5_3_seek_extends_binary_file` | wrong | VBA behaviour kept where MS-VBAL text differs: Seek beyond the end does not extend the file |
| `spec-data/5_4_5_4_lock_record_range_random` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_4_lock_start_greater_than_end_is_error` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_4_lock_start_zero_is_error` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_5_close_removes_locks` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_5_unlock_different_range_is_error` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_5_unlock_never_locked_range_is_error` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_5_unlock_twice_second_is_error` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_5_unlock_whole_file_removes_range_locks` | wrong | not implemented: tracking of Lock/Unlock ranges |
| `spec-data/5_4_5_6_line_input_long_variable_invalid` | wrong | not implemented: static checks of Line Input operands |
| `spec-data/5_4_5_6_line_input_unmarked_number_invalid` | wrong | not implemented: static checks of Line Input operands |
| `spec-data/5_4_5_7_width_wraps_output_lines` | wrong | not implemented: wrapping of a single long output item at the Width line length |
| `spec-data/5_4_5_8_bare_print_not_in_revision3_grammar` | wrong | VBA behaviour kept where MS-VBAL text differs: the file Print statement keeps its form without a Debug./Me. prefix (revision 3.0 grammar taken as additive) |
| `spec-data/5_4_5_8_me_print_form` | unsupported | Me.Print needs a host that provides forms |
| `spec-data/5_4_5_8_print_number_has_surrounding_spaces` | wrong | VBA behaviour kept where MS-VBAL text differs: the sign takes the leading space of a printed number |
| `spec-data/5_4_5_9_write_spc_item` | wrong | not implemented: Write with a trailing semicolon and Spc items |
| `spec-data/5_4_5_9_write_trailing_semicolon_continues_line` | wrong | not implemented: Write with a trailing semicolon and Spc items |
| `spec-decl/5_1_declaration_after_procedure` | wrong | not implemented: a declaration after a procedure is accepted |
| `spec-decl/5_2_3_1_1_as_new_lazy_array` | wrong | VBA behaviour kept where MS-VBAL text differs: As New is rejected for arrays and Type members |
| `spec-decl/5_2_3_1_1_typed_name_with_array` | wrong | not implemented: Dim a&(n) at module level |
| `spec-decl/5_2_3_1_2_withevents_class_without_events` | wrong | not implemented: static checks that a WithEvents class declares events and is not the containing class |
| `spec-decl/5_2_3_1_2_withevents_own_class` | wrong | not implemented: static checks that a WithEvents class declares events and is not the containing class |
| `spec-decl/5_2_3_1_variable_same_name_as_enum_type` | wrong | not implemented: a variable named like an Enum type of the module |
| `spec-decl/5_2_3_3_udt_member_as_new` | wrong | VBA behaviour kept where MS-VBAL text differs: As New is rejected for arrays and Type members |
| `spec-decl/5_2_3_3_udt_name_same_as_module` | wrong | not implemented: uniqueness of public Type/Enum names across modules and modules/types |
| `spec-decl/5_2_3_3_udt_public_duplicate_across_modules` | wrong | not implemented: uniqueness of public Type/Enum names across modules and modules/types |
| `spec-decl/5_2_3_3_udt_reserved_member_name` | wrong | not implemented: reserved words such as Date as Type member names |
| `spec-decl/5_2_3_4_enum_forward_member_reference` | wrong | not implemented: forward references between Enum members and Global Enum in class modules |
| `spec-decl/5_2_3_4_enum_forward_reference_later_enum` | wrong | not implemented: forward references between Enum members and Global Enum in class modules |
| `spec-decl/5_2_3_4_enum_global_in_class_module` | wrong | not implemented: forward references between Enum members and Global Enum in class modules |
| `spec-decl/5_2_3_4_enum_name_same_as_module` | wrong | not implemented: uniqueness of public Type/Enum names across modules and modules/types |
| `spec-decl/5_2_3_4_enum_public_duplicate_across_modules` | wrong | not implemented: uniqueness of public Type/Enum names across modules and modules/types |
| `spec-decl/5_2_3_5_declare_ordinal_alias_not_integer` | wrong | not implemented: validation of #ordinal aliases of Declare |
| `spec-decl/5_2_3_5_declare_ordinal_alias_out_of_range` | wrong | not implemented: validation of #ordinal aliases of Declare |
| `spec-decl/5_2_3_6_circular_const_dependency` | wrong | not implemented: detection of circular module dependencies |
| `spec-decl/5_2_3_6_circular_enum_dependency` | wrong | not implemented: detection of circular module dependencies |
| `spec-decl/5_2_3_6_circular_implements` | wrong | not implemented: detection of circular module dependencies |
| `spec-decl/5_2_4_1_2_global_namespace_class` | unsupported | VB_GlobalNameSpace classes |
| `spec-decl/5_2_4_1_2_global_namespace_shares_default_instance` | unsupported | VB_GlobalNameSpace classes |
| `spec-decl/5_2_4_3_event_name_same_as_member_ok` | wrong | not implemented: event name versus member names, event parameters as procedure variables |
| `spec-decl/5_2_4_3_event_params_define_no_variables` | wrong | not implemented: event name versus member names, event parameters as procedure variables |
| `spec-lex/2_5_1_array_elements_auto_instantiate` | unsupported | arrays declared As New |
| `spec-lex/3_2_1_unicode_line_terminators` | wrong | not implemented: Unicode line terminators and space characters (the lexer is byte-based) |
| `spec-lex/3_2_2_whitespace_dbcs_space_u3000` | wrong | not implemented: Unicode line terminators and space characters (the lexer is byte-based) |
| `spec-lex/3_2_2_whitespace_unicode_zs_u2003` | wrong | not implemented: Unicode line terminators and space characters (the lexer is byte-based) |
| `spec-lex/3_3_4_string_ends_at_line_continuation` | wrong | VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error |
| `spec-lex/3_3_4_string_ends_at_line_end` | wrong | VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error |
| `spec-lib-a/6_1_2_3_1_15_error_no_argument` | wrong | not implemented: Error$ with no argument and the 0..65535 range |
| `spec-lib-a/6_1_2_3_1_15_error_range` | wrong | not implemented: Error$ with no argument and the 0..65535 range |
| `spec-lib-a/6_1_2_3_1_17_hex_long_minus_one` | wrong | VBA behaviour kept where MS-VBAL text differs: Hex of a negative Long has 8 digits |
| `spec-lib-a/6_1_2_3_1_20_str_null_returns_null` | wrong | VBA behaviour kept where MS-VBAL text differs: Str(Null) is error 94 |
| `spec-lib-a/6_1_2_3_1_2_cbyte_boolean_true` | wrong | VBA behaviour kept where MS-VBAL text differs: CByte(True) is error 6 |
| `spec-lib-a/6_1_2_3_1_5_cdbl_sign_whitespace` | wrong | not implemented: whitespace between a sign and the digits of a numeric string |
| `spec-lib-a/6_1_2_4_1_11_timeserial` | wrong | not implemented: TimeSerial minute overflow carrying into the date part |
| `spec-lib-a/6_1_2_4_1_3_datepart_intervals` | wrong | VBA behaviour kept where MS-VBAL text differs: DatePart returns a Long |
| `spec-lib-a/6_1_2_4_1_6_day_errors` | wrong | not implemented: error 13 for Day/Year of an Error value |
| `spec-lib-b/6_1_2_5_1_3_eof_binary_short_get` | wrong | VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record |
| `spec-lib-b/6_1_2_5_1_3_eof_output_true` | wrong | VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record |
| `spec-lib-b/6_1_2_5_1_6_filelen_open_file_size_before_open` | wrong | VBA behaviour kept where MS-VBAL text differs: FileLen of an open file is its current size |
| `spec-lib-b/6_1_2_5_2_7_setattr_sets_and_open_errors` | wrong | not implemented: error when SetAttr names an open file |
| `spec-lib-b/6_1_2_7_1_8_isnumeric_empty_false` | wrong | VBA behaviour kept where MS-VBAL text differs: IsNumeric accepts numeric strings and Empty |
| `spec-lib-b/6_1_2_7_1_8_isnumeric_string_false` | wrong | VBA behaviour kept where MS-VBAL text differs: IsNumeric accepts numeric strings and Empty |
| `spec-lib-c/6_1_2_8_1_14_partition_interval_one` | wrong | VBA behaviour kept where MS-VBAL text differs: Partition pads to Len(Stop) as the Learn page shows |
| `spec-lib-c/6_1_2_8_1_14_partition_last_range_extends_to_stop` | wrong | VBA behaviour kept where MS-VBAL text differs: Partition pads to Len(Stop) as the Learn page shows |
| `spec-lib-c/6_1_2_8_1_14_partition_ranges_and_padding` | wrong | VBA behaviour kept where MS-VBAL text differs: Partition pads to Len(Stop) as the Learn page shows |
| `spec-lib-c/6_1_2_8_1_7_getallsettings_array_shape` | wrong | VBA behaviour kept where MS-VBAL text differs: GetAllSettings returns a zero-based array |
| `spec-lib-d/6_1_2_11_1_15_instrrev_null_arguments` | wrong | VBA behaviour kept where MS-VBAL text differs: Null arguments are error 94 |
| `spec-lib-d/6_1_2_11_1_37_strconv_propercase_word_separators` | wrong | VBA behaviour kept where MS-VBAL text differs: proper case also capitalises after a hyphen |
| `spec-lib-d/6_1_2_11_1_38_string_function_null` | wrong | VBA behaviour kept where MS-VBAL text differs: Null arguments are error 94 |
| `spec-lib-d/6_1_2_11_1_5_chrb_range_overflow6` | wrong | not implemented: ChrB range error 6 |
| `spec-lib-d/6_1_2_11_1_5_chrb_single_byte` | unsupported | ChrB yields a one-byte string |
| `spec-lib-d/6_1_2_11_1_6_chrw_range_error5` | wrong | VBA behaviour kept where MS-VBAL text differs: ChrW(-32768) is accepted |
| `spec-lib-d/6_1_3_2_err_clear_on_error_statement` | wrong | VBA behaviour kept where MS-VBAL text differs: On Error does not clear Err |
| `spec-lib-d/6_1_3_2_err_raise_reuses_uncleared_values` | wrong | not implemented: Err.Raise reusing uncleared Err values |
| `spec-ops/5_6_10_name_left_exception` | wrong | not implemented: library functions named Left/Mid/... shadowed by a project procedure |
| `spec-ops/5_6_12_member_access_type_char_mismatch_invalid` | wrong | not implemented: type character check on member access |
| `spec-ops/5_6_13_1_arglist_trailing_comma_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: a trailing comma in an argument list is accepted |
| `spec-ops/5_6_13_index_default_member_recursion` | wrong | not implemented: recursive default member resolution for index expressions |
| `spec-ops/5_6_13_index_object_named_argument_invalid` | wrong | not implemented: static rejection of named arguments in an index expression on an Object |
| `spec-ops/5_6_16_1_const_expr_intrinsics` | wrong | VBA behaviour kept where MS-VBAL text differs: function calls and Like are rejected in Const initialisers |
| `spec-ops/5_6_16_1_const_expr_like_and_relational` | wrong | VBA behaviour kept where MS-VBAL text differs: function calls and Like are rejected in Const initialisers |
| `spec-ops/5_6_16_2_cc_expr_operators` | wrong | not implemented: function calls and Like in conditional compilation expressions |
| `spec-ops/5_6_9_4_concat_error_value_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_currency_coerces_double` | wrong | not implemented: effective-type rules for comparisons involving Single/Currency/Decimal/LongLong and String |
| `spec-ops/5_6_9_5_rel_decimal_coerces_double` | wrong | not implemented: effective-type rules for comparisons involving Single/Currency/Decimal/LongLong and String |
| `spec-ops/5_6_9_5_rel_error_values` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_error_vs_other_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_longlong_exact` | wrong | not implemented: effective-type rules for comparisons involving Single/Currency/Decimal/LongLong and String |
| `spec-ops/5_6_9_5_rel_other_vs_error_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_single_double_is_single` | wrong | not implemented: effective-type rules for comparisons involving Single/Currency/Decimal/LongLong and String |
| `spec-ops/5_6_9_5_rel_string_variant_is_string_new30` | wrong | not implemented: String/Variant row added in revision 3.0 (ambiguous with the Integer/String row; numeric comparison kept) |
| `spec-ops/5_6_9_6_like_error_only_if_reached` | wrong | not implemented: Like validates the whole pattern up front |
| `spec-ops/5_6_9_6_like_star_not_reached_no_error` | wrong | not implemented: Like validates the whole pattern up front |
| `spec-ops/5_6_9_8_imp_null_table` | wrong | VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null |
| `spec-ops/5_6_9_8_logical_error_value_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_8_not_error_value_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_8_or_null_table` | wrong | VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null |
| `spec-proc/5_3_1_11_byref_object_param_class_arg_copy_in_out` | wrong | VBA behaviour kept where MS-VBAL text differs: ByRef arguments of a different declared type are rejected or copied |
| `spec-proc/5_3_1_11_byref_variant_accepts_any_variable` | wrong | VBA behaviour kept where MS-VBAL text differs: ByRef arguments of a different declared type are rejected or copied |
| `spec-proc/5_3_1_11_byref_variant_param_keeps_arg_declared_type_on_assign` | wrong | VBA behaviour kept where MS-VBAL text differs: ByRef arguments of a different declared type are rejected or copied |
| `spec-proc/5_3_1_11_byval_object_param_scalar_arg_invalid` | wrong | not implemented: static rejection of a scalar passed to a ByVal Object parameter |
| `spec-proc/5_3_1_11_late_bound_arg_mapped_twice_error_448` | wrong | not implemented: error numbers of late-bound named/omitted arguments |
| `spec-proc/5_3_1_11_late_bound_omitted_required_error_448` | wrong | not implemented: error numbers of late-bound named/omitted arguments |
| `spec-proc/5_3_1_2_static_trailing_keeps_locals` | wrong | not implemented: trailing Static in a procedure header |
| `spec-proc/5_3_1_5_optional_deftype_requires_default_invalid` | wrong | not implemented: Optional parameter default rule under Def directives |
| `spec-proc/5_3_1_8_event_handler_mechanism_mismatch_invalid` | wrong | not implemented: event handler declaration rules |
| `spec-proc/5_3_1_8_event_handler_must_be_sub_invalid` | wrong | not implemented: event handler declaration rules |
| `spec-proc/5_3_1_function_private_enum_type_invalid` | wrong | not implemented: private types as function result types |
| `spec-proc/5_3_1_function_private_udt_in_class_invalid` | wrong | not implemented: private types as function result types |
| `spec-proc/5_3_1_typed_name_with_function_type_invalid` | wrong | not implemented: a typed function name with an As clause |
| `spec-proc/5_4_1_1_label_named_randomize_invalid` | wrong | not implemented: statement keywords as label names |
| `spec-proc/5_4_2_1_call_object_default_property_error_450` | wrong | not implemented: error 450 for Call on an object whose default member is a property |
| `strings/strconv_unicode_roundtrip` | unsupported | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
