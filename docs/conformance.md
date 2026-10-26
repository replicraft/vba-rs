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
| implemented | 296 |
| partial | 8 |
| unsupported | 32 |
| not-started | 0 |
| Conformance programs | 657 |
| Programs that are known deviations (XFAIL) | 29 |

## Entries per category and status

| Category | Total | implemented | partial | unsupported | not-started |
| --- | ---: | ---: | ---: | ---: | ---: |
| statement | 76 | 66 | 1 | 9 | 0 |
| function-conversion | 8 | 8 | 0 | 0 | 0 |
| function-math | 13 | 13 | 0 | 0 | 0 |
| function-type-conversion | 13 | 13 | 0 | 0 | 0 |
| derived-math | 21 | 21 | 0 | 0 | 0 |
| function-string | 20 | 18 | 2 | 0 | 0 |
| function-datetime | 20 | 20 | 0 | 0 | 0 |
| function-financial | 13 | 13 | 0 | 0 | 0 |
| function-information | 11 | 10 | 0 | 1 | 0 |
| function-array | 3 | 3 | 0 | 0 | 0 |
| function-logic | 4 | 3 | 0 | 1 | 0 |
| function-formatting | 4 | 4 | 0 | 0 | 0 |
| function-file | 11 | 10 | 0 | 1 | 0 |
| function-interaction | 19 | 11 | 0 | 8 | 0 |
| operator-arithmetic | 8 | 8 | 0 | 0 | 0 |
| operator-concatenation | 1 | 1 | 0 | 0 | 0 |
| operator-comparison | 8 | 8 | 0 | 0 | 0 |
| operator-logical | 6 | 6 | 0 | 0 | 0 |
| operator-precedence | 1 | 1 | 0 | 0 | 0 |
| data-type | 18 | 14 | 3 | 1 | 0 |
| identifier-type-character | 7 | 6 | 1 | 0 | 0 |
| object | 13 | 3 | 0 | 10 | 0 |
| language-element | 38 | 36 | 1 | 1 | 0 |

## Entries per provenance

| Provenance | Entries |
| --- | ---: |
| spec | 0 |
| doc | 160 |
| derived | 66 |
| unverified | 110 |
| (no program) | 0 |

## Programs per area and provenance

| Area | Programs | spec | doc | derived | unverified |
| --- | ---: | ---: | ---: | ---: | ---: |
| arrays | 31 | 0 | 12 | 3 | 16 |
| classes | 27 | 0 | 11 | 10 | 6 |
| collections | 17 | 0 | 17 | 0 | 0 |
| conversions | 39 | 0 | 18 | 16 | 5 |
| datatypes | 52 | 0 | 38 | 0 | 14 |
| datetime | 44 | 0 | 23 | 0 | 21 |
| derived-math | 21 | 0 | 21 | 0 | 0 |
| errors | 26 | 0 | 17 | 0 | 9 |
| events | 7 | 0 | 5 | 2 | 0 |
| file-io | 29 | 0 | 27 | 1 | 1 |
| financial | 12 | 0 | 2 | 10 | 0 |
| host | 33 | 0 | 20 | 10 | 3 |
| language | 16 | 0 | 3 | 0 | 13 |
| logic | 10 | 0 | 8 | 2 | 0 |
| math | 37 | 0 | 8 | 16 | 13 |
| operators | 65 | 0 | 22 | 41 | 2 |
| procedures | 34 | 0 | 15 | 8 | 11 |
| statements | 88 | 0 | 78 | 8 | 2 |
| strings | 69 | 0 | 36 | 17 | 16 |

## Gaps

Entries that are not `implemented`, with the reason recorded in the inventory.

