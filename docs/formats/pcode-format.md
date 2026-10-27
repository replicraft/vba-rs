# VBA compiled-code cache: structure specification

Scope: where the compiled ("p-code") data of a VBA project lives, how its layout is selected, and how the tables and per-line code regions are located and interpreted. Opcode details are in `pcode-opcodes.md`.

Confidence tags: **[H]** high (consistently observed), **[M]** medium (observed but with oddities), **[L]** low (weak evidence, unverified, or inferred). "Unknown" means the meaning has not been established.

Terminology: the "module stream" is the OLE stream of one VBA module under the `VBA` storage (as in the public Office VBA file format documentation, MS-OVBA). The "directory stream" is the compressed `dir` stream; the "project stream" is `_VBA_PROJECT`. "x86" means the 32-bit Windows layout and "x64" the 64-bit Windows layout.

## 1. Where the compiled data lives

1.1 The compiled cache is the leading part of every code-module stream. The module's compressed source text begins at the byte offset given by the MODULEOFFSET record of the directory stream (record id 0x0031, field TextOffset, as in MS-OVBA). Everything before that offset is the cache. All offsets below are relative to the first byte of the module stream. **[H]** that the cache starts at stream offset 0, that all table offsets are stream-absolute, and (by MS-OVBA) that it ends exactly at TextOffset.

1.2 The cache is only valid if it matches the host's VBA build. Files saved with the cache stripped, or whose version indicators say the cache is not usable (MS-OVBA: the version word of the project stream is written as 0xFFFF and the cache must be ignored by readers), may have a short or meaningless area. Implementations must range-check every offset (see section 9).

1.3 Directory stream. It is a compressed stream (MS-OVBA compression); after decompression it is a sequence of records: id (U16), size (U32), then `size` bytes of data. One exception: the PROJECTVERSION record (id 0x0009) has size 4 but is followed by 6 further bytes (major version U32, minor version U16), so it occupies 2 + 4 + 6 bytes. This is as documented in MS-OVBA. Module names: each module has a MODULESTREAMNAME record (id 0x001A) whose data is the MBCS stream name, immediately followed by a sub-record with id 0x0032 (size U32, then UTF-16LE stream name). The module stream is looked up by that name. PROJECTCODEPAGE (id 0x0003) has size 2 and gives the code page used for all non-Unicode text in the project; when absent, use Latin-1. **[H]**

## 2. Selecting the layout

Three inputs select the layout.

### 2.1 32-bit vs 64-bit
The directory stream's PROJECTSYSKIND record (id 0x0001, 4-byte value): 0 = 16-bit Windows, 1 = 32-bit Windows, 2 = Macintosh, 3 = 64-bit Windows (this matches MS-OVBA). Value 3 selects the x64 layout, value 1 the x86 layout. **The bitness is selected by this record alone, never by the version word** (2.2). All other values are refused (16-bit, Macintosh, unknown). **[H]**: in 46 projects (492 modules) every project with value 1 parsed and decoded with the x86 arrangement and every project with value 3 with the x64 arrangement, and the version word `0xB2` occurs with both values.

### 2.2 Version word
In the project stream, the 16-bit word at offset 2 (after the 0xCC 0x61 signature word at offset 0) is the internal VBA version. MS-OVBA only says the field is the creating VBA version and that a value of 0xFFFF means "no usable cache". It names the compiler build, not the bitness.

Observed values (46 projects of documents created by Office 2003 to a recent 2016-generation build; the platform kind is the PROJECTSYSKIND of 2.1):

| Version word | Platform kinds seen | Produced by (generation of the creating Office) | Projects |
|---|---|---|---|
| 0x0079 | 1 | Office 2003 (VBA 6) | 3 |
| 0x0088 | 1 | Office 2007 (VBA 6) | 3 |
| 0x0097 | 1 | Office 2010, 32-bit (VBA 7.0) | 4 |
| 0x009A | 3 | Office 2010, 64-bit | 2 |
| 0x00A6 | 3 | Office 2013, 64-bit | 7 |
| 0x00AF | 1 | Office 2016, 32-bit (early build) | 5 |
| 0x00B2 | **1 and 3** | Office 2016, 32-bit and 64-bit (later builds) | 2 and 5 |
| 0x00B5 | 3 | recent builds, 64-bit | 15 |

