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
| implemented | 575 |
| partial | 111 |
| unsupported | 5 |
| not-started | 17 |
| Conformance programs | 2174 |
| Programs that are known deviations (XFAIL) | 94 |

## Entries per category and status

| Category | Total | implemented | partial | unsupported | not-started |
| --- | ---: | ---: | ---: | ---: | ---: |
| statement | 76 | 65 | 9 | 2 | 0 |
| function-conversion | 8 | 6 | 2 | 0 | 0 |
| function-math | 13 | 13 | 0 | 0 | 0 |
| function-type-conversion | 13 | 12 | 1 | 0 | 0 |
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
| operator-arithmetic | 8 | 2 | 6 | 0 | 0 |
| operator-concatenation | 1 | 0 | 1 | 0 | 0 |
| operator-comparison | 8 | 5 | 3 | 0 | 0 |
| operator-logical | 6 | 2 | 4 | 0 | 0 |
| operator-precedence | 1 | 1 | 0 | 0 | 0 |
| data-type | 18 | 18 | 0 | 0 | 0 |
| identifier-type-character | 7 | 7 | 0 | 0 | 0 |
| object | 13 | 10 | 0 | 3 | 0 |
| language-element | 38 | 29 | 9 | 0 | 0 |
| spec-environment | 5 | 5 | 0 | 0 | 0 |
| spec-lexical | 12 | 11 | 1 | 0 | 0 |
| spec-organization | 3 | 1 | 0 | 0 | 2 |
| spec-declarations | 21 | 17 | 4 | 0 | 0 |
| spec-procedures | 12 | 10 | 1 | 0 | 1 |
| spec-statements | 55 | 45 | 10 | 0 | 0 |
| spec-coercion | 18 | 12 | 6 | 0 | 0 |
| spec-expressions | 53 | 27 | 21 | 0 | 5 |
| spec-library | 193 | 164 | 20 | 0 | 9 |

## Entries per provenance

| Provenance | Entries |
| --- | ---: |
| oracle | 0 |
| spec | 435 |
| doc | 99 |
| derived | 57 |
| unverified | 100 |
| (no program) | 17 |

## Programs per area and provenance

| Area | Programs | oracle | spec | doc | derived | unverified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| arrays | 35 | 0 | 7 | 10 | 7 | 11 |
| classes | 30 | 0 | 12 | 6 | 9 | 3 |
| collections | 25 | 0 | 7 | 14 | 2 | 2 |
| conversions | 39 | 0 | 28 | 4 | 2 | 5 |
| datatypes | 52 | 0 | 27 | 16 | 0 | 9 |
| datetime | 48 | 0 | 18 | 14 | 5 | 11 |
| derived-math | 21 | 0 | 0 | 21 | 0 | 0 |
| errors | 27 | 0 | 18 | 6 | 1 | 2 |
| events | 7 | 0 | 6 | 0 | 1 | 0 |
| file-io | 48 | 0 | 19 | 19 | 3 | 7 |
| financial | 20 | 0 | 0 | 2 | 18 | 0 |
| host | 42 | 0 | 3 | 15 | 10 | 14 |
| language | 20 | 0 | 6 | 6 | 0 | 8 |
| logic | 11 | 0 | 7 | 3 | 1 | 0 |
| math | 37 | 0 | 24 | 4 | 2 | 7 |
| operators | 66 | 0 | 59 | 1 | 5 | 1 |
| procedures | 34 | 0 | 23 | 2 | 4 | 5 |
| spec-coerce | 188 | 0 | 187 | 0 | 1 | 0 |
| spec-data | 289 | 0 | 279 | 0 | 8 | 2 |
| spec-decl | 168 | 0 | 168 | 0 | 0 | 0 |
| spec-lex | 145 | 0 | 145 | 0 | 0 | 0 |
| spec-lib-a | 97 | 0 | 96 | 0 | 1 | 0 |
| spec-lib-b | 63 | 0 | 62 | 1 | 0 | 0 |
| spec-lib-c | 58 | 0 | 58 | 0 | 0 | 0 |
| spec-lib-d | 88 | 0 | 74 | 0 | 14 | 0 |
| spec-ops | 132 | 0 | 132 | 0 | 0 | 0 |
| spec-proc | 222 | 0 | 221 | 0 | 1 | 0 |
| statements | 89 | 0 | 75 | 13 | 1 | 0 |
| strings | 73 | 0 | 33 | 17 | 8 | 15 |

