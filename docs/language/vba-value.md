# vba-value

`crates/runtime/vba-value` is the VBA value system: strings, scalar types,
`Variant`, arrays, coercions and operators. It depends on no other vba-rs
crate, no async runtime, no COM and no host. Objects and user-defined types
are opaque handles (`ObjectHandle`, `UdtHandle`, `ObjectRef::Nothing`) that the
VM defines.

Provenance tags used below: **spec** = MS-VBAL text, **doc** = documented or
well-established VBA behavior not stated (or not fully stated) in MS-VBAL.
Nothing here has been compared against a real VBA host; doc-tagged rules are
covered by explicit tests but remain unverified against Office.

## Implemented

| Area | Module | Notes |
| --- | --- | --- |
| Errors | re-exported from `vba-core` | `VbaError` (number + standard description), constants for 5, 6, 7, 9, 10, 11, 13, 94. |
| Strings | `string`, `collation` | `VbaString` is a `Vec<u16>`; unpaired surrogates survive every operation; `Len` counts code units; strict/lossy conversion to Rust text; Binary and Text comparison (Text through the pluggable `Collation`, default `EnUs`). |
| Currency | `currency` | Scaled `i64` (/10000), exact add/sub, mul/div rounded half to even, overflow error 6, division by zero error 11. |
| Decimal | `decimal` | Own implementation: 96-bit mantissa, sign, scale 0..=28, 256-bit intermediates. See rules below. |
| Date | `date` | `f64` OLE date, range 0100-01-01..9999-12-31, epoch 1899-12-30, negative-date quirk, `DateSerial`/`TimeSerial` roll-over, weekday, en-US `CStr`. |
| Variant | `variant` | All subtypes, `VarType` codes, `VT_ARRAY`/`VT_BYREF` flag constants, `LongPtr` via caller-chosen `PointerWidth`. |
| Arrays | `array` | N-dimensional (max 60), per-dimension bounds, fixed/dynamic, `ReDim`, `ReDim Preserve`, `Erase`, column-major storage, element coercion on write. |
| Conversions | `convert` | `CBool CByte CCur CDate CDbl CDec CInt CLng CLngLng CSng CStr CVar` (`convert_to`). |
| Text | `text` | Double/Single to text (15/7 significant digits), numeric string parsing, `CDate` string parsing. |
| Operators | `ops` | `+ - * / \ Mod ^`, unary `-`, `&`, `= <> < > <= >=`, `And Or Xor Eqv Imp Not`. `Is` and `Like` are not here. |

## Rules

### Exchange with Rust (`exchange`)

`From` for `bool`, `u8`, `i8`, `i16`, `u16`, `i32`, `u32`, `i64`, `f32`, `f64`, `&str`,
`String`, `Option<T>` (`None` is `Null`), `Vec<T>` (a zero-based `Variant` array),
`VbaError`, `ObjectRef`; `TryFrom<&Variant>`/`TryFrom<Variant>` for the same scalar types,
`Currency`, `Decimal`, `Date`, `String`, `Vec<Variant>` and `ObjectRef`, following the
`convert` functions below (`"12"` is 12, `Null` is error 94, overflow error 6).
`Variant::empty/null/nothing`, `date_from_serial`, `date_from_ymd_hms`,
`array_with_lower_bound/array_zero_based/array_one_based`, `array_bounds`, `to_vec`;
`Date::from_unix_seconds`/`unix_seconds` (no time zones). `Display` is a debugging
form (strings quoted, `Empty`/`Null` named, arrays with bounds). One-dimensional
arrays only.

### Conversions (`convert`)
- Float, Currency and Decimal to integer subtypes round half to even, then
  range-check; NaN, infinities and out-of-range values are error 6 (**doc**,
  consistent with spec §5.6.9 conversions).
- `Null` is error 94 for every conversion except `CVar`; a `vbError` value
  re-raises its number; objects, UDTs and arrays are error 13 (the VM must
  resolve default members first).