Earlier versions of this document gave 0x00B2 as the 32-bit and 0x00D7 as the 64-bit word of Office 2016, 0x00A3 for Office 2013 and 0x00D9 for the Mac. That was wrong: 0x00B2 is written by both bitnesses and 0x00D7, 0x00A3 and 0x00D9 were not seen in any project. Bitness is therefore taken from PROJECTSYSKIND, and the version word selects only the family below and the combinations of `VERIFIED_LAYOUTS` (a combination that was not observed, for example 0x00B5 on 32-bit Windows, is decoded with the arrangement of its family and bitness and reported as unverified).

The MS-OVBA header of this stream is 7 bytes: signature U16 (0x61CC), version U16, one byte 0x00, then an undefined U16; the version-dependent cache follows at offset 7.

| Version word | Family | Module layout section |
|---|---|---|
| < 0x6B | VBA5 (VBA3 is not identified; see 9) | 4.2 |
| 0x6B - 0x96 | VBA6 | 4.1 |
| >= 0x97 | VBA7 | 4.1 |

**[H]** for the < 0x6B vs >= 0x6B split (it selects completely different offset arithmetic; no project below 0x6B was available, so the VBA5 arrangement is still implemented from the text only). **[H]** for the VBA6/VBA7 boundary at 0x97 as far as it matters here: VBA6 (0x79, 0x88) and VBA7 (0x97 and above) modules have the same header arrangement and the same x86 opcode numbering; they differ in the identifier index adjustment (5.2).

### 2.3 Endianness
* Module stream: if the 16-bit word at offset 2 read little-endian exceeds 0xFF, the stream is big-endian (Mac). Otherwise little-endian. **[M]**
* Project stream: the 16-bit word at offset 5 equal to 0x000E (read little-endian) indicates big-endian. **[L]** (odd unaligned location; Mac files were not available for confirmation)
* The directory stream is always little-endian.
All structure descriptions below assume little-endian; for Mac read multi-byte fields big-endian (opcode bits then follow the same positions after reading the word big-endian). The one exception is the identifier-entry header (3.3): its two bytes are (type, length) in stream order on big-endian but (length, type) in stream order on little-endian; that is, on little-endian the first byte is the length and the second is the type.

### 2.4 What differs between builds
Differs by version family: module-header offset arithmetic (4.1 vs 4.2); project-stream parsing flags (3.1); identifier index adjustment (5.2: none for VBA6, 4 for VBA7); function-record field offsets (6.3, VBA5 vs later).
Differs by bitness (VBA6/7): the position of the indirect-table length (4.1); record field offsets in the indirect table (6.3, 6.4, 6.6); the scale of object-table references (6.1, 6.7); the identifier index adjustment (5.2: 7 instead of 4); opcode numbering (opcodes document section 2); the size of the function record (88 instead of 64 bytes); the result-record name sentinel (6.6); presumably a second 32-bit operand after the context instruction (not observed).
The same for all VBA6/VBA7 builds examined: the position of the declaration table length (0x3F), object table location rule, line-table format, 0xCAFE marker, line record size (12), the 16-bit opcode word layout, the encoding of variable-length trailers, the sizes of the argument (32), variable (24), user-type (24) and type-descriptor (8) records, the 10-byte object entry.

## 3. The project stream (identifier table)

Names used in the code are not stored in the module; they are indices into an identifier list stored in the project stream (plus a fixed table of built-in names, 5.4). To obtain the list the project stream must be walked structurally, skipping many fields whose meaning is not needed. **This walk is [M]-[L] confidence: it is a long chain of relative skips where one mistake corrupts everything after it.** Verify with the checks in section 10.

### 3.1 Flags derived from the version word `v`
* `wideRefs` = (v >= 0x5B and v not in {0x60, 0x62, 0x63}) or v == 0x4E.
* `wideNames` = (v >= 0x59 and v not in {0x60, 0x62, 0x63}) or v == 0x4E.
* `narrowNames` = (v <= 0x59 and v != 0x4E). (v = 0x59 therefore sets both name flags; both skips then apply.) **[L]** for v < 0x6B (essentially untested for old versions). For v >= 0x6B: `wideRefs` and `wideNames` are true, `narrowNames` false. **[H]**

### 3.2 Layout of the walked region (offsets in the project stream; U16/U32 little-endian)
The signature at offset 0 must be 0x61CC, else there is no usable identifier table. The region consists of the following, in order. "String16" means a U16 length followed by that many bytes; "String16F" is a String16 where length 0xFFFF means absent (no payload).

