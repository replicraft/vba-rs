# Type libraries (`vba-typelib`)

`vba-typelib` reads Windows type libraries, the metadata VBA binds
early-bound calls to: libraries, coclasses, interfaces, dispinterfaces,
enums, records, aliases and modules, with their members (functions and
property accessors with member ids, invoke kinds, parameter names, types,
flags and defaults), constants, GUIDs, help strings, version, LCID and flags.
It works on bytes only: no COM, no code is loaded, no `unsafe`.

Inputs: a raw MSFT-format file (`.tlb`, `.olb`) or a PE image (`.dll`,
`.ocx`, `.exe`, 32- or 64-bit) whose `TYPELIB` resources are read. The older
SLTG format is recognised and rejected with `TypelibError::UnsupportedFormat`
(see "Open unknowns").

The crate lives in `crates/interoperability/vba-typelib`. It is independent
of the `vbars` CLI; `cargo run -p vba-typelib --example dump -- FILE
[--members] [--type NAME]` prints the libraries of a file.

## Interface

```rust
let lib   = vba_typelib::load(&bytes)?;          // raw typelib or first TYPELIB resource of a PE
let libs  = vba_typelib::load_all(&bytes)?;      // every library, with its resource id
let lib   = vba_typelib::load_file(path)?;
let lib   = vba_typelib::parse_msft(raw)?;       // raw MSFT bytes only
let lib   = vba_typelib::msft::parse(raw, Limits { max_types: 1000, ..Limits::default() })?;
let res   = vba_typelib::pe::typelib_resources(&pe_bytes)?;   // raw resource bytes + ids
```

`TypeLib` holds `name`, `guid`, `lcid`, `version`, `flags`, `sys_kind`, `doc`,
`imports` and `types`; each `TypeInfo` has `name`, `kind` (Enum, Record,
Module, Interface, Dispatch, CoClass, Alias, Union), `guid`, `flags`,
`version`, `doc`, `implements` (base interface / coclass members, as
`TypeRef::Local(index)` or `TypeRef::Imported(index)`), `alias`, `dll_name`,
`functions` (with `memid`, `kind`, `invoke`, `call_conv`, `flags`,
`vtable_offset`, `ret`, `params`, `optional_count`, `doc`) and `variables`
(record fields, dispinterface properties, enum members and constants).
Parameter and variable types are `TypeDesc` (primitive `VT_*`, pointer,
SAFEARRAY, fixed array, user-defined reference). Constants and parameter
defaults are `Const { vt, value }`. Soft problems that did not stop the read
are listed in `TypeLib::warnings` (none occur on any sample).

All reads are bounds checked and return typed `TypelibError`s
(`NotATypelib`, `UnsupportedFormat`, `Pe`, `NoTypelibResource`, `OutOfRange`,
`Malformed`, `LimitExceeded`, `Io`); malformed input never panics.
`Limits` caps types (131072), members over all types (1 048 576), parameters
(2 097 152) and type-description nesting (32); counts are additionally checked
against the data actually present.

### `vba-sema` adapter

With the default `sema` feature, `vba_typelib::sema::TypelibProvider`
implements `vba_sema::TypeProvider` for one or more loaded libraries.
Dependency direction: `vba-typelib` (interoperability tier) then depends on
`vba-sema` and `vba-value` (language and runtime tiers). Nothing in those
crates depends on `vba-typelib`, so there is no cycle; the reader stays
usable without sema through `default-features = false`. This was preferred
over a feature in `vba-rs` because the adapter needs the reader's model
internals (inheritance, default interfaces, imports across libraries).

Mapping:

- `resolve_type(qualifier, name)`: interfaces, dispinterfaces, coclasses,
  records and unions; an alias of one of those resolves to its target. The
  qualifier is the library name (`Scripting`), compared case-insensitively.
- Members: `[lcid]` and `[retval]` parameters are hidden; the dereferenced
  `[retval]` type is the result; `void`/`HRESULT` members without a retval
  are `Sub`s. Functions of one name merge into one property (get, put,
  putref) with the getter's index parameters. Restricted members
  (`FUNCFLAG_FRESTRICTED`, for example `IUnknown` and `_NewEnum`) are invisible.
  Dispatch variables and record fields are properties (read-only ones are
  `PropertyGet`). The member with id 0 is the default member.