| Id | Category | Name | Status | Notes |
| --- | --- | --- | --- | --- |
| `stmt.appactivate` | statement | AppActivate | unsupported | AppActivate is not defined ('Sub or Function not defined') |
| `stmt.date` | statement | Date | unsupported | setting the system clock is not provided by the stdlib host |
| `stmt.deletesetting` | statement | DeleteSetting | unsupported | the registry-backed settings statements have no host service |
| `stmt.let` | statement | Let | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `stmt.load` | statement | Load | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `stmt.reset` | statement | Reset | unsupported | the Reset statement is not bound by the analysis |
| `stmt.savesetting` | statement | SaveSetting | unsupported | the registry-backed settings statements have no host service |
| `stmt.sendkeys` | statement | SendKeys | unsupported | SendKeys is not defined ('Sub or Function not defined'); with UiAutomation as the gating capability it should be error 70 |
| `stmt.time` | statement | Time | unsupported | setting the system clock is not provided by the stdlib host |
| `stmt.unload` | statement | Unload | unsupported | UserForm objects need a host that provides forms; forms require a host that provides them |
| `func.len` | function-string | Len | partial | Len() of a user-defined type variable is rejected ('cannot convert Rec to Variant') |
| `func.strconv` | function-string | StrConv | partial | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
| `func.imestatus` | function-information | IMEStatus | unsupported | IME status needs a host IME |
| `func.partition` | function-logic | Partition | unsupported | Partition is not defined ('Sub or Function not defined') |
| `func.fileattr` | function-file | FileAttr | unsupported | FileAttr is not defined ('Sub or Function not defined') |
| `func.callbyname` | function-interaction | CallByName | unsupported | CallByName is not defined ('Sub or Function not defined') |
| `func.getallsettings` | function-interaction | GetAllSettings | unsupported | the registry-backed settings statements have no host service; registry-backed settings have no host service |
| `func.getsetting` | function-interaction | GetSetting | unsupported | the registry-backed settings statements have no host service |
| `func.input` | function-interaction | Input | unsupported | the Input function is not defined ('Sub or Function not defined') |
| `func.macid` | function-interaction | MacID | unsupported | Macintosh-only functions |
| `func.macscript` | function-interaction | MacScript | unsupported | Macintosh-only functions |
| `func.qbcolor` | function-interaction | QBColor | unsupported | QBColor is not defined ('Sub or Function not defined') |
| `func.rgb` | function-interaction | RGB | unsupported | RGB is not defined ('Sub or Function not defined') |
| `type.currency` | data-type | Currency | partial | the largest Currency value written as a literal is rejected as 'Currency literal overflows' (SEM010) |
| `type.string` | data-type | String | partial | assigning a String to a Byte() array (b = s) is rejected: 'cannot convert String to Byte()' |
| `type.udt` | data-type | User-defined type (Type) | partial | Len() of a user-defined type variable is rejected ('cannot convert Rec to Variant') |
| `type.dictionary` | data-type | Dictionary | unsupported | Scripting.Dictionary is a COM library; the default host registers no classes (429) |
| `typechar.at` | identifier-type-character | @ Currency type character | partial | the largest Currency value written as a literal is rejected as 'Currency literal overflows' (SEM010) |
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
| `expr.default-member` | language-element | Default member resolution | partial | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |

## Known deviations

Programs whose documented expectation the implementation does not meet (kept failing on purpose).

| Program | Kind | Reason |
| --- | --- | --- |
| `classes/default_member_let` | unsupported | 'w = 5' on an object whose default member has a Property Let is rejected with SEM007 (requires Set) |
| `collections/dictionary_not_provided` | unsupported | Scripting.Dictionary is a COM library; the default host registers no classes (429) |
| `collections/filesystemobject_not_provided` | unsupported | Scripting.FileSystemObject is a COM library; the default host registers no classes (429) |
| `collections/userform_not_provided` | unsupported | UserForm objects need a host that provides forms |
| `datatypes/currency_max_literal` | wrong | the largest Currency value written as a literal is rejected as 'Currency literal overflows' (SEM010) |
| `datetime/date_statement` | unsupported | setting the system clock is not provided by the stdlib host |
| `datetime/time_statement` | unsupported | setting the system clock is not provided by the stdlib host |
| `file-io/fileattr_mode` | unsupported | FileAttr is not defined ('Sub or Function not defined') |
| `file-io/input_function` | unsupported | the Input function is not defined ('Sub or Function not defined') |
| `file-io/input_function_past_end` | unsupported | the Input function is not defined ('Sub or Function not defined') |
| `file-io/len_of_user_defined_type` | unsupported | Len() of a user-defined type variable is rejected ('cannot convert Rec to Variant') |
| `file-io/reset_statement` | unsupported | the Reset statement is not bound by the analysis |
| `host/appactivate_denied` | unsupported | AppActivate is not defined ('Sub or Function not defined') |
| `host/deletesetting` | unsupported | the registry-backed settings statements have no host service |
| `host/getallsettings` | unsupported | registry-backed settings have no host service |
| `host/imestatus` | unsupported | IME status needs a host IME |
| `host/load_unload_form` | unsupported | forms require a host that provides them |
| `host/macid_macscript` | unsupported | Macintosh-only functions |
| `host/qbcolor_function` | unsupported | QBColor is not defined ('Sub or Function not defined') |
| `host/rgb_function` | unsupported | RGB is not defined ('Sub or Function not defined') |
| `host/sendkeys_denied` | unsupported | SendKeys is not defined ('Sub or Function not defined'); with UiAutomation as the gating capability it should be error 70 |
| `host/unsupported_feature_is_not_success` | unsupported | the registry-backed settings statements have no host service |
| `language/addressof_without_native` | unsupported | AddressOf produces no callable native pointer without the Declare bridge |
| `language/callbyname_function` | unsupported | CallByName is not defined ('Sub or Function not defined') |
| `logic/partition_doc_ranges` | unsupported | Partition is not defined ('Sub or Function not defined') |
| `logic/partition_interval_one` | unsupported | Partition is not defined ('Sub or Function not defined') |
| `logic/partition_null` | unsupported | Partition is not defined ('Sub or Function not defined') |
| `strings/strconv_unicode_roundtrip` | unsupported | StrConv(..., vbFromUnicode) of an odd number of bytes: 'a string with an odd number of bytes cannot be represented as UTF-16' |
| `strings/string_to_byte_array` | unsupported | assigning a String to a Byte() array (b = s) is rejected: 'cannot convert String to Byte()' |