1. At offset 0x1E: U16 `nRefs`, then 2 bytes to skip.
2. `nRefs` reference records, each:
   a. U16 `L`. If `L` == 0, 6 bytes follow. Otherwise `L` bytes follow, except: when (`wideRefs` and L >= 5) or (not `wideRefs` and L >= 3), the byte at +4 (`wideRefs`) or +2 (otherwise) inside those `L` bytes is examined, and if it is the ASCII letter 'C' or 'D' a String16 follows the `L` bytes. When L is smaller than the threshold the `L` bytes are simply skipped.
   b. 10 bytes.
   c. U16 `flag`. If non-zero: a String16; then U16 `M`; if `M` is non-zero, 2 more bytes; then `M` + 30 bytes.
3. A U16 count followed by that many 2-byte entries (form/class table); a U16 count followed by that many 4-byte entries (compile-time constant pairs); 2 bytes.
4. Three String16F values (type-info id, project description, help file), then 0x64 bytes.
5. U16 `nModules`, then `nModules` module descriptors, each:
   - U16 `L`; if `wideNames`, `L` bytes follow; if `narrowNames` and `L` != 0, a U16 `L2` and `L2` bytes follow (when `L` == 0 nothing further is read in this step), then a String16 (no 0xFFFF escape);
   - String16F; U16; if v >= 0x6B, String16F; String16F; 2 bytes; if v != 0x51, 4 bytes; a U16 count followed by that many 8-byte entries; 11 bytes.
6. After the modules: 6 bytes; a block with a **U32** length prefix; 6 bytes.
7. U16 `a`, U16 `b`, U16 `c`; 4 bytes. Define `nIdents = a - c` and `nSkipped = b + c - a`. Semantics (low confidence): `b` is the total number of entries, `a - c` of them are real identifiers, and a prefix of the rest are reserved/dummy entries; the arithmetic is as stated. **[L]**
8. `nSkipped` dummy entries, each: 4 bytes, a 2-byte (type, length) header (3.3), 6 more bytes if type > 0x7F, then `length` bytes.
9. `nIdents` identifier entries (3.3). The identifier list index starts at 0 with the first entry that yields text.

### 3.3 Identifier entry
Header: 2 bytes giving (type, length) (byte order per 2.3).
* If both are zero: 2 bytes follow and a new 2-byte header is read. This entry is *reserved/internal* and has no trailing 4 bytes.
* If bit 0x80 of type is set: 6 bytes follow.
* If length > 0: the next `length` bytes are the name text in the project code page; it is appended to the list. Zero-length entries append nothing, which shifts later indices. **[L]**: whether the original compiler also leaves holes in its numbering is unknown.
* If the entry is not reserved/internal: 4 trailing bytes (hash/flags, meaning unknown).

Type bits other than 0x80 are not interpreted. **[M]**

## 4. Module-stream header and table locations

All positions are stream offsets; fields are U32 unless noted. A "table" below is a U32 byte length followed by that many bytes unless a different location is given.

### 4.1 VBA6 / VBA7 (version word >= 0x6B)

| Item | Where | Notes |
|---|---|---|
| Declaration table | length at 0x3F (x86 **and** x64); bytes start 4 later (0x43) | **[H]** (all 492 modules of 46 projects; earlier text put the x64 length at 0x43, which reads 0xFFFFFFFF there) |
| Indirect table | let `P` = U32 at 0x11. Length at `P` + 10 (x86) or `P` + 12 (x64); bytes start 4 later | **[H]** (the x64 position was exact for all 410 x64 modules, the x86 one for all others) |
| Object table | let `Q` = U32 at 0x05. Length at `Q` + 0x8A; bytes start 4 later | **[H]** |
| Line-table pointer | U32 `R` at 0x19; the line table header is at `R` + 0x3C | **[H]** |

The first 0x3F bytes are identical in arrangement on x86 and x64 (the pointers at 0x05 to 0x1D move, nothing else). The word at offset 2 of a module stream is the platform kind the cache was compiled for: 1 in every module of a project with PROJECTSYSKIND 1 and 3 in every module of a project with PROJECTSYSKIND 3 (492 of 492 modules, VBA6 included). A decoder checks it against the layout and refuses a module that disagrees. The same arrangement holds for the VBA6 modules (version words 0x79 and 0x88).