## Gaps

Entries that are not `implemented`, with the reason recorded in the inventory.

| Id | Category | Name | Status | Notes |
| --- | --- | --- | --- | --- |
| `stmt.call` | statement | Call | partial |  |
| `stmt.dim` | statement | Dim | partial |  |
| `stmt.let` | statement | Let | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `stmt.load` | statement | Load | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `stmt.on-error` | statement | On Error | partial |  |
| `stmt.open` | statement | Open | partial |  |
| `stmt.print` | statement | Print # | partial |  |
| `stmt.put` | statement | Put | partial |  |
| `stmt.redim` | statement | ReDim | partial | element assignment into an array held in a Variant (ReDim v(..); v(i) = x) is rejected as 'late binding' |
| `stmt.seek` | statement | Seek | partial |  |
| `stmt.unload` | statement | Unload | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `func.hex` | function-conversion | Hex | partial |  |
| `func.str` | function-conversion | Str | partial |  |
| `func.cbyte` | function-type-conversion | CByte | partial |  |
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
| `op.mul` | operator-arithmetic | * | partial |  |
| `op.div` | operator-arithmetic | / | partial |  |
| `op.intdiv` | operator-arithmetic | \ | partial |  |
| `op.mod` | operator-arithmetic | Mod | partial |  |
| `op.add` | operator-arithmetic | + | partial |  |
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
| `expr.default-member` | language-element | Default member resolution | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `expr.implicit-conv` | language-element | Implicit conversion in expressions | partial |  |
| `lex.continuation` | language-element | Line continuation | partial |  |
| `cc.const` | language-element | Conditional compilation constants (#Const) | partial |  |
| `decl.byref-byval` | language-element | ByRef and ByVal | partial |  |
| `vbal.3.3.4` | spec-lexical | String Tokens | partial |  |
| `vbal.4.1` | spec-organization | Projects | not-started |  |
| `vbal.4.2.1` | spec-organization | Module Extensibility | not-started |  |
| `vbal.5.2.3.1` | spec-declarations | Module Variable Declaration Lists | partial |  |
| `vbal.5.2.3.3` | spec-declarations | User Defined Type Declarations | partial |  |
| `vbal.5.2.4.1.2` | spec-declarations | Default Instance Variables Static Semantics | partial |  |
| `vbal.5.2.4.3` | spec-declarations | Event Declaration | partial |  |
| `vbal.5.3.1.3` | spec-procedures | Procedure Names | not-started |  |
| `vbal.5.3.1.11` | spec-procedures | Procedure Invocation Argument Processing | partial |  |
| `vbal.5.4.1.1` | spec-statements | Statement Labels | partial |  |
| `vbal.5.4.2.1` | spec-statements | Call Statement | partial |  |
| `vbal.5.4.3.1` | spec-statements | Local Variable Declarations | partial |  |
| `vbal.5.4.3.3` | spec-statements | ReDim Statement | partial |  |
| `vbal.5.4.3.8` | spec-statements | Let Statement | partial |  |
| `vbal.5.4.4.1` | spec-statements | On Error Statement | partial |  |
| `vbal.5.4.5.1` | spec-statements | Open Statement | partial |  |
| `vbal.5.4.5.3` | spec-statements | Seek Statement | partial |  |
| `vbal.5.4.5.8` | spec-statements | Print Statement | partial |  |
| `vbal.5.4.5.11` | spec-statements | Put Statement | partial |  |
| `vbal.5.5.1.1` | spec-coercion | Static semantics | partial |  |
| `vbal.5.5.1.2.2` | spec-coercion | Let-coercion to and from Boolean | partial |  |
| `vbal.5.5.1.2.4` | spec-coercion | Let-coercion to and from String | partial |  |
| `vbal.5.5.1.2.9` | spec-coercion | Let-coercion to and from Error | partial |  |
| `vbal.5.5.1.2.11` | spec-coercion | Let-coercion from Empty | partial |  |
| `vbal.5.5.1.2.13` | spec-coercion | Let-coercion to and from a class or Object or Nothing | partial |  |
| `vbal.5.6.1` | spec-expressions | Expression Classifications | not-started |  |
| `vbal.5.6.2.1` | spec-expressions | Evaluation to a data value | partial |  |
| `vbal.5.6.2.2` | spec-expressions | Evaluation to a simple data value | partial |  |
| `vbal.5.6.2.3` | spec-expressions | Default Member Recursion Limits | not-started |  |
| `vbal.5.6.4` | spec-expressions | Expression Binding Contexts | not-started |  |
| `vbal.5.6.5` | spec-expressions | Literal Expressions | partial |  |
| `vbal.5.6.6` | spec-expressions | Parenthesized Expressions | partial |  |
| `vbal.5.6.7` | spec-expressions | TypeOf…Is Expressions | partial |  |
| `vbal.5.6.9.3` | spec-expressions | Arithmetic Operators | partial |  |
| `vbal.5.6.9.3.4` | spec-expressions | * Operator | partial |  |
| `vbal.5.6.9.3.5` | spec-expressions | / Operator | partial |  |
| `vbal.5.6.9.3.6` | spec-expressions | \ Operator and Mod Operator | partial |  |
| `vbal.5.6.9.3.7` | spec-expressions | ^ Operator | partial |  |
| `vbal.5.6.9.4` | spec-expressions | & Operator | partial |  |
| `vbal.5.6.9.5` | spec-expressions | Relational Operators | partial |  |
| `vbal.5.6.9.8` | spec-expressions | Logical Operators | partial |  |
| `vbal.5.6.9.8.1` | spec-expressions | Not Operator | partial |  |
| `vbal.5.6.9.8.3` | spec-expressions | Or Operator | partial |  |
| `vbal.5.6.9.8.6` | spec-expressions | Imp Operator | partial |  |
| `vbal.5.6.10` | spec-expressions | Simple Name Expressions | partial |  |
| `vbal.5.6.13` | spec-expressions | Index Expressions | partial |  |
| `vbal.5.6.13.1` | spec-expressions | Argument Lists | partial |  |
| `vbal.5.6.13.2` | spec-expressions | Argument List Queues | not-started |  |
| `vbal.5.6.16.1` | spec-expressions | Constant Expressions | partial |  |
| `vbal.5.6.16.2` | spec-expressions | Conditional Compilation Expressions | partial |  |
| `vbal.5.6.16.8` | spec-expressions | AddressOf Expressions | not-started |  |
| `vbal.6.1.2.3.1.2` | spec-library | CByte | partial |  |
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
| `vbal.6.1.2.11.1.37` | spec-library | StrConv | partial |  |
| `vbal.6.1.2.11.1.38` | spec-library | String | partial |  |
| `vbal.6.1.3.2.1.1` | spec-library | Clear | partial |  |
| `vbal.6.1.3.3.1.1` | spec-library | Load | not-started |  |
| `vbal.6.1.3.3.1.2` | spec-library | Unload | not-started |  |

## Known deviations

Programs whose documented expectation the implementation does not meet (kept failing on purpose).

| Program | Kind | Reason |
| --- | --- | --- |
| `collections/userform_not_provided` | unsupported | UserForm objects need a host that provides forms |
| `file-io/fso_drives_not_provided` | unsupported | FileSystemObject.Drives, GetDrive and DriveExists are not provided: a virtual file system has no drives to describe (error 438) |
| `host/load_unload_form` | unsupported | forms require a host that provides them |
| `spec-coerce/5_5_1_1_longlong_to_double_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: implicit LongLong narrowing is accepted and checked at run time |
| `spec-coerce/5_5_1_1_longlong_to_long_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: implicit LongLong narrowing is accepted and checked at run time |
| `spec-coerce/5_5_1_2_11_empty_to_object_object_required` | wrong | VBA behaviour kept where MS-VBAL text differs: Object = Variant without Set is rejected at analysis (SEM007), not error 424 at run time |
| `spec-coerce/5_5_1_2_13_variant_scalar_to_object_object_required` | wrong | VBA behaviour kept where MS-VBAL text differs: Object = Variant without Set is rejected at analysis (SEM007), not error 424 at run time |
| `spec-coerce/5_5_1_2_2_boolean_to_numeric` | wrong | VBA behaviour kept where MS-VBAL text differs: Byte = True is error 6 |
| `spec-coerce/5_5_1_2_4_number_to_string_small_fraction` | wrong | VBA behaviour kept where MS-VBAL text differs: small fractions format as 1E-05 |
| `spec-coerce/5_5_1_2_9_error_to_other_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-coerce/5_5_1_2_9_error_to_string_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-coerce/5_6_2_1_function_name_inside_function_is_return_value` | wrong | VBA behaviour kept where MS-VBAL text differs: reading the function name inside its own body is a recursive call |
| `spec-coerce/5_6_2_1_function_result_indexed_after_empty_arguments` | wrong | VBA behaviour kept where MS-VBAL text differs: F(1) passes 1 as an argument of F |
| `spec-coerce/5_6_2_1_variant_object_indexed_subscript_out_of_range` | wrong | VBA behaviour kept where MS-VBAL text differs: a Variant holding an object followed by an argument list calls the object's default member (the Learn page on default members documents it) instead of raising error 9 |
| `spec-coerce/5_6_2_2_parenthesized_object_without_default_fails` | wrong | not implemented: error 438 for (v) when v holds an object without a default member |
| `spec-coerce/5_6_5_literal_identifier_type_suffix_no_effect` | wrong | not implemented: type suffix on True/False/Empty/Null |
| `spec-coerce/5_6_7_typeof_is_nothing_raises_91` | wrong | VBA behaviour kept where MS-VBAL text differs: TypeOf applied to a Nothing variable is False, not error 91 (the idiom `If TypeOf x Is T` relies on it) |
| `spec-coerce/5_6_9_3_4_multiply_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_5_divide_result_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_6_intdiv_mod_result_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_6_mod_negative_dividend_absolute_value` | wrong | VBA behaviour kept where MS-VBAL text differs: Mod keeps the sign of the dividend (-7 Mod 4 is -3) |
| `spec-coerce/5_6_9_3_7_power_zero_to_negative_invalid_call` | wrong | VBA behaviour kept where MS-VBAL text differs: 0 ^ negative is error 11 |
| `spec-coerce/5_6_9_3_binary_declared_and_effective_float_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_empty_operand_effective_types` | wrong | VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3 |
| `spec-coerce/5_6_9_3_error_operand_with_other_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-data/5_4_3_1_local_same_as_function_name` | wrong | VBA behaviour kept where MS-VBAL text differs: a local may share the name of the enclosing Function or Property Get |
| `spec-data/5_4_3_1_local_same_as_implicit_variable` | wrong | not implemented: conflict between a local declaration and an earlier implicit variable |
| `spec-data/5_4_3_1_local_same_as_property_get_name` | wrong | VBA behaviour kept where MS-VBAL text differs: a local may share the name of the enclosing Function or Property Get |
| `spec-data/5_4_3_3_redim_as_new_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: ReDim ... As New is accepted |
| `spec-data/5_4_3_8_let_object_variable_without_default_error438` | wrong | VBA behaviour kept where MS-VBAL text differs: misuse of Let on objects and read-only properties is rejected at analysis, not at run time |
| `spec-data/5_4_3_8_let_property_get_only_variant_error451` | wrong | VBA behaviour kept where MS-VBAL text differs: misuse of Let on objects and read-only properties is rejected at analysis, not at run time |
| `spec-data/5_4_4_1_on_error_resets_err` | wrong | VBA behaviour kept where MS-VBAL text differs: On Error does not reset Err |
| `spec-data/5_4_5_11_put_variant_string_descriptor` | wrong | VBA behaviour kept where MS-VBAL text differs: a Variant String is written with its length descriptor |
| `spec-data/5_4_5_1_open_append_with_access_read_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_append_with_access_write_accepted` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_input_with_access_read_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_input_with_access_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_output_with_access_read_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_open_output_with_access_read_write_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: incompatible Open mode/access pairs fail at run time, not at analysis |
| `spec-data/5_4_5_1_table_print_on_random_errors` | wrong | VBA behaviour kept where MS-VBAL text differs: Print on a Random file is allowed |
| `spec-data/5_4_5_3_seek_extends_binary_file` | wrong | VBA behaviour kept where MS-VBAL text differs: Seek beyond the end does not extend the file |
| `spec-data/5_4_5_8_bare_print_not_in_revision3_grammar` | wrong | VBA behaviour kept where MS-VBAL text differs: the file Print statement keeps its form without a Debug./Me. prefix (revision 3.0 grammar taken as additive) |
| `spec-data/5_4_5_8_me_print_form` | unsupported | Me.Print needs a host that provides forms |
| `spec-data/5_4_5_8_print_number_has_surrounding_spaces` | wrong | VBA behaviour kept where MS-VBAL text differs: the sign takes the leading space of a printed number |
| `spec-decl/5_2_3_1_variable_same_name_as_enum_type` | wrong | not implemented: a module-level variable and an Enum of the same name are one namespace here (SEM001 duplicate declaration); VBA is believed to do the same, unconfirmed |
| `spec-decl/5_2_3_3_udt_member_as_new` | wrong | VBA behaviour kept where MS-VBAL text differs: As New is rejected for Type members (VBA is believed to reject it; no oracle result yet) |
| `spec-decl/5_2_4_1_2_global_namespace_class` | unsupported | VB_GlobalNameSpace classes |
| `spec-decl/5_2_4_1_2_global_namespace_shares_default_instance` | unsupported | VB_GlobalNameSpace classes |
| `spec-decl/5_2_4_3_event_name_same_as_member_ok` | wrong | not implemented: events share the symbol table with variables, constants and procedures, so an event named like a member is a duplicate declaration (the specification text allows it; whether VBA accepts it is unconfirmed) |
| `spec-lex/3_3_4_string_ends_at_line_continuation` | wrong | VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error |
| `spec-lex/3_3_4_string_ends_at_line_end` | wrong | VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error |
| `spec-lib-a/6_1_2_3_1_15_error_range` | wrong | VBA behaviour kept where MS-VBAL text differs: an argument outside 0..65535 is error 5 (the expectation of error 6 for 65536 and of success for negatives is not confirmed against real VBA) |
| `spec-lib-a/6_1_2_3_1_17_hex_long_minus_one` | wrong | VBA behaviour kept where MS-VBAL text differs: Hex of a negative Long has 8 digits |
| `spec-lib-a/6_1_2_3_1_20_str_null_returns_null` | wrong | VBA behaviour kept where MS-VBAL text differs: Str(Null) is error 94 |
| `spec-lib-a/6_1_2_3_1_2_cbyte_boolean_true` | wrong | VBA behaviour kept where MS-VBAL text differs: CByte(True) is error 6 |
| `spec-lib-a/6_1_2_4_1_11_timeserial` | wrong | not implemented: TimeSerial minute overflow carrying into the date part |
| `spec-lib-a/6_1_2_4_1_3_datepart_intervals` | wrong | VBA behaviour kept where MS-VBAL text differs: DatePart returns a Long |
| `spec-lib-a/6_1_2_4_1_6_day_errors` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-lib-b/6_1_2_5_1_3_eof_binary_short_get` | wrong | VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record |
| `spec-lib-b/6_1_2_5_1_3_eof_output_true` | wrong | VBA behaviour kept where MS-VBAL text differs: EOF on an Output file is error 54; EOF is True after the last full record |
| `spec-lib-b/6_1_2_5_1_6_filelen_open_file_size_before_open` | wrong | VBA behaviour kept where MS-VBAL text differs: FileLen of an open file is its current size |
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
| `spec-ops/5_6_10_name_left_exception` | wrong | not implemented: library functions named Left/Mid/... shadowed by a project procedure |
| `spec-ops/5_6_13_1_arglist_trailing_comma_invalid` | wrong | VBA behaviour kept where MS-VBAL text differs: a trailing comma in an argument list is accepted |
| `spec-ops/5_6_13_index_object_named_argument_invalid` | wrong | not implemented: static rejection of named arguments in an index expression on an Object |
| `spec-ops/5_6_16_1_const_expr_intrinsics` | wrong | VBA behaviour kept where MS-VBAL text differs: function calls and Like are rejected in Const initialisers |
| `spec-ops/5_6_16_1_const_expr_like_and_relational` | wrong | VBA behaviour kept where MS-VBAL text differs: function calls and Like are rejected in Const initialisers |
| `spec-ops/5_6_16_2_cc_expr_operators` | wrong | VBA behaviour kept where MS-VBAL text differs: function calls and Like are rejected in conditional compilation expressions, as in Const initialisers |
| `spec-ops/5_6_9_4_concat_error_value_type_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_error_values` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_error_vs_other_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_other_vs_error_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_5_rel_single_double_is_single` | wrong | VBA behaviour kept where MS-VBAL text differs: a Single compared with a Double (or with a numeric String) compares as Double, so CSng(0.1) = 0.1 is False |
| `spec-ops/5_6_9_5_rel_string_variant_is_string_new30` | wrong | not implemented: String/Variant row added in revision 3.0 (ambiguous with the Integer/String row; numeric comparison kept) |
| `spec-ops/5_6_9_8_imp_null_table` | wrong | VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null |
| `spec-ops/5_6_9_8_logical_error_value_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_8_not_error_value_mismatch` | wrong | VBA behaviour kept where MS-VBAL text differs: an Error operand re-raises its own error code instead of error 13 |
| `spec-ops/5_6_9_8_or_null_table` | wrong | VBA behaviour kept where MS-VBAL text differs: Or/Imp with a Null operand yield Null |
| `spec-proc/5_3_1_11_byref_object_param_class_arg_copy_in_out` | wrong | VBA behaviour kept where MS-VBAL text differs: a ByRef Object parameter given a variable of a specific class is rejected (SEM026 ByRef argument type mismatch; VBA is believed to reject it at compile time, unconfirmed); only ByRef Variant parameters take a copy |
| `spec-proc/5_4_1_1_label_named_randomize_invalid` | wrong | not implemented: Randomize is not a reserved word of the lexer, so `Randomize:` is accepted as a label (VBA may read it as the Randomize statement; unconfirmed) |
| `spec-proc/5_4_2_1_call_object_default_property_error_450` | wrong | not implemented: Call on an object variable whose default member is a property runs the property instead of raising error 450; the behaviour of VBA is unconfirmed |
| `strings/strconv_unicode_roundtrip` | unsupported | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |

## Running the corpus under another engine

The programs, the header format and the runner live in the `vba-conformance` crate
(`tooling/vba-conformance`), not in the facade. This is deliberate: a `conformance` feature on
the `vba` facade would put test tooling (file walking, header parsing) in the public API
surface that embedding applications depend on, while a dev-only crate costs them nothing and
can change freely. The repository's own test (`crates/api/vba-rs/tests/conformance.rs`) uses it
with `Engine::new` and owns the inventory and this document, which stay byte-identical for the
default engine.

An embedding application adds `vba-conformance` as a path or git dev-dependency and runs the
corpus (a checkout of `tests/vba/`) with its own engine configuration:

```rust
let options = vba_conformance::Options {
    filter: vec!["strings/".into()],                       // optional: ids containing any of these
    provides: Some(["host-clock".to_owned()].into()),     // optional: what this host provides
};
let report = vba_conformance::run_corpus(corpus_dir, || my_engine(), &options)?;
assert!(report.is_ok(), "{}\n{}", report.summary(), report.failures().join("\n"));
```

Semantics:

* The factory is called once per program, so no state leaks between programs. The program's
  own `STRICT_SPEC`, `GRANT`, `CLOCK` and `MAX_STEPS` headers are applied on top of the engine
  the factory returns (they override the same settings of the factory's engine).
* `' REQUIRES: a, b` (repeatable) names host features the program needs. A feature is an
  arbitrary token the embedding application chooses to provide (for example `host-clock`);
  the repository's programs use none yet. Every `GRANT` also implies the feature
  `grant:<capability>` (for example `grant:file-write`).
* With `provides: None` (the default, used by the repository test) every feature is
  provided and nothing is skipped. With `provides: Some(set)`, a program that needs a
  feature outside the set is skipped and reported as `Outcome::Skipped` with the missing
  features; it is neither a pass nor a failure. A skipped program is never run, so it cannot
  fail under a restricted host.
* A program with `XFAIL` must still fail, and one that now passes is reported as
  `Outcome::UnexpectedPass`, exactly as in the repository test. Skipping takes precedence.
* The expected values were produced for the default engine. An embedding application that
  changes language behavior (not just host features) can see real differences; those are
  reported as failures, not hidden.
