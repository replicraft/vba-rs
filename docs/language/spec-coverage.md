# MS-VBAL coverage

Status of the implementation against the VBA language specification MS-VBAL, protocol
revision 3.0 (major revision, 2026-08-18; previous revision 2.4, 2025-05-20). The
specification text was read in full for chapters 3 to 6; this page cites section numbers and
short rule names only. Every statement below is backed by a conformance program under
`tests/vba/spec-*` (sources `spec ms-vbal <section>`) or is listed as a gap.

The numbering used everywhere in this repository is the document's own: 2 computational
environment, 3 lexical rules, 4 program organisation, 5 module bodies (5.2 declarations, 5.3
procedures, 5.4 statements, 5.5 coercion, 5.6 expressions), 6 standard library, 7 change
tracking. `tests/inventory.csv` has one `vbal.<section>` entry per testable section; the
generated report is `docs/tooling/conformance.md`.

## What changed in revision 3.0

The change-tracking section lists five edits relative to 2.4:

| Section | Change | Status here |
| --- | --- | --- |
| 5.4.5.8 Print statement | grammar now `("Debug"/"Me") "." ("Print"/"?") [file number ","] [list]` | `Debug.Print`/`Debug.?` with an optional `#n,` are accepted and write to the file (covered). The old bare `Print #n` form and `Me.Print` are kept as in every earlier revision (the bare form is what all VBA versions accept; see deliberate differences). `Me.Print` needs a form host: unsupported. |
| 5.5.1.2.1 Let-coercion between numeric types | rounding rule applies to floating and fixed point sources (was: integral) | Banker's rounding from Single, Double, Currency and Decimal to integral types is covered and passes. |
| 5.6.9.5 Relational operators | rows for Single/LongLong with Double effective type; two String/Variant rows | Single/LongLong row followed. The String/Variant (not Null) row is not followed: the text is ambiguous with the Integer/String row and a numeric Variant is still compared numerically (known deviation `ops/5_6_9_5_rel_string_variant_is_string_new30`). |
| 5.6.9.5 Relational operators | Double effective type rows (minor) | covered |
| 6.1.2.8.1.13 MsgBox | `vbDefaultButton4` (768) added to the default-button group | defined and tested. |

Older VBA versions behave per older revisions only where the specification itself says so; it
does not distinguish versions in any rule we test, so the profile model needed no change.

## Per chapter

Entries are the `vbal.*` inventory entries (one per testable section); "programs" counts
program-to-entry links. Entry status follows the programs: implemented means every program of
the entry passes, partial that at least one is a recorded known deviation, not-started that no
program exists.

| Chapter | Entries | Implemented | Partial / unsupported | Not started | Programs |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2 Computational environment | 5 | 4 | 1 | 0 | 16 |
| 3 Lexical rules (3.1-3.3) | 9 | 6 | 3 | 0 | 87 |
| 3.4 Conditional compilation | 3 | 3 | 0 | 0 | 22 |
| 4 Program organisation | 3 | 1 | 0 | 2 | 7 |
| 5.1 Module body | 1 | 0 | 1 | 0 | 2 |
| 5.2 Declarations | 20 | 11 | 9 | 0 | 150 |
| 5.3 Procedures | 12 | 6 | 5 | 1 | 92 |
| 5.4 Statements | 55 | 32 | 22 | 1 | 390 |
| 5.5 Coercion | 18 | 9 | 8 | 1 | 101 |
| 5.6 Expressions | 53 | 22 | 26 | 5 | 225 |
| 6.1.1 Predefined enums | 16 | 16 | 0 | 0 | 16 |
| 6.1.2.1-6.1.2.2 Colour and general constants | 2 | 2 | 0 | 0 | 3 |
| 6.1.2.3 Conversion | 21 | 16 | 5 | 0 | 52 |
| 6.1.2.4 DateTime | 19 | 14 | 5 | 0 | 35 |
| 6.1.2.5 FileSystem | 17 | 14 | 3 | 0 | 28 |
| 6.1.2.6 Financial | 13 | 13 | 0 | 0 | 13 |
| 6.1.2.7 Information | 13 | 11 | 1 | 1 | 22 |
| 6.1.2.8 Interaction | 21 | 13 | 2 | 6 | 35 |
| 6.1.2.9, 6.1.2.12 Key code and system colour constants | 2 | 2 | 0 | 0 | 4 |
| 6.1.2.10 Math | 12 | 12 | 0 | 0 | 20 |
| 6.1.2.11 Strings | 43 | 25 | 5 | 13 | 55 |
| 6.1.3 Predefined classes | 14 | 9 | 2 | 3 | 22 |