### 4.2 VBA5 (version word < 0x6B)
Positions follow from a sequence of variable-size items starting at offset 11:
1. At 11: a U32-length-prefixed block = declaration table (length at 11, bytes at 15).
2. Following it (at 15 + length): 64 bytes.
3. A U16 count followed by that many 16-byte entries.
4. A U32-length-prefixed block; 6 bytes; a U32-length-prefixed block. The position after these is `X`.
5. Indirect table: U32 `S` at `X` + 8; length at `S` + 10, bytes start at `S` + 14. Object table: U32 `Q` at `X`; length at `Q` + 0x8A, bytes at `Q` + 0x8E.
6. Line-table pointer: U32 `R` at `X` + 77; line table header at `R` + 0x3C.
**[M]** (the content of the 16-byte entries is unknown).

### 4.3 Table bounds
Bound each table to the stream and to its stated length. A table whose declared length extends past the end of the stream is invalid.

### 4.4 Reference list and member table (VBA 7.0 x86; the same extents hold on x64 and in VBA6)
Observed:

* The type-library reference list runs from the pointer at 0x09 to the member table (or to the line-table pointer when there is no member table). Layout: dword 0x5AFEB7A9, word (value 9), then entries `01 | U16 byte length | UTF-16 text`, then the byte 0xDF. **[H]**
* The member table (class modules; pointer at 0x15, 0xFFFFFFFF in standard modules) is a U16 count followed by `count` records of three dwords. It fills the span up to the line-table pointer exactly. The meaning of the dwords is unknown. **[H]** for the extent, **[L]** for the content.
* The 60 bytes between the line-table pointer and the line table header are a block of unknown content. **[M]**

## 5. Identifier references

### 5.1 Reference word
Name operands in the code and name fields in records are 16-bit words. The low bit is dropped (`k = word >> 1`); its meaning is **unknown**. **[M]**

### 5.2 Resolving `k`
* If `k >= 0x100`: user identifier. Let `j = k - 0x100`. For VBA6 (version word below 0x97) `j` indexes the identifier list of section 3 directly. For VBA7 (version word >= 0x97) subtract 4 on x86 and 7 on x64. An out-of-range index means "unknown name". **[H]** for all three: the difference between `j` and the index of the identifier whose spelling is on the source line was constant over every name operand of 46 projects (up to 1,100 identifiers, 14,000 operands in the largest), for the version words 0x79, 0x88 (offset 0), 0x97, 0xAF, 0xB2 on 32-bit Windows (offset 4) and 0x9A, 0xA6, 0xB2, 0xB5 on 64-bit Windows (offset 7). A 32-bit and a 64-bit build of the same source gave the same names with the offsets 4 and 7 (1,893 of 1,893 name operands). The step "subtract one more above 0xBE" that earlier versions of this document described does not exist: it was tried and made about 6,800 of the 11,300 name operands of the largest project resolve to a wrong identifier.
* A reference whose index lies outside the identifier table, or a built-in index with no confirmed spelling, is still an identity: equal words are the same name. The name word of the result record in 6.6 is 0xFFFE on x86 (the disassembly shows it as `?ident(32507)`) and 0xFFFF on x64, which is also the word of an implicit `Me` in code: names in records (as opposed to operands) are therefore never `Me`. The same sentinel is carried by the nameless symbol records of the indirect table. The lowering gives such a parameter or local a placeholder spelling and resolves nothing else to it.
* If `k < 0x100`: built-in name from the fixed table in 5.3. For VBA7: if `k >= 0xC3` subtract 1 first. **[L]**: this adjustment is still unverified; the only operand seen at or above 0xC3 in all the material is a single `.Write` member (raw index 0xC3), which does not settle it. Raw indices below 0xC3 are unaffected and were checked on VBA6, VBA7 x86 and x64 (the same spelling for the same index everywhere it was seen).

### 5.3 Built-in names

An index below 0x100 (after the adjustments of 5.2) selects a built-in name: a
fixed table of about 255 spellings of VBA keywords, attribute names and operator
or punctuation tokens, indexed from 0. The table is not reproduced in this
document because its order and some of its slots could not be confirmed; an
implementation must establish it by observation:

1. For each built-in reference found in a project that also has source, align the
   reference with the source token at the same position and record
   `index -> spelling`.
2. Keep an index only when it was observed at least once and never contradicted;
   leave every other index as `unknown` and report it when it is met, never guess.
3. Sanity expectations: the table is roughly alphabetical for keywords, with
   attribute names after the keywords and operator and punctuation tokens in the
   last rows; a few slots are reserved or duplicated.