- `Empty` is 0, `""`, `False`, or the date 0.
- Boolean is -1/0 numerically and `"True"`/`"False"` as text. `CBool` of a
  String accepts `True`/`False` case-insensitively, else a number.
- String to number: surrounding blanks, sign, thousands commas, leading `$`,
  `.`, exponent `E`/`D`, `&H`/`&O` literals (16-bit literal without `&` suffix
  is sign-extended: `&HFFFF` = -1). Non-numeric text, including `""`, is error
  13; out-of-double-range is error 6 (**doc**).
- `CDec(Double)` keeps 15 significant digits, `CDec(Single)` 7 (**doc**).
- `CStr(Double)`: up to 15 significant digits, scientific notation
  (`1E+15`, `1.5E-05`) when the exponent is below -4 or at least 15; Single
  uses 7 digits (**doc**). `CStr(Error)` is `Error n`.
- `CDate` of strings: numeric strings, `m/d/y`, `y-m-d`, month-name forms,
  `h:mm[:ss]`, AM/PM; two-digit years 00-29 mean 2000s, 30-99 the 1900s
  (**doc**, en-US only).

### Decimal (`decimal`; **doc**, modeled on `VarDecAdd/Mul/Div`)
- `+`/`-`: result scale is the larger scale. `*`: scales add. Excess digits
  (more than 96 bits or scale above 28) are removed with half-even rounding;
  needing a negative scale is error 6.
- `/`: as many fractional digits as fit (scale at most 28); an exact quotient
  has trailing zeros stripped. `1/3` is `0.3333333333333333333333333333`.
- Comparison and equality are by value (`1.0 = 1.00`).
- `CDec(Currency)` keeps scale 4 (`1.5000`).

### Date (`date`; **doc** unless noted)
- Valid serials: `-657434 <= x < 2958466`; outside is error 6 on conversion
  and arithmetic. `Date::from_ymd_hms` rejects invalid fields with error 5.
- Negative serials: time of day is `abs(frac)`, so `-1.25` is
  1899-12-29 06:00.
- Seconds are rounded to the nearest second when splitting a serial.
- The proleptic Gregorian calendar is used; 1900 is not a leap year.

### Operators (`ops`)
- Typed operands use `Overflow::Error` (Integer + Integer overflow is error
  6); Variants use `Overflow::Promote` (Byte to Integer to Long to Double;
  LongLong overflow is error 6) (**doc**).
- `+ - *` result type: Decimal > Currency > Double/Date > Single > LongLong >
  Long > Integer > Byte; Single with LongLong gives Double. The Currency vs
  Double ordering is the least certain rule.
- `+` concatenates two Strings (or String with Empty); String with a number
  adds numerically (String becomes Double).
- `/`: Decimal if either is Decimal, Currency if either is Currency, Single if
  both are Single, else Double. Division by zero is 11 including `0/0`.
