# vba-stdlib

`crates/runtime/vba-stdlib` implements the VBA standard library (the intrinsic
functions of `VBA.Strings`, `VBA.Math`, `VBA.DateTime`, `VBA.Information`,
`VBA.Conversion`, `VBA.Financial` and the pure part of `VBA.Interaction`) and
the `Like` operator. It depends only on `vba-value`: no VM, parser or host.

Provenance tags: **spec** = MS-VBAL §6 text (which mostly lists signatures and
result types), **doc** = documented Microsoft VBA behavior, **ref** = behavior
taken from a published reference algorithm (named where used), **unverified** =
a rule chosen where no source was available; it is covered by a test of our own
expectation but has not been compared with Office. Nothing here has been run
against a real VBA host.

## Design

- `registry`: case-insensitive lookup by `Left`, `Left$`, `Strings.Left`,
  `VBA.Left`, `VBA.Strings.Left`; `FunctionInfo` carries parameter names,
  types, optional/default text, `ByRef`/`ByVal` (all `ByVal`), `ParamArray`,
  return type, whether the `$` form exists, and a `Kind`. `invoke` checks the
  argument count (error 450); an omitted required argument is error 449.
  `Kind::Eager` functions are called with all arguments evaluated, which is
  also how VBA treats `IIf`, `Choose` and `Switch` (no short-circuit).
  `Kind::Vm` (`IsMissing`) and `Kind::PrintHelper` (`Tab`, `Spc`) must be
  handled by the VM.
- `host`: `Clock` (`Now`, `Timer`), `Environment` (`Environ`), `Locale`
  (explicit en-US default; every `Format`/`MonthName`/`Weekday` rule reads it)
  combined in `Host`. `FixedHost` is the fully deterministic implementation.
  Random state lives in the caller-owned `State`; `Ctx` also carries
  `Option Compare`, `Option Base` and the `LongPtr` width.
- Errors: `StdError::Vba(VbaError)` for trappable errors, `Unsupported(&str)`
  for documented behavior not implemented (never a VBA error), and
  `VmRequired(&str)` for functions that need VM state.
- `Null`: functions taking a `Variant` string (`Left`, `Mid`, `Trim`, `LCase`,
  ...) return `Null` for `Null`; the `$` form raises 94. Functions with a
  `String` parameter (`Replace`, `Split`, `Space`, `Asc`, `Chr`, ...) raise 94.
  (**doc**)
- `vba-profile` is not used: nothing here differs by VBA version except
  `CLngLng`/`CLngPtr` (VBA7), which the VM's profile layer must hide for VBA6.

## Implemented

| Library | Functions | Notes |
| --- | --- | --- |
| Strings | `Asc AscB AscW Chr ChrW Left Right Mid Len LenB LeftB RightB MidB InStr InStrB InStrRev LCase UCase LTrim RTrim Trim Replace Space String StrReverse StrComp StrConv Split Join Filter Format FormatNumber FormatCurrency FormatPercent FormatDateTime` and all `$` forms | `Like` is `like::like` (it is an operator). |
| Math | `Abs Atn Cos Exp Log Rnd Randomize Round Sgn Sin Sqr Tan` | `Fix`/`Int` are listed under Conversion as in the type library. |
| DateTime | `Date Time Now Timer DateAdd DateDiff DatePart DateSerial DateValue TimeSerial TimeValue Day Month Year Hour Minute Second Weekday WeekdayName MonthName` | `Date$`/`Time$` included. |
| Information | `IsArray IsDate IsEmpty IsError IsNull IsNumeric IsObject TypeName VarType LBound UBound` | `IsMissing` is VM-only. |
| Conversion | `CBool CByte CCur CDate CDbl CDec CInt CLng CLngLng CLngPtr CSng CStr CVar CVDate CVErr Error Fix Int Hex Oct Str Val` | `C*` delegate to `vba_value::convert`. |
| Financial | `DDB FV IPmt IRR MIRR NPer NPV Pmt PPmt PV Rate SLN SYD` | |
| Interaction | `Array Choose IIf Switch Environ` and the `PrintHelper` values for `Tab`/`Spc` | `Command`, UI and `CreateObject`-style functions are out of scope. |