Entries marked not started are sections with no testable normative content or only host
behaviour (4.1 projects, 4.2.1 extensibility, 5.3.1.3 names, 5.4.2.11 Stop, 5.6.1 and 5.6.4
informative classifications, 5.6.2.3 and 5.6.16.8 implementation-defined limits, 5.6.13.2,
`Command`, `CreateObject`, `GetObject`, `InputBox`, `AppActivate`, `SendKeys`, `IMEStatus`,
`Load`/`Unload`) and the `$`/`B` variants of string functions that share their programs with
the base function (the base entry covers them). `DDB` is covered by the documented formula
(its formula in the text differs from the documented function).

## Defects found and fixed (revision 3.0 text)

Literals: float literals beyond Single/Double range and radix literals beyond 32 bits without
the `^` suffix are invalid (3.3.2); date literals follow the 3.3.3 grammar (numeric day first,
month-and-year forms, `3 p`, `13:30 PM`, dotted time) and invalid ones are rejected at
analysis; `><`, `=<`, `=>` operator spellings. Conditional compilation (3.4): every `#Const` is
processed, even in excluded blocks; names are unique; type suffixes are ignored; conditions
must be Boolean-coercible and are checked in excluded blocks. Types: `[Integer]` and the other
bracketed built-in type names. Declarations: Def ranges (descending, overlap), empty `Type` and
`Enum`, `Global` procedures in classes, `ByVal` array parameters, parameter named like its
function, Get/Let parameter mechanism, interface prefix and underscore rules, underscore in an
event name, module names over 31 characters, creatable/global-namespace source classes.
Statements: `Go To`/`Go Sub`, `Case < 5` without `Is`, conditions on both `Do` and `Loop`,
label after `Then`, `Mid` start beyond the length, `Width` range, `Debug.Print #n`.
Expressions: `0 / 0` is Overflow; `(x) = 5` and a mismatching type character are errors;
Boolean versus String comparison; `#TRUE#`/`#FALSE#`. Library: 169 predefined constants were
missing (enums, colours, key codes, system colours); `Len` of a numeric variable; `Int`/`Fix`
of a Date; conversion functions on Error values; `FileAttr` optional argument; `ChDrive ""`;
`Err.LastDllError` read-only; `ReDim x(n) As T` on an implicit name.

## Severity of the static rules

Rejecting a program that real VBA runs is worse than accepting one it refuses, and there is
no oracle yet for the static rules read from the specification text. Diagnostics therefore
have two classes. A *spec-strict* diagnostic is a warning by default: the project loads and
runs and the warning is reported (`Session::warnings`, `vbars run` on stderr, `vbars check`).
With `strict_spec` they are errors that block loading: `Engine::strict_spec(true)`,
`SessionBuilder::strict_spec(true)`, `vba_sema::Options::strict_spec`, and
`vbars check|run --strict-spec`. Conformance programs that expect "does not compile" for a
spec-strict rule carry the header `' STRICT_SPEC: true`; each has a sibling `*_warns`
program that checks the default behaviour (warn and run).