- Types: `I2` Integer, `I4/INT/UINT/UI4/UI2/HRESULT/ERROR` Long, `UI1/I1`
  Byte, `I8/UI8` LongLong, `R4` Single, `R8` Double, `CY` Currency, `DATE`
  Date, `BSTR/LPSTR/LPWSTR` String, `BOOL` Boolean, `DECIMAL` Decimal,
  `VARIANT` Variant, `IDispatch/IUnknown` Object, enums Long, `void*` Any.
  A pointer to an interface is an object passed by value; any other pointer
  is a `ByRef` parameter of the pointee type. A SAFEARRAY is a dynamic array,
  a fixed array carries its rank. `nroargs = -1` marks a `ParamArray` tail.
- A coclass exposes the members of its default (non-source) interface; its
  `[source]` interfaces supply `events`. `describes_members` is `true` only
  when every base interface was found (`IUnknown`/`IDispatch` need no
  members); otherwise `sema` reports `SEM005` instead of `SEM019`.
- `global(name)`: enum and module constants (typed by their stored value),
  module functions, and application-object coclasses.
- Imported types (bases in another library) are resolved by GUID against the
  other libraries given to the same provider.

Gaps, all visible as explicit diagnostics: `vba-sema` has no library enum
type, so `Dim c As Lib.EnumName` and aliases of primitives give `SEM004`
(the enum's members and its parameters work, as `Long`); assignments to
read-only library properties are not diagnosed by sema for library types;
library `Variant` constants of unsupported tags are skipped.

## Test material and its limits

The real libraries available for verification are Wine-built (the Wine
Stable application bundle: `stdole2`, `stdole32`, `activeds`, `mshtml` as
`.tlb` PE stubs, and about 100 DLL/OCX/EXE with `TYPELIB` resources), not
Microsoft's originals. Parsing them validates the reader and the layout below
as written by that toolchain; it does not validate Microsoft's member
ordering, hash tables, optional header fields or the layout choices that
only a Microsoft compiler makes. Everything below marked "verified" means
"verified on those files".

Committed tests use only synthetic libraries and PE images that the tests
build themselves with a small writer that follows this document
(`tests/common`); tests on the real files are `#[ignore]`d, skip cleanly when
the files are absent and run with
`cargo test -p vba-typelib -- --ignored` (set `VBA_TYPELIB_SAMPLES` to search
another directory). No bytes of any real library are committed.

## PE container

A TYPELIB is stored as a resource whose type is the string name `TYPELIB`
(not an id). Verified layout of the resource tree: level 1 entry named
`TYPELIB`, level 2 entries are numeric ids (`LoadTypeLib("x.dll\N")` selects
the id; `vbscript.dll` has ids 1, 2 and 3), level 3 is the language (0
everywhere observed), the data entry points to the raw typelib bytes by RVA.
The reader walks that tree with the `object` crate's resource directory
reader (see `docs/dependencies.md`) and picks the 32/64-bit parser from the
optional-header magic. A PE without a resource directory or without a
`TYPELIB` entry gives `NoTypelibResource`. `.tlb` files from this toolchain
are themselves PE stubs holding the resource; a raw typelib starts with the
signature directly.

Confidence: verified on PE32 and PE32+ files and on synthetic images.

## MSFT layout

All integers are little endian. "Offset" columns are file offsets from the
start of the typelib bytes (the resource data, or the file).

### Header and directory

| Offset | Size | Field | Confidence |
| --- | --- | --- | --- |
| 0x00 | 4 | signature `MSFT` | verified |
| 0x04 | 4 | format version `0x00010002`; other values are rejected | verified (all files) |
| 0x08 | 4 | offset of the library GUID in the GUID table | verified |
| 0x0c | 4 | LCID | verified (0x409; 0 for one library) |
| 0x10 | 4 | second LCID, 0 everywhere | unknown meaning |
| 0x14 | 4 | variable flags; low 4 bits are the target system (1 Win32, 3 Win64; 0x41 on i386 files, 0x43 on x64 files); bit 0x100 is believed to insert one more header word before 0x2c shifting everything after by 4 | system bits verified; bit 0x100 unverified (no sample), implemented as a shift |
| 0x18 | 4 | version: major in the low 16 bits, minor in the high 16 bits | verified (2.0, 1.0, 4.0, 5.5, 1.2) |
| 0x1c | 4 | library flags (`LIBFLAG_*`; 4 hidden observed) | verified |
| 0x20 | 4 | number of types `N` | verified |
| 0x24 | 4 | library help string: offset in the string table, or -1 | verified |
| 0x28 | 4 | library help context | verified (0) |
| 0x2c .. 0x4f | 36 | nine words: `0, a, b, 0, -1, 0x18, 0x20, 0x80, 1` | words at 0x2c, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x4c are constant over all files (0, 0, -1, 0x18, 0x20, 0x80, 1); the words at 0x30 and 0x34 vary from file to file (for example 0xa8 and 0x5cb in `stdole2`) and are not understood; none is read |
| 0x50 | 4 | number of import entries | verified equal to the import table length / 12 in every file; the reader uses the table length |
| 0x54 | 4N | type table: offset of each type record inside the type record segment | verified: entry `i` is `i * 0x64` in all files |
| 0x54 + 4N | 15 * 16 | segment directory | verified |

Each directory entry is `{offset, length, -1, 0xf}`; the last two words are
constant in every file (read as unknown). An absent segment has offset -1 and
length 0. The 15 segments, in directory order:

| # | Segment | Entry size | Confidence |
| --- | --- | --- | --- |
| 0 | type records | 0x64 | verified; length is exactly `N * 0x64` |
| 1 | import infos | 12 | verified |
| 2 | import files | variable | verified |
| 3 | reference table (implemented types of coclasses) | 16 | verified |
| 4 | GUID hash table (128 bytes) | | present; contents ignored |
| 5 | GUID table | 24 | verified |
| 6 | name hash table (512 bytes) | | present; contents ignored |
| 7 | name table | variable | verified |
| 8 | string table | variable | verified |
| 9 | type description table | 8 | verified |
| 10 | array descriptions | variable | verified |
| 11 | custom data (values) | variable | verified for the value uses; custom attributes not interpreted |
| 12 | custom data GUIDs (0x24 bytes) | | present; ignored |
| 13, 14 | unused (offset -1) | | verified absent in all files |

On disk the segments appear in a different order than the directory (type
records, GUID hash, GUID table, reference table, import infos, import files,
name hash, ...), contiguously, and the member data follows the last segment;
the reader uses only the directory.

### Tables

GUID table entry (24 bytes): the 16 GUID bytes (Data1 as u32, Data2 and Data3
as u16, Data4 as 8 bytes), then two words (-1, -1 in all files; hash-chain
fields, ignored). References to GUIDs are byte offsets into this table
(`stdole2`: library GUID at 0 is `{00020430-0000-0000-C000-000000000046}`).

Name table entry: `u32 owner` (-1 for the library name entry; otherwise the
type record offset the name belongs to), `u32 next` (hash chain, ignored),
`u32 info` = `length | byte1 << 8 | hash << 16`, then `length` bytes of text
padded with `0x57` so the entry size is a multiple of 4. Observed values of
`byte1`: 0x00, 0x10, 0x28, 0x30, 0x38 (meaning unknown, never has bit 0 set,
names are read as single-byte text, Windows-1252). Names are referenced by
byte offset; the library name is the first entry whose owner is -1.

String table entry: `u16 length`, text, padded with `0x57` so the entry is a
multiple of 4 bytes and at least 8 bytes long. Referenced by byte offset; -1
means none.

Import info (12 bytes): `u32 flags` (bits 24..31 hold the imported type's
kind, low bits unknown: observed `0x03010000`, `0x03010001`), `u32` offset in
the import file table, `u32` offset of the imported type's GUID.
Import file: `u32` GUID offset (library), `u32` LCID, `u32` version (major
low, minor high), `u16` `(name length << 2) | 1`, name, padded with `0x57` to
a multiple of 4 (the size is 14 + length rounded up).

Reference table entry (16 bytes): `u32 hreftype`, `u32` implementation flags
(1 default, 2 source, 4 restricted, 8 default vtable), `u32` custom data (-1),
`u32` offset of the next entry or -1.

Type references (`hreftype`): bit 0 set means imported: `(h & !1) / 12` is the
index in the import table (verified: `1` is the first, `0xd` the second);
otherwise `h` is the offset of the target in the type table (`index * 0x64`,
verified for coclass members, interface bases and user-defined types).

Type descriptions: a 32-bit value `v`. If bit 31 is set it is an inline
primitive and `v & 0xffff` is the `VT_*` value (the bits 16..30 repeat it,
or give the underlying storage type for `INT`/`UINT`). Otherwise it is a byte
offset into segment 9 where an 8-byte entry `{u16 vt, u16 tag, u32 data}`
sits: `vt` 26 (PTR) and 27 (SAFEARRAY) have a nested description in `data`;
29 (USERDEFINED) has an `hreftype`; 28 (CARRAY) has an offset into segment
10; other `vt` values are primitives. The `tag` is 0x7fff/0x7ffe, or
`0x4000 | vt` for a pointer to a primitive (informational, ignored).
Array description: `u32` element description, `u16` dimension count, `u16`
(8 in the one observed case, ignored), then per dimension `u32` element count
and `u32` lower bound. Verified (`GUID.Data4` is `UI1[8]`).

Constants and default values: a `u32` `v`. If bit 31 is set the value is
inline: bits 26..30 are the `VT_*` tag and bits 0..25 the payload (sign
extended for signed integer tags; verified for `I4`, `INT`, `UI4`, `R4`
(integer payload), `DISPATCH`/`UNKNOWN`/`VARIANT` (payload 0); negative
payloads, `BOOL` and `CY` are unverified). Otherwise `v` is an offset into
segment 11 where a `u16 vt` is followed by the payload: `I4/UI4/INT/UINT/R4`
4 bytes, `I2/UI2/BOOL` 2, `I1/UI1` 1, `R8/DATE/CY/I8/UI8` 8, `BSTR` a `u32`
length and the bytes; entries are padded to 4 bytes with `0x57`. Verified for
`BSTR`, `UI4` and `I4`; the other widths are unverified.

### Type record (0x64 bytes = 25 words)

| Word | Meaning | Confidence |
| --- | --- | --- |
| 0 | `kind (bits 0..3), misc, alignment << 11, index << 16` combined; kinds 0 enum, 1 record, 2 module, 3 interface, 4 dispatch, 5 coclass, 6 alias, 7 union; alignment is in bytes (2, 4, 8) | kind and alignment verified; bits 4..10 unknown; the high half equals the type index |
| 1 | absolute file offset of the member block, or -1 | verified (see below) |
| 2, 3 | 0xd0, 0xb0, ... sizes whose meaning is unknown | unknown, ignored |
| 4, 5 | 3, 0 | unknown, ignored |
| 6 | member counts: functions in the low 16 bits, variables in the high 16 | verified |
| 7..10 | 0 | unknown |
| 11 | GUID offset or -1 | verified |
| 12 | type flags (`TYPEFLAG_*`) | verified |
| 13 | name offset | verified |
| 14 | version (major low, minor high) | verified (0 observed) |
| 15 | help string offset or -1 | verified |
| 16 | help string context | unknown, ignored |
| 17 | help context | read |
| 18 | custom data offset (-1) | ignored |
| 19 | implemented type count in the low 16 bits, vtable size in bytes in the high 16 | verified (vtable size = slots * pointer size) |
| 20 | instance size in bytes (records, interface pointer size otherwise) | verified |
| 21 | alias: type description; module: string offset of the DLL name; interface and dispatch: `hreftype` of the base or -1; coclass: offset of the first reference table entry | verified |
| 22 | e.g. `0x30001`, `0x70002` for types with a base | unknown, ignored |
| 23 | 0 | unknown |
| 24 | -1 | unknown |

A dispinterface with a base shows `1` (the imported `IDispatch`) or -1 in word
21. An interface or dispinterface with more than one implemented type is
read with only the first base and gets a warning.

### Member block

The block is at the absolute offset in word 1. In all 11 libraries checked
for it, the blocks of the types that have members follow one another in type
order, the first starts exactly at the end of the last directory segment and
the last ends exactly at the end of the file (block size = 4 + record area
size + 12 per member). The reader does not rely on that and accepts any
offset inside the file:

```
u32  size of the record area in bytes
     function records (count from word 6, low half), then variable records
     (high half)
u32  member ids,   one per record (functions first, then variables)
u32  name offsets, one per record
u32  record offsets inside the record area, one per record (ignored)
```

The record area is verified to end exactly at its declared size for every
type of every sample (a mismatch would be a warning).

Function record, `rs` bytes:

```
u16 rs      u16 index
u32 return type description
u32 function flags (FUNCFLAG_*)
u16 vtable offset (bytes)      u16 0x.. (in-memory size, ignored)
u32 FKCCIC  bits 0..2 function kind (0 virtual, 1 pure virtual, 2 non-virtual,
            3 static, 4 dispatch), bits 3..6 invoke kind (1 func, 2 get,
            4 put, 8 putref), bits 8..11 calling convention (4 = stdcall)
u16 parameter count            i16 optional parameter count (-1 = varargs)
[help words]                   helpcontext, help string offset, ...
[default values]               one u32 per parameter, only when a parameter
                               has PARAMFLAG_FHASDEFAULT
parameters                     per parameter u32 type description,
                               u32 name offset (-1 none), u32 flags
```

`rs` always equals `24 + 4 * (help words + defaults) + 12 * parameters`.
Help words observed: none (23 000 functions), two (helpcontext, help string),
three (a third word whose meaning is unknown: `0x64` in `stdole2`).
Verified: FKCCIC fields, vtable offsets (every pure-virtual slot of every
interface lies inside the interface's vtable and is pointer aligned, for 32-
and 64-bit files), parameter flags, optional counts (-1 occurs for varargs),
default values (inline and out-of-line). In FKCCIC, bit 12 (0x1000) is set
exactly when a parameter has a default and bit 14 (0x4000) exactly when a
parameter is `[retval]` (23 000 functions, no exception); the high 16 bits
equal the function's own index or that of the paired accessor, meaning
unknown, ignored.

Variable record: `u16 rs` (0x14 in all 1038 records of the seven libraries sampled for it), `u16 index`,
`u32` type description, `u32` flags (`VARFLAG_*`), `u32`
`kind | in-memory size << 16` (kinds 0 per instance, 1 static, 2 constant,
3 dispatch), `u32` field offset (kinds 0/1) or constant value (kind 2, the
constant encoding above). Records longer than 20 bytes would carry help words
like functions (unverified: none occur). Enum members are kind 2, record and
union fields kind 0, dispinterface properties kind 3 (their id is in the id
array).

## Verification performed

- Every `.tlb`, `.dll`, `.ocx` and `.exe` in the Wine bundle's `x86_64-windows`
  and `i386-windows` directories was read: 104 libraries parse (every MSFT
  library, from `stdole2` with 42 types to `mshtml` with 399 types and 22 000
  functions; `vbscript.dll` and `uiautomationcore.dll` hold several), 1315
  files have no `TYPELIB` resource and 2 (`stdole32.tlb` per architecture) are
  SLTG. No warning is produced by any of them.
- Invariants asserted by the ignored test over all of them: names present,
  every type reference resolves, vtable slots are aligned and inside the
  vtable, record fields lie inside the record, enum members are valued
  constants, property accessors of one name share a member id, the
  type-record table, import count and member-block sizes are consistent.
- Content checks on `stdole2`: library name and GUID, `IFont` GUID, the
  `Picture` dispinterface member ids (Handle 0, hPal 2, Type 3, Width 4,
  Height 5), `StdFunctions.LoadPicture` (DLL name, help string, default
  value, optional count), the raw resource bytes parse identically to the
  PE route.
- `scrrun.dll` drives `vba-sema` (`Scripting.Dictionary` members and a
  misspelled member diagnosed).
- Synthetic round trips through the test writer for every type kind,
  constants (inline and out-of-line), defaults, arrays, imports, coclass
  chains, PE32 and PE32+ images with several resources; truncation at every
  length and every single-byte corruption of a library; proptests with random
  bytes, random bytes behind a valid signature and mutated libraries and PE
  images; size limits; the type-description depth limit.

## Open unknowns

- Anything only a Microsoft compiler writes: the optional header word
  (`varflags & 0x100`), custom attributes (`custdata` entries other than
  values), `oCustData` fields, the hash tables, the non-zero second LCID,
  type records with unusual words 2..5, 7..10, 16, 22..24.
- The meaning of header words 0x30 and 0x34, the segment directory's last two
  words, the high half of FKCCIC, the third function help word, the name
  `byte1` values, bits 4..10 of type word 0.
- The member block offset is an absolute file offset in every sample; whether
  Microsoft-built files use the same convention is not established.
- Inline constants: negative payloads, `BOOL`, `CY`; out-of-line widths other
  than `BSTR`, `UI4`, `I4`.
- The SLTG format (the whole `stdole32.tlb` family, and some older
  libraries). A SLTG file starts with `SLTG`; it is detected and rejected with
  `UnsupportedFormat`. No SLTG structure is read.
- The code page of names and strings (read as Windows-1252, ignoring the
  library LCID).
- Behaviour of `vba-automation` consumers: the model exposes member ids,
  invoke kinds and vtable offsets, but no dispatch table is built here.