## Not implemented (explicit errors)

- Byte-oriented functions work on the UTF-16LE bytes of the string
  (`LenB` = 2 * code units, `AscB` the low byte of the first unit). A VBA
  BSTR may hold an odd number of bytes; `vba-value` strings are UTF-16 and
  cannot. **Policy**: any result with an odd byte count (`ChrB`, `LeftB`,
  `RightB`, `MidB`, and `StrConv(vbFromUnicode)` into an odd number of bytes)
  is `Unsupported`, never silently rounded and never error 5 (it is valid VBA).
  `InStrB` with text comparison is `Unsupported`.
- `Asc`/`Chr` use the host locale's ANSI code page (`Locale::ansi_code_page`,
  1252 for en-US). `Asc` of a character the page cannot encode is
  `Unsupported` (real VBA substitutes an environment-dependent code); double
  byte pages (932, 936, 949, 950) give `lead*256+trail` from `Asc` and accept
  256..65535 in `Chr`.
- `StrConv`: `vbSimplifiedChinese`, `vbTraditionalChinese` and
  `vbLinguisticCasing` are `Unsupported`; `vbUnicode`/`vbFromUnicode` use
  `encoding_rs` code pages only (874, 932, 936, 949, 950, 1250-1258, 10000,
  20866, 21866, ISO-8859-x, 65001); any other page, or a `LocaleID` outside the
  small built-in table (`textconv::lcid_code_page`), is `Unsupported`.
- `Str` of a `Date`, `DDB` with a fractional period: `Unsupported`.
- `TypeName` of objects and user-defined types, `IsMissing`: `VmRequired`.
- Windows LCID collation: `Option Compare Text` is a simple per-code-unit case
  fold (inherited from `vba-value`).
- Empty `Split`/`Filter`/`Array()` results are allocated arrays without
  elements (`LBound` 0, `UBound` -1; `Option Base 1` gives 1 and 0), built with
  `VbaArray::new_empty`. A user `ReDim` to an upper bound below the lower bound
  remains error 9.
- `Format`: locale-dependent calendars (non-Gregorian) are not supported.

## Rules

### Strings
- Argument checks and results (**doc**): `Left/Right` negative length 5;
  `Mid` start < 1 or length < 0 5, start past the end gives `""`; `InStr` start
  < 1 5, start past the end 0, empty needle returns `start`, empty haystack 0;
  `InStrRev` start 0 or < -1 is 5, match must lie within the first `start`
  characters, empty needle returns `start`; `Replace` returns the text from
  `Start`, empty `Find` returns that text unchanged, count 0 replaces nothing,
  `Count` < -1 or `Start` < 1 is 5; `Space`/`String` negative count 5;
  `String(n, code)` uses `code Mod 256` above 255; `Split` empty expression
  gives no elements, empty delimiter one element, `Limit` < -1 is 5;
  `Trim` removes U+0020 only; `Filter` source must be a one-dimensional array,
  else 5. `Compare` 0 binary, 1 text, -1 option default, other (including 2,
  Access-only) 5; omitted uses `Option Compare` (except `InStrRev`, which
  defaults to binary as documented).
- `Split` with `Limit = 0` returns no elements (**unverified**).
- `StrReverse` reverses UTF-16 code units, splitting surrogate pairs
  (**unverified**). `StrConv(vbProperCase)` capitalizes after any character
  that is not a letter, digit or apostrophe (**unverified**).