Each confirmed entry is a fact about the format and may be added to a table in
the implementation together with the project and position it was observed at.

Status: 29 entries are confirmed (`CONFIRMED_BUILTINS`). Ten were added from the
VBA6 and 64-bit material (index: spelling, uses): 9 Array 105, 11 Assert 136,
13 Base 430, 77 Error 4, 116 Line 9, 137 Object 133, 139 Open 14, 164 Select 63,
182 Type 10, 185 Unload 3. In every case the spelling is the only
identifier-like token shared by all lines that use the index.

## 6. Tables

### 6.1 Object table
* Entries are 10 bytes on x86 and on x64 **[H]**. A reference to an entry, whether the operand of an "obj" instruction or the word in a type descriptor (6.7), is the entry **index times the pointer size**: index `* 4` on x86 and index `* 8` on x64; the byte offset of the entry is index `* 10`. **[H]** on x64 (292 `New` and `TypeOf` operands and 858 variable types, every one matching the class named on the source line; two `TypeOf` operands name a library-qualified type and do not); on x86 for the descriptors (11 of 11) and for the `obj` operands of the one project that has any (6 operands, all multiples of 4, two of them naming a class that sits at the entry `operand / 4`). Earlier text called the operand "a byte offset, a multiple of 10, used verbatim"; that does not hold (operand 28 occurs).
* Within an entry: U16 at +0 flags (bit 0x02 = the entry names a list of identifiers rather than a single name; this case is **poorly understood**); U16 at +6 = identifier reference of the type/class name.
* Observed: the word at +6 is an identifier reference only for entries whose word at +2 has bit 0 clear. With bit 0 of the word at +2 set, the entry names a library-qualified type (`Lib.Type`) and the word at +6 is not an identifier reference. **[M]**
* Sanity bound: `offset + 8 <= table length`. **[M]**
* Entry 0 is an ordinary entry (a reference with index 0 names it).

### 6.2 Indirect table
A heap of variable-size records referenced from code by 32-bit byte offsets (the "rec" operands) and from other records. Record kinds (function, variable, argument, user-defined-type, type descriptor) are distinguished only by the context in which they are referenced; there is no kind tag. Record layouts follow.

### 6.3 Function (procedure / property / declare) record
Minimum size needed: x86 offset + 62, x64 offset + 84. Observed record size (spacing of reachable records): 64 bytes on x86 (VBA6 and VBA7, 67 of 76 spacings), 88 bytes on x64 (2,464 of 2,531 spacings). **[H]** Let `base` = 0 for VBA5, 4 for VBA6/VBA7 x86, 4 + 16 for VBA6/VBA7 x64, and `late` = `base` on x86 and `base + 6` on x64.

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | flags: 0x0008 (VBA5 only) public when set; 0x0020 has declared return type; 0x0080 Static; 0x1000 Sub or Function; 0x2000 Property Get; 0x4000 Property Let; 0x8000 Property Set |
| 2 | 2 | identifier reference: procedure name |
| 36 + base | 4 | offset (in indirect table) of the first argument record, 0 or 0xFFFFFFFF if none |
| 40 + base | 4 | return type field: a type field as defined in 6.4a; only meaningful when flag 0x0020 is set |
| 44 + base | 2 | offset into the declaration table of the external-library record; 0xFFFF if none |
| 54 + late | 1 | option byte: if (value & 0x90) == 0 and the declaration offset is not 0xFFFF, the procedure is an external `Declare` |
| 55 + late | 1 | argument count |
| 57 + late | 1 | VBA6+ flags: bit 0x02 set = Public (clear = Private); 0x04 Friend (a `Friend` procedure has 0x02 as well); 0x20 PtrSafe |

The 6 bytes between the declaration offset (+2 bytes) and the option byte are 8 bytes on x86 (`+50` to `+57`) and 14 bytes on x64 (`+66` to `+79`); the extra six are zero in the projects seen.