- `\` and `Mod`: operands rounded half to even; result type LongLong, Long,
  Integer or Byte as described in the `ops` module docs; sign of `Mod` follows
  the dividend; `Integer.MinValue \ -1` is error 6 (typed).
- `^` is always Double; `0 ^ negative` is error 11; a NaN result is error 5.
- Comparison: Null gives Null; two Variants of number and String order the
  number first; with `StringNumberRule::Coerce` the string is converted
  (error 13 if not numeric); Empty compares as 0 or `""`; Date versus a
  parsable date string compares as dates; NaN is unordered (`<>` true, others
  false). Result is Boolean or Null.
- Logic: two Booleans give Boolean, otherwise bitwise on the wider integer
  type; Null follows the VBA truth tables (`Null And False` is False,
  `Null Or True` is True, otherwise Null).
- `&`: operands converted as `CStr`; `Null & Null` is Null, a single Null is
  `""`.

## Known gaps

- **When a Double or Single prints in scientific notation is unverified.** `format_double`
  (`CStr`, `&`, `Debug.Print`, `Str`) switches to `E` notation for a decimal exponent below
  -4 (so `0.0001` prints `0.0001` and `0.00001` prints `1E-05`), and for 15 or more integer
  digits (`1E+15`); `format_single` does the same with 7 significant digits. A common belief is
  that real VBA already prints `0.0001` as `1E-04` (a threshold of -3). Searched: the Learn pages of `Str` and the type-conversion functions (the `CStr` entry) say only that the number becomes a string
  and give no switch-over point (their examples are `437.324`, `459.001`); the 15/7 digit counts are not stated there either. With no
  document to back a change, the threshold is left as it was and the conformance programs that
  pin it (`datatypes/double_default_display`, `conversions/double_to_string`) stay tagged
  `unverified`. Changing it needs a documented example or a run on real VBA.

- **Strings with an odd number of bytes are not representable.** `VbaString`
  is a sequence of 16-bit code units; a native `BSTR` can carry a trailing
  odd byte (`StrConv(s, vbFromUnicode)` of an odd number of characters,
  `ChrB`, `LeftB`). Decision: the representation is not widened. A pending
  half unit would have to be preserved or rejected by every unit-based
  operation (concatenation shifts it, comparison, `Mid`, file output), and a
  silent drop is wrong, so each producer of an odd byte count returns the
  explicit `Unsupported` error instead (see `docs/language/vba-stdlib.md`), and the
  conformance program `strings/strconv_unicode_roundtrip` stays an XFAIL with
  that reason. Even byte counts are exact.
- `b = s` / `s = b` between a String and a dynamic `Byte` array use the
  UTF-16LE layout (two bytes per code unit, low byte first; the array is
  indexed from 0). `StrConv(b, ...)` with a Byte array reads it as the string
  holding those bytes. A Byte array of odd length converted to a String is
  `Unsupported` for the reason above. **unverified** against real VBA (derived
  from the documented "Unicode string stored in a byte array" behaviour).

- Text comparison (`collation`) approximates Windows' `NORM_IGNORECASE |
  NORM_IGNOREKANATYPE | NORM_IGNOREWIDTH` for en-US: BMP simple case folding,
  fullwidth to ASCII, halfwidth to fullwidth katakana (no voiced-mark
  combining), katakana to hiragana. Accents are significant but accented
  letters do not sort next to their base letters (order is by folded code
  unit); ligatures, linguistic ordering and non-BMP folding are not
  reproduced. This is NOT LCID-exact. The `Collation` trait lets a host
  supply its own (`VbaString::compare_with`; the stdlib `*_with` functions and
  `Host::collation`).
- Only en-US text formats (decimal point, `m/d/yyyy`, 12-hour clock) exist.
  Dates without a year are rejected (they depend on the current date).
  Parenthesized negatives, trailing type characters and currency symbols other
  than `$` are not accepted in numeric strings.
- Operators do not resolve default members of objects, do not handle
  `Variant` arrays, and do not implement `Is` or `Like`.
- Rules tagged **doc** (especially Currency/Double promotion, Decimal division
  digits, Currency multiplication tie rounding, Variant LongLong overflow, the
  `x - Date` result type and `CStr` of dates) are not verified against real
  VBA.
- Arrays store every element as a `Variant`; there is no packed storage.
  User-defined-type elements start as `Empty`. No array locking.
- `Decimal` remainder is exact but limited to divisors below 2^97 after scale
  alignment.

## Tests

`cargo test -p vba-value`: unit tests, property tests (`tests/properties.rs`)
and differential oracle tests (`tests/oracle.rs`). The oracles are
dev-dependencies only and are never used by shipped code:

- `jiff`: every day from 0100-01-01 to 9999-12-31 (3,615,900 days) is checked
  for calendar fields to serial and back, weekday and leap year/month length;
  `DateSerial` day and month roll-over; time of day for non-negative serials.
  Negative-serial time of day (the OLE `abs(frac)` quirk) has no oracle.
- `rust_decimal`: `+ - *` agree on value and scale where the exact result fits
  (including scale rules: larger scale for `+`/`-`, summed scales for `*`);
  over the full range only results with no dropped digits are compared;
  ordering agrees. Not asserted because VBA differs: overflow (`rust_decimal`
  returns `None`, VBA error 6), rounding of excess digits, and division.