- `StrConv` width/kana (`textconv`, **unverified** against Windows for rare
  characters): `vbWide` maps ASCII 0x20-0x7E, the symbols cent/pound/not/
  macron/broken bar/yen/won and halfwidth katakana (with voiced and semi-voiced
  marks combined) to fullwidth forms; `vbNarrow` is the inverse;
  `vbKatakana`/`vbHiragana` map U+3041-3096 and U+30A1-30F6 (offset 0x60) and
  leave halfwidth katakana and the prolonged sound mark alone. Width and kana
  are applied before case; `vbWide`+`vbNarrow`, `vbKatakana`+`vbHiragana`,
  and `vbUnicode`/`vbFromUnicode` with any other flag are error 5
  (**unverified**). `vbFromUnicode` encodes unmappable characters as `?` and
  packs the bytes two per unit, little endian.
- `Asc`/`Chr` use Windows-1252 for 0x80-0x9F (**doc**: default ANSI code page
  assumption, documented as a gap); `ChrW`/`AscW` use UTF-16 units, `AscW`
  returning a signed `Integer` (**spec**).
- `Like` (**spec** §5.6.9, **doc**): `? * # [list] [!list]`, ranges, `-`/`!`
  literal at the edges, `[]` matches the empty string. An unterminated `[` and
  a descending range are error 93 (descending range: **unverified**).

### Format family (**doc**)
- Source checked: Microsoft Learn, "Format function (Visual Basic for
  Applications)". That page defines named formats, user-defined numeric
  (`0 # . % , : / E- E+ e- e+ - + $ ( )`, backslash, quoted text), date/time
  and string formats and the section rules, and nothing else: **VBA `Format`
  has no fraction formats, no `*` fill, no `?` placeholder, no `_` spacing and
  no `[Red]`/`[Color n]`/`[condition]` sections**. These are therefore not
  implemented as features; the implementation merely skips `*x`, `_x` (as a
  space), `?` (as a digit placeholder) and `[...]` so that Excel-style formats
  do not produce garbage (**unverified** what Office does with them). The doc
  examples are asserted in `format::tests::learn_page_*`.
- Format is truncated to 257 characters. One to four numeric sections: pos/zero;
  pos, neg; pos, neg, zero; pos, neg, zero, Null. An empty section between
  semicolons uses the positive section (a negative value then gets a minus
  sign; that sign is **unverified**). String formats: two sections, the
  second for `Null` and `""`.
- `Format(Null, fmt)` returns the literal text of the fourth section of a
  four-section numeric format (or the second of a two-section string format)
  as the page says; any other format gives `Null` (the page's four-section row
  is the only source; **unverified** against Office).
- Numbers are formatted from their 15-significant-digit decimal value, ties
  away from zero (so `Format(0.125,"0.00")` is `0.13`). Sections
  `positive;negative;zero`; with one section a negative value gets `-` in
  front. `0 # ?` placeholders, `.`, `,` (grouping between digits, scaling by
  1000 after the last digit), `%`, `E+ E- e+ e-`, `"text"`, `\c`, `_c`.
  `Format(5,"0.##")` is `"5."` (the point stays).
- Named formats: `General Number`, `Currency`, `Fixed`, `Standard`,
  `Percent`, `Scientific`, `Yes/No`, `True/False`, `On/Off`; dates `General
  Date`, `Long/Medium/Short Date`, `Long/Medium/Short Time`. Their patterns
  come from `Locale`.
- Date tokens: `c d dd ddd dddd ddddd dddddd w ww m mm mmm mmmm q y yy yyyy h
  hh n nn s ss ttttt AM/PM am/pm A/P a/p AMPM / :`. `m` is minutes right after
  `h` or right before `s`.
- Strings: `@ & < > !` with right-to-left fill by default; second section for
  the empty string.
- `FormatNumber/Currency/Percent` use `-1` / `-2` (`vbUseDefault`) as defaults;
  `Null` is error 94. `FormatDateTime` `Short Time` is 24-hour `hh:mm`.
- `Format(Null)` is `Null` unless a Null section exists; `Format$(Null)` is 94.