Additionally, the declaring instruction's MOD bit 0x04 means the `Public` keyword was written (not the effective visibility: a `Sub` without a keyword is public with MOD bit clear), and MOD value 2 or 6 means "Function" for flag 0x1000 (opcodes document 3.2). For `Declare` procedures the library name is the identifier reference at (declaration table offset + 2).
Verification **[H]** on x86 and x64, VBA6 and VBA7 (2,693 procedures of 46 projects against the source of the same procedure): the name, the kind (Sub/Function, Property Get/Let/Set), the `Declare` test (the record is a `Declare` exactly when the source line says so), `Static`, `Friend`, visibility (bit 0x02: public unless the keyword `Private` was written) and `PtrSafe` (20 of 20 `Declare` statements) all agree. The `Declare` test reported no other procedure as external.
Observed: flag 0x0020 means a declared return type; a Function without `As` has the return word 0xFFFF000C (Variant; seen once).
Unknown: everything else in the 61+ bytes. Two procedures of the same name in a 32-bit and a 64-bit build differ in the low bits of the flag word and in the option byte (for example 0x0C against 0x0B and 0x94 against 0x9C on event handlers of a sheet module); the meaning of those bits is not known.

### 6.4 Variable record
Minimum size: offset + 16.

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | flags 1: 0x20 = has an `As` type clause |
| 1 | 1 | flags 2: 0x20 = declared `New` |
| 2 | 2 | identifier reference: variable name |
| 12 (x86) / 16 (x64) | 4 | type descriptor position (6.7), valid when flags 1 bit 0x20 is set |

The type position holds a type field (6.4a). **[H]** on x86 and x64 (the names of 3,185 variable records and the types of 2,083 intrinsic and 869 named ones agree with the source). Observed record size: 24 bytes on x86 and x64 (2,424 of 3,000 x64 spacings, 47 of 51 on x86). **[H]**

### 6.4a Type fields
Observed (VBA 7.0 x86 and x64): every type field in the variable (+12 x86, +16 x64), argument (+12 x86, +16 x64) and function return-type (+40 + base) positions is a dword:

* 0xFFFFFFFF: no type.
* upper 16 bits 0xFFFF: intrinsic type; the low byte is the type id (6.8), whose bits 0xE0 are flags.
* otherwise: the dword is an offset in the indirect table of an 8-byte type descriptor (6.7).

The same rule serves all three positions. **[H]** for VBA 7 on x86 and x64. On x64 `LongPtr` is the intrinsic id 20 with the pointer flag 0x80 (8 of 8 variables); the corresponding 32-bit encoding was not seen.

### 6.5 User-defined-type record
Minimum size offset + 20. U16 at +2: identifier reference of the type name; U16 at +18: option word, bit 0 clear = Private. **[M]** Observed record size: 24 bytes on x86 and on x64 (132 of 133 x64 spacings). The name field was checked on x64 (the `Type` names agree with the source); the layout is the same as on x86 there. **[M]**

### 6.6 Argument record
Reached through the argument chain of a function record. Let `a` = 4 on x64, else 0.

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | flags: 0x0020 = has `As` type; 0x0040 is set on most arguments and its meaning is undetermined (observed: it is not the ParamArray marker) |
| 2 | 2 | identifier reference: argument name |
| 12 + a | 4 | type field (6.4a) |
| 20 + a | 4 | offset of next argument record (0 or 0xFFFFFFFF ends the chain) |
| 24 + a | 2 | option bits: 0x0004 ByVal, 0x0002 ByRef, 0x0200 Optional, 0x0400 Optional with a default value, 0x0001 ParamArray (observed, VBA 7.0 x86). The default values themselves are instructions on the header line, before the `PROC_HEADER` instruction and after a `CONST_EXPR_WITH_CALL` marker, in reverse argument order (430 of 430 header lines agree with the number of 0x0400 arguments) |

Sanity bound: offset + 26 + a within the table. **[M]** Observed record size: 32 bytes on x86 and x64 (3,235 of 3,577 x64 spacings; the four filler bytes at +28 of the x86 record are the shifted option word on x64). **[H]** The x64 offsets hold for the names of 2,487 arguments in 46 projects and agree argument by argument with the 32-bit build of the same source (23 of 23 arguments with name, ByVal/ByRef, Optional and type).

**Result record `[obs]`.** In class-like modules the argument chain of a `Function` and of a `Property Get` has one more record at its end: the result. Its name word lies outside the identifier table (5.2; `0xFFFE` on x86, `0xFFFF` on x64), its type field is the return type, and the argument-count byte of the function record counts it. 54 of 54 functions and 20 of 20 `Property Get` procedures of the class-like modules of the first (x86) project have it (these are the 74 argument records without a resolvable name noted earlier); `Property Let` chains hold only the parameters, and no function of a standard module has the record. A reader that takes the chain for the parameter list sees a phantom last parameter.

