# Foundation crates

`crates/foundation/` holds the small, dependency-light crates every other
component builds on: `vba-source`, `vba-diagnostics` and `vba-profile`. There is
deliberately no catch-all core crate.

## vba-source

Implemented:

- `SourceFile` keeps the **original bytes** of a module together with a
  declared `SourceEncoding` (`CodePage(u16)`, `Utf8`, `Unknown`). Bytes are
  never converted or normalised.
- `Span` is a half-open `u32` byte range (`merge`, `contains`, `slice`).
- `LineIndex` maps a byte offset to a zero-based line and byte column. A line
  ends with CR, LF or CRLF (MS-VBAL section 3.2.1).
- `SourceMap` assigns `FileId`s.
- `SourceFile::line_text_lossy` gives a display-only rendering (UTF-8 for
  `Utf8`, Latin-1 otherwise).

Not implemented:

- Decoding from code pages to Unicode. Columns are byte columns, not
  characters or UTF-16 units; an LSP front end must convert.
- U+2028 / U+2029 as line terminators (the grammar lists them, but they cannot
  occur in single-byte code pages; they are not recognised in UTF-8 files
  either).
- Files larger than 4 GiB (spans are 32-bit).

## vba-diagnostics

Implemented:

- `Diagnostic { severity, code, file, span, message, labels, notes }`, with
  `Severity` ordered Error, Warning, Info, Hint, and stable `Code("LEX001")`
  style codes.
- `Diagnostics` collection: `sort_and_dedup` orders by file, start, end,
  severity, code, message, so output is independent of production order.
- `render_plain` renders a diagnostic with its source line and `^` underlines,
  without colour and without third-party dependencies.

Not implemented: multi-file labels, multi-line underlines (only the first line
of a span is underlined), colour, JSON/SARIF output.

## vba-profile

The VBA-visible target model. Nothing in it reads the machine running vba-rs,
so an ARM64 Mac evaluates `#If Win64` for a Windows x86 profile exactly like an
x64 Windows machine.

Implemented:

- `LanguageGeneration`: `Access95` (experimental), `Vba5`, `Vba6`, `Vba7_0`,
  `Vba7_1`, with `Maturity`.
- `Feature` queries: `LongPtr`, `PtrSafe`, `LongLong` (VBA 7.0+),
  `Implements`, `Events`, `AddressOf` (VBA 5.0+).
  `Profile::supports(LongLong)` additionally requires a 64-bit target
  (MS-VBAL section 3.3.2 makes LongLong literals invalid without 64-bit
  arithmetic).
- `HostInfo` (application, version, release year), `Target` (Windows or macOS,
  16/32/64 bit).
- `CompilationConstant` (`VBA6`, `VBA7`, `Win16`, `Win32`, `Win64`, `Mac`) and
  `Profile::predefined_constant`. `VBA6`/`VBA7` are `None` (undefined) in
  generations that predate them. `Win32` is true on 32- and 64-bit Windows.
- A catalog of 11 built-in profiles (`profiles()`, `find_profile`,
  `default_profile`; default `vba7.1-excel-win64`).

Caveats and not implemented:

- The `Win*` constants on macOS are false and `Mac` is true per Microsoft's
  documentation; this has **not** been verified against a real Mac host.
- Which features Access 95 has is a conservative guess; the profile is marked
  experimental.
- Host application and version are metadata only; they do not yet drive any
  object-model availability.
- User-defined `#Const` evaluation is not here (it belongs with the
  conditional-compilation processor).