### Math
- `Rnd`/`Randomize` (**ref**: Microsoft KB 231847 for the generator; the
  Visual Basic runtime for seeding): 24-bit LCG `seed = (seed * 1140671485 +
  12820163) mod 2^24`, `Rnd = seed / 2^24` as `Single`, initial seed `0x50000`.
  The first five values with the default seed, `0.7055475 0.533424 0.5795186
  0.2895625 0.301948`, match the sequence published in the KB and are asserted.
  `Rnd(0)` repeats the last value. `Rnd(x<0)` sets `seed = (b + (b >> 24)) &
  0xFFFFFF` with `b` the 32 bits of `x` as a `Single`, then advances
  (**unverified** for the exact reseed formula). `Randomize n` replaces the
  middle 16 bits of the seed with `((hi & 0xFFFF) ^ (hi >> 16)) << 8`, `hi`
  being the high 32 bits of `n` as a `Double` (**unverified**);
  `Randomize` alone uses the host `Timer`.
- `Round` uses ties-to-even on the 15-digit decimal value (**doc**: banker's
  rounding; **unverified**: behavior for values like `1.005`); negative digits
  is 5. `Abs(Integer -32768)` is 6. `Sqr(<0)`, `Log(<=0)` are 5; `Exp` overflow 6.
  `Int`/`Fix` keep the subtype; strings become `Double`.

### DateTime (**doc**)
- `DateAdd` clamps the day to the end of the target month; `number` is rounded
  to a whole number; results outside year 100-9999 are 5.
- `DateDiff` counts boundaries crossed for `yyyy q m d y ww h n s`; `w` is whole
  7-day periods; `ww` honors `FirstDayOfWeek`.
- `DatePart("ww")` implements `vbFirstJan1`, `vbFirstFourDays`,
  `vbFirstFullWeek`; with `vbFirstJan1` days after the last full week stay in
  the same year (week 53), with the other two rules they can belong to week 1 of
  the next year (**unverified** against Office for `vbFirstJan1`).
- `DateSerial` two-digit years: 0-29 are 2000-2029, 30-99 are 1930-1999;
  negative years 5. `TimeSerial` wraps into one day.
- `IsDate("12")` is False (numeric strings are not dates).

### Financial (**doc**, formulas shared with the Excel functions)
- Sign convention and `Due` (0 end, non-zero beginning). `IRR` and `Rate` use
  Newton iteration with step damping, at most 20 tries, tolerance 1e-7
  (0.00001 percent); failure is 5. `IPmt` for beginning-of-period payments is
  0 in period 1 and uses the balance after the first payment afterwards (same
  convention remembered from open-source reimplementations; **unverified**). Tests use the
  worked examples of the Excel documentation (`Pmt`, `FV`, `PV`, `NPer`,
  `Rate`, `IPmt`, `PPmt`, `NPV`, `IRR`, `MIRR`, `SLN`, `SYD`, `DDB`) with
  tolerances, and identities (`IPmt + PPmt = Pmt`, `NPer(Pmt) = n`).

### Conversion
- `Hex/Oct`: `Byte` unsigned, `Integer`/`Boolean` 16-bit two's complement,
  `Long` 32-bit, `LongLong` 64-bit, everything else rounded to `Long` (outside
  the range: 6) (**doc**).
- `Val`: stops at the first non-numeric character, ignores blanks inside the
  number, `&H`/`&O` forms with 16-bit sign extension (`Val("&HFFFF")` = -1)
  unless a trailing `&` is present; beyond `Double` range 6 (**doc**).
- `Str`: leading space for non-negative numbers, `.` always the decimal
  separator.

## Tests

`cargo test -p vba-stdlib`: unit tests next to the code, table-driven function
tests through the registry (`tests/functions.rs`) and property tests
(`tests/properties.rs`: Split/Join round trip, InStr/InStrRev against naive
search, Like against an independent matcher, Format against integer and
grouping text, DateAdd/DateDiff round trips, Hex/Val round trip, Rnd
determinism, financial identities).