| Rule | Default | Rationale |
| --- | --- | --- |
| Def<Type> ranges naming a letter twice (5.2.2) | warning | Real VBA accepts overlapping `DefInt A-D` / `DefLng C-F`; the later directive wins. |
| `Implements` interface prefix that begins another's, or interface member with an underscore (5.2.4.2) | warning | Common in real projects; the implementing name is still resolved by exact match. |
| Empty `Type` or `Enum` (5.2.3.3, 5.2.3.4) | warning | Acceptance of an empty block is uncertain; nothing downstream depends on it. |
| `Global` on a procedure in a class (5.3.1.1) | warning | Treated as `Public`. |
| `ByVal` array parameter (5.3.1.5) | warning | Not checked by older hosts; the parameter keeps its declared mechanism. |
| Parameter named like its function (5.3.1.5) | warning | The name resolves to the parameter inside the body. |
| Get/Let/Set parameter mechanism mismatch (5.3.1.7) | warning | Each procedure keeps its own mechanism. |
| Underscore in an event name (5.2.4.3) | warning | The event is usable; handlers resolve by exact name. |
| Module name longer than 31 characters (4.2) | warning | Names come from the project; nothing truncates them. |
| Source class with `VB_Creatable` / `VB_GlobalNameSpace` True (4.2) | warning | Exported files of real projects carry these attributes. The flags are otherwise ignored. |
| `#Const` name repeated (3.4.1) | warning | Real VBA lets a later `#Const` redefine an earlier one. |
| `#If`/`#ElseIf` condition in an excluded block that is not Boolean-coercible (3.4.2) | warning | An excluded block is not compiled by real VBA. The same condition in a live block is an error. |
| Type character that differs from the declared type, `n$` for a `Long` (5.6.10) | warning | The declared type wins. |
| Assignment to a parenthesized expression, `(x) = 5` (5.6.6) | warning | Assigns to the inner expression. |
| Enum member value referring to a following member of the same enum or a member of a later enum of the module (5.2.3.4) | warning | The member is evaluated lazily and resolves; real VBA's behaviour is unconfirmed. |
| `Global Enum` in a class module (5.2.3.4) | warning | Treated as `Public Enum`. |
| `Declare` alias `#n` whose remainder is not an integer from 0 to 32767 (5.2.3.5) | warning | The alias is kept as written; nothing here calls the library. |
| `WithEvents` variable of its own class or of a class that declares no event (5.2.3.1.2) | warning | The variable is usable; it just never receives an event. |
| Event handler that is not a `Sub`, or whose parameter mechanism differs from the event's (5.3.1.8) | warning | A non-`Sub` is not wired as a handler; each handler keeps its own mechanism. |
| Non-private function whose type is a private enum, or a private user-defined type in a class module (5.3.1) | warning | Callers in other modules cannot name the type; nothing else depends on it. |
| Function name with a type character and an `As` clause (5.3.1) | warning | The `As` clause wins. |
| `Optional` parameter typed only by a Def directive without a default value (5.3.1.5) | warning | The parameter is optional and takes the type's default when omitted. |
| Public `Type` or `Enum` named like a module or like another public type or enum of the project (5.2.3.3, 5.2.3.4) | warning | Resolution of an unqualified use reports the ambiguity (`SEM015`); qualified uses work. |
| Circular module dependency through `Const` or enum member expressions or `Implements` (5.2.3.6) | warning | Constants are evaluated lazily and resolve; a value that depends on itself is an error in every mode. |

These stay errors in every mode because real VBA rejects them (or the program has no
meaning): syntax errors, undefined names under `Option Explicit`, type mismatches, duplicate
declarations, missing `End` statements, assignments to constants, unresolved references or
libraries, bad argument lists, misplaced `Option` statements, declarations after a procedure, `Declare` in a public class
member, `Friend` in a standard module, `Static` at module level, `Implements` with missing
members, `Do` with a condition on both ends, invalid date literals, float and radix literals
out of range, wrong `Exit` kinds, and property
pairing errors other than the parameter mechanism.

## GoTo into a For or For Each body

Control that enters the body of a `For` or `For Each` statement through a label, without the
statement's initialisation having run, raises the trappable run-time error 92 ("For loop not
initialized") when `Next` is reached (5.4.2.3, 5.4.2.4); it is no longer an internal error at
load. The loop state is "initialised" when the registers its initialisation defines are set in
the current activation, so a loop whose initialisation ran earlier in the same procedure call
(and was left by `Exit For` or `GoTo`) is entered again without error; real VBA is assumed to
behave the same but there is no oracle result for it yet.

## Date literals without a year

`#3/4#`, `#Mar 4#` and `#4 March 10:30#` name no year. Real VBA uses the year in which the
code runs, and so does this implementation: analysis validates the literal (the day against a
leap year, so `#2/29#` is accepted) and keeps it as an unresolved-year constant that is never
folded; the interpreter gives it the year of the host clock each time it is evaluated
(`Host` clock; `FixedHost` and the default engine clock give 2000, `Engine::fixed_clock`
another fixed year). `#2/29#` evaluated in a year that is not a leap year is run-time error 13.

## Deliberate differences

Where the text and documented VBA behaviour disagree, the documented behaviour is kept (each
is a known deviation program with its reason in `docs/tooling/conformance.md`):