### 6.7 Type descriptor
U16 flags at +0. If flag 0x02 is set: simple type, id in the byte at +6 (6.8). Otherwise U16 at +2 is the object-table reference of the type: the entry index times the pointer size (4 on x86, 8 on x64; 6.1), and the entry's identifier reference at +6 names the class. Index 0 is an ordinary entry: `Dim x As Thing` has the value 0 when `Thing` is entry 0 (earlier text read 0 as "no type name"). Observed flag values for object and record types: 0x001D, 0x101D, 0x401D (a user-defined type of the project), 0x601D; the low byte 0x1D is the user-defined/object kind and the high bits are unexplained flags. **[H]** (the class name agreed with the source for 858 variables on x64 and 11 on x86, none disagreed; a type whose entry has bit 0 of the word at +2 set is library-qualified and is not resolved). Observed size: 8 bytes on x86 and x64 (3,504 of 3,810 x64 spacings). **[H]**

### 6.8 Intrinsic type ids
1 Null, 2 Integer, 3 Long, 4 Single, 5 Double, 6 Currency, 7 Date, 8 String, 9 Object, 10 Error, 11 Boolean, 12 Variant, 14 Decimal, 17 Byte, 20 LongLong (with flag 0x80 it is `LongPtr` on x64), 29 user-defined type (observed for arrays and records of user-defined types; the type name is not in the field); the top three bits (mask 0xE0) are flags, of which bit 0x80 means pointer (`LongPtr`-style pointer-sized variant of the type). Other ids unknown. These coincide with the OLE automation variant-type numbering. **[H]**

### 6.9 Declaration table
Holds records for external-library `Declare` statements and similar. U16 identifier reference at (record offset + 2) = library name, where the record offset is the value from the function record.

Observed (VBA 7.0 x86): a declaration record has a 20-byte header: the library reference at +2 and a word at +4 that is the offset, in the declaration table, of a NUL-terminated text (the entry-point name, the `Alias` text, or the procedure name itself). The text follows the header and is padded to 4 bytes, so records are variable in size. **[M]** Other contents **unknown**.

## 7. Line table and code regions

At position `T` = (line-table pointer from section 4) + 0x3C:

| Offset from T | Size | Field |
|---|---|---|
| 0 | 2 | marker 0xCAFE; if different, the module has no usable code region. **[H]** |
| 2 | 2 | unknown |
| 4 | 2 | `nLines` |
| 6 | 12 x nLines | line records |
| 6 + 12 x nLines | 10 | unknown trailer |
| then | | the code area; `codeBase` = T + 6 + 12 x nLines + 10 |

Line record (12 bytes): +0 4 bytes unknown; +4 U16 `length` (bytes of code for the line); +6 2 bytes unknown; +8 U32 `offset` relative to `codeBase`. The line's instructions occupy `[codeBase + offset, codeBase + offset + length)`. Lines are numbered 0..nLines-1 and correspond to source lines of the module in order; a zero length means no code (blank, or a line with nothing to emit). Whether attribute lines and lines before the first declaration are counted is **unknown**. **[M]**

No secondary or duplicate code area is known. **[L]** (negative evidence only).

Observed (VBA 6 and 7, x86 and x64): lines with code occupy 8-byte-aligned slots; the line length is not rounded, but the next offset advances by the rounded length. The padding bytes hold stale data, not zeros. After the last slot come 8 bytes (0, 16 or 24 in 11 of 419 x64 modules and 5 of 55 VBA7 x86 modules; always 8 in the 18 VBA6 modules), then a 6-byte trailer at the offset stored in the dword at 0x1D (that dword equals the cache length minus 6); the trailer is FF FF FF FF 00 00 in every module observed. **[H]**

The slots are not always contiguous: bytes of the code area between the slots are claimed by no line (x64: 329 interior gaps, 287 of them 8 bytes and the others 16 to 80, and one module whose first slot starts at +8; VBA7 x86: 15 interior gaps and 2 such modules; VBA6: one gap of 16). They are probably left by edits. The gaps do not affect decoding, which goes by the line records. The first project examined, and the small documents, have none.

