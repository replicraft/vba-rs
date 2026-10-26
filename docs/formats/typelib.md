# Type libraries (`vba-typelib`)

`vba-typelib` reads Windows type libraries, the metadata VBA binds
early-bound calls to: libraries, coclasses, interfaces, dispinterfaces,
enums, records, aliases and modules, with their members (functions and
property accessors with member ids, invoke kinds, parameter names, types,
flags and defaults), constants, GUIDs, help strings, version, LCID and flags.
It works on bytes only: no COM, no code is loaded, no `unsafe`.

Inputs: a raw MSFT-format or SLTG-format file (`.tlb`, `.olb`) or a PE image
(`.dll`, `.ocx`, `.exe`, 32- or 64-bit) whose `TYPELIB` resources are read.
The format is chosen by the first four bytes (`MSFT` or `SLTG`); both readers
produce the same `TypeLib` model. SLTG coverage and gaps are listed in "SLTG
layout" and "SLTG coverage and known gaps".

The crate lives in `crates/interoperability/vba-typelib`. It is independent
of the `vbars` CLI; `cargo run -p vba-typelib --example dump -- FILE
[--members] [--type NAME]` prints the libraries of a file.

## Interface

```rust
let lib   = vba_typelib::load(&bytes)?;          // raw typelib or first TYPELIB resource of a PE
let libs  = vba_typelib::load_all(&bytes)?;      // every library, with its resource id
let lib   = vba_typelib::load_file(path)?;
let lib   = vba_typelib::parse_any(raw)?;        // raw bytes of either format, by magic
let lib   = vba_typelib::parse_msft(raw)?;       // raw MSFT bytes only
let lib   = vba_typelib::parse_sltg(raw)?;       // raw SLTG bytes only
let lib   = vba_typelib::msft::parse(raw, Limits { max_types: 1000, ..Limits::default() })?;
let lib   = vba_typelib::sltg::parse(raw, Limits::default())?;
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
`parse_msft` given SLTG data answers `UnsupportedFormat`, `parse_sltg` given
anything else `NotATypelib`. `Limits` (shared by both readers) caps types (131072), members over all types (1 048 576), parameters
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
reader (see `docs/architecture/dependencies.md`) and picks the 32/64-bit parser from the
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

## SLTG layout

SLTG is the older compound format. The layout below was written from the
publicly documented structure of the format and then checked byte by byte
against the only SLTG files available, the two `stdole32.tlb` stubs (i386 and
x86_64, same layout, six types: `GUID`, `DISPPARAMS`, `EXCEPINFO`,
`IUnknown`, `IDispatch`, `IEnumVARIANT`). "Verified" below means "decoded
correctly on those files" and nothing more; fields without a sample are
marked. All integers are little endian, a `pstring` is a `u16` length
followed by the bytes (`0xffff` = none), names are Windows-1252.

### File structure

| Offset | Size | Field | Confidence |
| --- | --- | --- | --- |
| 0x00 | 4 | signature `SLTG` | verified |
| 0x04 | 2 | `n`: number of index names (types + 2) | verified |
| 0x06 | 2 | 9 | unknown meaning, ignored |
| 0x08 | 2 | 0x4f | unknown meaning, ignored |
| 0x0a | 2 | `first`: 1-based number of the first block entry in file order | verified |
| 0x0c | 16 | GUID `{000204FF-0000-0000-C000-000000000046}` | constant, ignored |
| 0x1c | 8 | `0x44`, `0xffff0000` | unknown, ignored |
| 0x24 | 8 (n-1) | block entries | verified |
| next | variable | the index: `n` NUL-terminated names | verified |
| next | 9 | zero padding | verified |
| next | sum of the lengths | the blocks, in chain order | verified |

A block entry is `{u32 length, u16 index_string, u16 next}`: `index_string`
is the offset of the block's name inside the index, `next` the 1-based number
of the entry whose block follows in the file (`0` ends the chain). The reader
follows the chain from `first`, requires it to cover every entry, and accepts
blocks in any physical order (the sample has them in entry order; other
orders are exercised with synthetic files only). The index starts with
`\x01CompObj` (no block), then `dir`, then one stream name per type (the
samples use `BAAAAAAAAA`, `DAAAAAAAAA`, ...). In the samples the `dir` block
comes last. Bytes after the last block are ignored.

### The `dir` block

| Part | Layout | Confidence |
| --- | --- | --- |
| library block | `u16 0x51cc`, `u16 3`, `u16` library name (name table offset), `u16 0xffff`, `pstring` help string, `pstring` help file, `u32` help context, `u16` syskind, `u16` lcid, `u32 0`, `u16` library flags, `u16` major, `u16` minor, 16-byte GUID | verified (help file none and help context 0 only) |
| gap | 0x40 bytes of `0xff` | verified |
| type list | `u16` type count, then per type: `pstring` stream name (matches an index name), `pstring` second stream name, `u16 0xffff`, `u16` type name (name table offset), `u16` `m`, `m` bytes, `u16 0xffff`, `u32` help context, `u16 0xffff`, 16-byte GUID, `u16` type kind (`TKIND_*`) | verified for `m = 0`; the `m` extra bytes are skipped (believed to be a help string, not decoded) |
| gap | 0x216 bytes: a 22-byte record and the 512-byte name hash table (`0xff` in the samples) | verified, contents ignored |
| name table | `u16` size, `u16 0`, then the entries | verified |

The order of the type list is the type index of the model. A name table entry
is eight bytes of `0xff` (hash chain, ignored), the NUL-terminated name,
padded to an even length; a name is addressed by the byte offset of its
entry, and `0xfffe`/`0xffff` mean no name. The library, type, member and
parameter names all live in this table.

### Type block

Offsets are relative to the start of the block.

| Offset | Field | Confidence |
| --- | --- | --- |
| 0x00 | `u16 0x0501` | verified |
| 0x02 | `u32` offset of the reference table, or `0xffffffff` | verified (records have none) |
| 0x0a | `u32` offset of the member header | verified |
| 0x12 | `u32` version (major low, minor high) | verified as 0 only |
| 0x1a | `u8 f1`, `u8 f2`: type flags = `f1 >> 3 \| f2 << 5` | assumed; verified for flags 0 only |
| 0x1d | `u8` type kind (checked against the type list, a difference is a warning) | verified |
| 0x22 | the reference table, when present | verified |

Reference table: `u16 0x00df`, 66 bytes of `0xff`, `u32 number` (8 bytes per
reference), `number + 2` bytes of `0xff`, five bytes `01 00 00 00 00`, then
`number / 8` `pstring`s, then a byte `0xdf`. The strings are `*\Rffff*#N`
(type `N`, hexadecimal, of this library: `TypeRef::Local(N)`) or
`*\Rxxxx*#N` with `xxxx` the name table offset of a
`*\G{guid}#major.minor#lcid#file#` entry describing another library
(`TypeRef::Imported`, one `Import` per distinct library and type; only the
type's number is stored, so `Import::type_guid` is `None`). Local references
are verified; imported ones are implemented from the public description and
have no sample.

Member header (9 bytes: `u16 1`, `u16 0xffff`, `u8 1`, `u32` byte count
`c`), then the `c` bytes of member data (the "group"), then a fixed 0x36-byte
tail:

| Tail offset | Field | Confidence |
| --- | --- | --- |
| 0x00, 0x02, 0x04 | function, variable, implemented-type counts | verified |
| 0x08, 0x0a, 0x0c | group offsets of the functions, variables and implemented types (`0xffff` = none) | verified |
| 0x20, 0x22 | instance size, alignment | verified (records) |
| 0x28 | vtable size in bytes | verified |
| others | `0xffff`, 0, or sizes of the member areas | ignored |

All offsets inside the group are relative to the start of the group.

Implemented-type record (22 bytes): byte 0 `0x4a`; `u16` next at 2
(`0xffff` last); byte 6 `IMPLTYPEFLAG_*`; at 18 a `USERDEFINED` type
description whose reference is at 20. Verified for one base interface with
flags 0; the default and source bits and a chain of several records are
synthetic only.

Function record: byte 0 `0x6c` (with function flags) or `0x4c` (without; no
sample); byte 1 the invoke kind in the high nibble (1 function, 2 get, 4
put, 8 putref; low nibble 2); `u16` next function (group offset, `0xffff`
last) at 2; name at 4; `i32` member id at 6; `u16` help context at 10 and help
string at 12 (not decoded); `u16` offset of the argument table at 14; byte 16
`nacc` = `argument count << 3 | 4`; byte 17 `0x80`; `u16` return type at 18;
`u16` vtable offset at 20; `u16` function flags at 22 (`0x6c` only). The
argument table is four bytes per argument: `u16` name (bit 0 set means the
type is inline) and `u16` type word or group offset. The return type is
inline when bit 7 of byte 17 is set, otherwise a group offset; every sample
function has an inline return type, so the out-of-line form is unverified.
Functions of a dispinterface get `FuncKind::Dispatch`, all others
`PureVirtual`.

Variable record: byte 0 `0x2a` (with variable flags) or `0x0a` (without,
0x12 bytes, no sample); byte 1 flags (`0x02` the type word is inline, `0x40`
a constant, `0x08` the constant is stored in the offset field); `u16` next at
2; name at 4; `u16` field offset or inline constant at 6; `u16` type word or
group offset at 8; `i32` member id at 10; help context and string at 14 and
16; `u16` variable flags at 18. Record and union fields are `PerInstance`
with their offset, dispinterface variables `Dispatch`. Verified: record
fields with inline types and a fixed array. Constants with an inline value
(`0x08`) follow the public description and have no sample; constants stored
out of line are not decoded (`ConstValue::Unsupported` and a warning).

Type words are shared by arguments, returns and variables. The first word
also carries the parameter flags: `0xc000` neither in nor out, `0x8000` in
and out, `0x4000` out, otherwise in; `0x2000` lcid; `0x0080` retval. The low
six bits are the `VT_*` tag and `0x0e00` (all three bits) adds a pointer. A
`PTR` tag adds a pointer and continues with the next word; `USERDEFINED` is
followed by a word holding four times the index into the reference table;
`CARRAY` by the group offset of a 32-bit `SAFEARRAY` header (16 bytes, then a
`u32` count and an `i32` lower bound per dimension) and continues with the
element type; `SAFEARRAY` by an unused offset and continues; any other tag
ends the description. Verified: primitives, pointers to primitives and to
user types, pointer chains (`void**`) and the `UI1[8]` array. A `SAFEARRAY`
parameter is synthetic only.

## SLTG coverage and known gaps

Decoded into the model: library name, GUID, version, lcid, flags, syskind,
help string and help context; types with name, kind, GUID, flags, version,
alignment, instance and vtable size, base interface and coclass members (with
their flags); functions and property accessors with member id, invoke kind,
vtable offset, flags, return type and parameters (name, type, in/out, lcid,
retval); record and union fields, dispinterface properties and enum members
with inline values.

Known gaps, reported as a warning or a plain absence rather than an error
where possible:

- Type, function and variable help strings are not decoded (`doc` is `None`;
  the help contexts of members are read).
- Module DLL names and entry points, and alias targets: `dll_name` and
  `alias` stay `None`, and an alias gets a warning (the field holding an alias
  target is not known with confidence).
- Optional and default parameters (`optional_count` is 0, `default` is
  `None`), custom attributes, and function flags beyond the stored word.
- Constants stored out of line (strings, wide integers) are
  `ConstValue::Unsupported`.
- Imported types cannot be resolved by GUID (only a type number in the other
  library is stored), so the sema adapter treats a base interface in another
  library as unknown.
- Type flags are `f1 >> 3 | f2 << 5`, confirmed only for the value 0.
- Only 32-bit `SAFEARRAY` headers are read (the x86_64 `stdole32.tlb` uses
  the same layout). The fixed gaps (`0x40`, `0x216`, 9 bytes after the index)
  are assumed; a library with other gaps fails with `Malformed` or
  `OutOfRange` instead of being guessed at.

## Verification performed

- Every `.tlb`, `.dll`, `.ocx` and `.exe` in the Wine bundle's `x86_64-windows`
  and `i386-windows` directories was read: 104 libraries parse (every MSFT
  library, from `stdole2` with 42 types to `mshtml` with 399 types and 22 000
  functions; `vbscript.dll` and `uiautomationcore.dll` hold several), 1315
  files have no `TYPELIB` resource and 2 (`stdole32.tlb` per architecture) are
  SLTG, read by the SLTG reader (106 libraries in all). No warning is
  produced by any of them.
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
- `stdole32.tlb` (SLTG): library name and GUID, the six types and their
  GUIDs, `IDispatch.Invoke` (member id, eight parameters, vtable offset), the
  `GUID` record (`Data4` as `UI1[8]`), the nine `EXCEPINFO` fields; the
  rendered members match the well-known OLE definitions.
- Synthetic SLTG libraries written by the test writer
  (`tests/common/sltg.rs`, which follows the layout above): every type kind,
  imports, fixed-array and SAFEARRAY descriptions, both function and variable
  record forms, blocks in a non-sequential chain order, PE images mixing both
  formats, the `vba-sema` adapter; truncation at every length, every
  single-byte corruption, every word set to an extreme value, random bytes
  and mutated files. The writer encodes the same assumptions as the reader, so
  it shows consistency, not fidelity; only `stdole32.tlb` checks the layout
  against real data.
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
- SLTG: the layout was verified on two files of one library, so every field
  that varies between libraries is unverified: out-of-line constants and
  return types, imported references, the `0x4c` and `0x0a` record forms,
  alias, module and enum blocks, the type list's extra bytes and the type
  flag encoding (see "SLTG coverage and known gaps").
- The code page of names and strings (read as Windows-1252, ignoring the
  library LCID).
- Behaviour of `vba-automation` consumers: the model exposes member ids,
  invoke kinds and vtable offsets, but no dispatch table is built here.