* An Error value used as an operand re-raises its own code (the text says error 13).
* `Mod` keeps the sign of the dividend; `0 ^ negative` is error 11; `Byte = True` is error 6;
  small fractions print as `1E-05`; `Hex` of a negative `Long` has eight digits.
* `Or`/`Imp` with a Null operand yield Null; `Str(Null)`, `String(Null, ...)`,
  `InStrRev(Null, ...)` are error 94; `IsNumeric` accepts numeric strings and Empty.
* `EOF` on an Output file is error 54; `FileLen` of an open file is its current size; `Print`
  on a Random file is allowed; the sign of a printed number takes the leading space.
* `GetAllSettings` is zero-based; `Partition` pads to `Len(Stop)`.
* `On Error` does not clear `Err`.
* An unterminated string literal is a lexical error; `F(1, )` is accepted; `ReDim ... As New`
  is accepted; a local may share the name of the enclosing function.
* Function calls and `Like` are rejected in `Const` initialisers and in conditional
  compilation expressions (the text allows a set of intrinsics).
* `TypeOf x Is T` with a `Nothing` variable is False (the text says error 91); `Error(n)`
  with `n` outside 0..65535 is error 5 (the text's error 6 for 65536, and success for negative
  numbers, are not confirmed against real VBA).
* Result types of mixed Single/Long/Currency/Date/Empty operands follow the Learn operator
  pages, not the type tables of 5.6.9.3.
* Bare `Print #n` and `?` forms: kept (revision 3.0's grammar requires the `Debug.`/`Me.` prefix,
  which looks like an editorial slip because the statement keyword list still contains `Print`).

Stricter than the text: `Object = Variant` without `Set`, `Let` on read-only properties and
object variables without a default member are analysis errors rather than run-time errors.

## Gaps (known deviations, not fixed)

* The lexer recognises U+2028/2029 line terminators and Unicode space characters only in
  sources that are valid UTF-8; code-page text is byte-based (3.2.1, 3.2.2).
* Circular dependencies between modules through `Const`, `Enum`, `Type`, `Implements`, `Event`
  (5.2.3.6); forward references between Enum members; uniqueness of public type names across
  modules; reserved words as `Type` member names; `Dim a&(n)` at module level; `#ordinal`
  alias checks of `Declare`; `VB_GlobalNameSpace` classes.
* `Call` on an object whose default member is a property is not error 450;
  late-bound named-argument error numbers (448/449); trailing `Static` in a procedure header;
  event-handler declaration rules; private types as result types; Variant overflow promotion
  for Date and Single operands.
* `Lock`/`Unlock` ranges are not tracked; `Width` does not wrap a single long item;
  `Write` with a trailing semicolon or `Spc`; static checks of `Input`/`Put`/`Get` operands.
* `MidB` statement, `LSet`/`RSet` between UDTs, `Str` of a Date.
* Strings with an odd number of bytes (`ChrB`, `StrConv(..., vbFromUnicode)` of an odd result,
  `LeftB`/`MidB` of an odd length, a Byte array of odd length converted to a String) are not
  representable: a string is a sequence of 16-bit units, and `Len`, `Mid`, `&`, comparison,
  `Like` and the memory meter are all unit based. Holding half a unit needs a byte-granular
  string type in every one of them, which is not a local change; these cases stay
  `unsupported` rather than being approximated.
* `y = 1` followed by `Dim y` in one procedure is accepted (procedure scope is collected before
  binding, so the earlier use binds to the later declaration); `(v)` of a Variant holding an
  object without a default member is not error 438; a type character on `True`, `False`,
  `Empty` or `Null` is a lexical error; a project function named like a library function
  (`Left`) hides it for calls with arguments.
* Revision 3.0 String/Variant relational row (see above).
* Financial functions: the text gives no formulas (DDB's differs from the documented one), so
  only definitional cases are checked.

## Ambiguities and slips in the text

Noted while testing (not tested where they matter): the radix-literal table has typos in its
upper bounds and in the `^` rows; the date rules refer to formulas that are not in the
distributed text; `RGB` uses a maximum where a minimum is meant; `Fix` and `Oct` descriptions
are copied from `Int` and `Hex`; several parameter tables say "default 1" where -1 is meant;
`VbTriState` values differ between two places; §5.4.2.13 and §5.4.2.16 give two selector
limits; the arithmetic operand-type tables have blank cells and overlapping rows; error
operands are said to raise 13 in the preamble but to yield Error in the effective-type table.