Source lines that start with `Attribute` have **no line** in the cache, wherever they stand: the `Attribute VB_...` lines at the top and also those after a procedure header (`Attribute Item.VB_UserMemId = 0`, `Attribute Foo.VB_Description = "..."`). With every such line dropped from the source, the number of source lines equals `nLines` in all 492 modules of 46 projects (with only the leading ones dropped, 39 of 123 modules of one project differed). A line the compiler did not compile is stored as a `TEXT_UNPARSED` line carrying the source text: the unselected branch of an `#If`/`#Else`, and a `Declare` without `PtrSafe` in a 64-bit build. In a project with both forms of a `Declare` the cache therefore has a procedure header for one and plain text for the other.

Offsets for lines need not be monotonic or contiguous; bound-check against the stream. Declarations, attribute lines, conditional compilation, events and procedure headers are not stored specially: they are ordinary lines whose instructions are the declaration/marker opcodes in the opcodes document (a procedure header line uses PROC_HEADER pointing to a function record; a variable declaration line uses VAR_DECL). **[M]**

## 8. Operand resolution summary

* Names: 16-bit identifier references (5).
* Object/class: 16-bit object-table offsets ("obj", 6.1).
* Records: 32-bit indirect-table offsets ("rec").
* Strings and comments: inline text in a length-prefixed trailer, in the project code page (default Latin-1).
* Numbers: inline little-endian words, low word first for wider values; floating-point literals are the raw IEEE bytes; Currency is a scaled integer (x10000). Date is an IEEE double.
* External library members: only visible via a Declare procedure, as the library name (declaration table, 6.9) plus the procedure name; type-library member ids (DISPIDs) are **not** known to be stored. Alias names are **unknown**.
* Early- versus late-bound: both appear to use the same member-access opcodes; no separate dispatch-id or early-bound vtable offset is known in the instruction stream. **[L]** (negative evidence).

## 9. Known limitations and unknowns

* The VBA3 layout is not distinguished by the version-based selection in 2.2; the VBA3 opcode mapping in the opcodes document is low confidence. **[L]**
* Library-qualified type names (`Lib.Type`) and the type of a variable declared with one are not resolved: the object entry has bit 0 of the word at +2 set and does not hold the names. `ParamArray` on x64 was not seen.
* The first word of `IMPLEMENTS_STMT` (a multiple of 8 on x64, different for each interface; the 32-bit form was not seen) names the interface through a table that has not been identified; it is not an object-table reference.
* Never seen on any layout: `PROC_CONTEXT` (so the second operand dword on x64 is unverified), the three 8-byte integer literals, VBA5, VBA3, big-endian streams.
* Project-stream structures (reference records, form tables, module descriptors) are skipped, not decoded.
* Alternative locations of line data, "reserved" module headers and Mac-specific details are untested.
* One built-in name table entry is unresolved (5.3).

## 10. Verification checklist

Run against real compiled projects (one per version/bitness if possible):

1. Directory stream: after decompression, record ids walk cleanly to the end; PROJECTSYSKIND is 1 (x86) or 3 (x64) as expected; exactly one MODULESTREAMNAME record (with its 0x0032 sub-record) appears per module.
2. Project stream: first word 0x61CC; version word is one of the values of 2.2; after the walk in section 3.2, the cursor must end close to the stream end (the identifier area is the tail); the number of text entries read must equal `nIdents` minus zero-length entries.
3. All of the following offsets lie inside the module stream and before TextOffset: declaration table (0x3F + 4 + length), indirect table, object table, `R + 0x3C`.
4. The word at the line-table position is 0xCAFE for every code module with compiled data.
5. `codeBase + offset + length <= TextOffset` for every line; the covered bytes do not overlap unexpectedly; the largest `offset + length` is no larger than the code area size.
6. Each line decodes opcode-by-opcode to exactly its length (the last instruction ends exactly on `length`). This is the strongest check of the opcode table and operand widths for your build (x86 vs x64 numbering).
7. Every name operand resolves (identifier index in range, or built-in table index in range). Compare resolved names against the module's source text: they must appear in the source (case may differ).
8. Every "obj" operand, scaled to a byte offset (6.1), + 8 is within the object table; every "rec" operand + required minimum size is within the indirect table.
9. A function record's name must resolve to a name that appears in the source as a `Sub`/`Function`/`Property` header at the corresponding line.
10. 32-bit and 64-bit variants of the same source should yield the same sequence of canonical opcodes except for the 8-byte integer literals and context operand size. Observed: a 32-bit and a 64-bit build of the same 25-module source gave the same opcode and modifier sequence on all 1,021 lines with code, equal literal operands, and equal argument, variable and procedure information apart from the record offsets and low flag bits.
