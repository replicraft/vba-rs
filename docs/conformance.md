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

* `spec`: stated by MS-VBAL (a section is cited)
* `doc`: stated by the cited Microsoft language-reference page (example or remark)
* `derived`: derived by reasoning from documented semantics, not stated verbatim
* `unverified`: only the current behavior; no document backs it

## Totals

| Measure | Count |
| --- | ---: |
| Inventory entries | 336 |
| implemented | 321 |
| partial | 1 |
| unsupported | 14 |
| not-started | 0 |
| Conformance programs | 664 |
| Programs that are known deviations (XFAIL) | 6 |

## Entries per category and status

| Category | Total | implemented | partial | unsupported | not-started |
| --- | ---: | ---: | ---: | ---: | ---: |
| statement | 76 | 74 | 0 | 2 | 0 |
| function-conversion | 8 | 8 | 0 | 0 | 0 |
| function-math | 13 | 13 | 0 | 0 | 0 |
| function-type-conversion | 13 | 13 | 0 | 0 | 0 |
| derived-math | 21 | 21 | 0 | 0 | 0 |
| function-string | 20 | 19 | 1 | 0 | 0 |
| function-datetime | 20 | 20 | 0 | 0 | 0 |
| function-financial | 13 | 13 | 0 | 0 | 0 |
| function-information | 11 | 11 | 0 | 0 | 0 |
| function-array | 3 | 3 | 0 | 0 | 0 |
| function-logic | 4 | 4 | 0 | 0 | 0 |
| function-formatting | 4 | 4 | 0 | 0 | 0 |
| function-file | 11 | 11 | 0 | 0 | 0 |
| function-interaction | 19 | 19 | 0 | 0 | 0 |
| operator-arithmetic | 8 | 8 | 0 | 0 | 0 |
| operator-concatenation | 1 | 1 | 0 | 0 | 0 |
| operator-comparison | 8 | 8 | 0 | 0 | 0 |
| operator-logical | 6 | 6 | 0 | 0 | 0 |
| operator-precedence | 1 | 1 | 0 | 0 | 0 |
| data-type | 18 | 17 | 0 | 1 | 0 |
| identifier-type-character | 7 | 7 | 0 | 0 | 0 |
| object | 13 | 3 | 0 | 10 | 0 |
| language-element | 38 | 37 | 0 | 1 | 0 |

## Entries per provenance

| Provenance | Entries |
| --- | ---: |
| spec | 0 |
| doc | 148 |
| derived | 71 |
| unverified | 117 |
| (no program) | 0 |

## Programs per area and provenance

| Area | Programs | spec | doc | derived | unverified |
| --- | ---: | ---: | ---: | ---: | ---: |
| arrays | 31 | 0 | 12 | 3 | 16 |
| classes | 28 | 0 | 11 | 11 | 6 |
| collections | 17 | 0 | 17 | 0 | 0 |
| conversions | 39 | 0 | 18 | 16 | 5 |
| datatypes | 52 | 0 | 38 | 0 | 14 |
| datetime | 44 | 0 | 21 | 2 | 21 |
| derived-math | 21 | 0 | 21 | 0 | 0 |
| errors | 26 | 0 | 17 | 0 | 9 |
| events | 7 | 0 | 5 | 2 | 0 |
| file-io | 31 | 0 | 27 | 2 | 2 |
| financial | 12 | 0 | 2 | 10 | 0 |
| host | 34 | 0 | 17 | 10 | 7 |
| language | 17 | 0 | 3 | 0 | 14 |
| logic | 10 | 0 | 8 | 2 | 0 |
| math | 37 | 0 | 8 | 16 | 13 |
| operators | 65 | 0 | 22 | 41 | 2 |
| procedures | 34 | 0 | 15 | 8 | 11 |
| statements | 88 | 0 | 78 | 8 | 2 |
| strings | 71 | 0 | 36 | 18 | 17 |

## Gaps

Entries that are not `implemented`, with the reason recorded in the inventory.

| Id | Category | Name | Status | Notes |
| --- | --- | --- | --- | --- |
| `stmt.load` | statement | Load | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `stmt.unload` | statement | Unload | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `func.strconv` | function-string | StrConv | partial | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
| `type.dictionary` | data-type | Dictionary | unsupported | Scripting.Dictionary is a COM library; the default host registers no classes (429) |
| `obj.dictionary` | object | Dictionary | unsupported | Scripting.Dictionary is a COM library; the default host registers no classes (429) |
| `obj.drive` | object | Drive | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.drives` | object | Drives | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.file` | object | File | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.files` | object | Files | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.filesystemobject` | object | FileSystemObject | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.folder` | object | Folder | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.folders` | object | Folders | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.textstream` | object | TextStream | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `obj.userform` | object | UserForm | unsupported | UserForm objects need a host that provides forms |
| `expr.addressof` | language-element | AddressOf expression | unsupported | AddressOf produces no callable native pointer without the Declare bridge |

## Known deviations

Programs whose documented expectation the implementation does not meet (kept failing on purpose).

| Program | Kind | Reason |
| --- | --- | --- |
| `collections/dictionary_not_provided` | unsupported | Scripting.Dictionary is a COM library; the default host registers no classes (429) |
| `collections/filesystemobject_not_provided` | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `collections/userform_not_provided` | unsupported | UserForm objects need a host that provides forms |
| `host/load_unload_form` | unsupported | forms require a host that provides them |
| `language/addressof_without_native` | unsupported | AddressOf produces no callable native pointer without the Declare bridge |
| `strings/strconv_unicode_roundtrip` | unsupported | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
