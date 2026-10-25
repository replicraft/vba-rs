# vba-format: MS-OVBA reader and writer

Crate: `crates/formats/vba-format`. An independent implementation of the
parts of [MS-OVBA] needed to read and write a VBA project storage
(`vbaProject.bin`, a compound file) and convert it to and from the
`vba-project` model. Section numbers below refer to MS-OVBA.

## API

- `read_project(bytes, &Limits) -> Result<VbaProject, Error>`
- `write_project(&VbaProject, &WriteOptions) -> Result<Vec<u8>, Error>`
- `compress(&[u8]) -> Vec<u8>` and `decompress(&[u8], limit)` (§2.4.1)
- `codepage` (MBCS conversion via `encoding_rs`) and `crypt` (§2.4.3, used
  only to report protection state)
- `keys`: the `Preserved` keys this crate uses.

## Implemented

| Area | Sections |
| --- | --- |
| Compression and decompression | §2.4.1 |
| `dir` stream: PROJECTINFORMATION, PROJECTREFERENCES (REFERENCENAME, REFERENCEORIGINAL, REFERENCECONTROL, REFERENCEREGISTERED, REFERENCEPROJECT), PROJECTMODULES with MODULEOFFSET, MODULETYPE, read-only/private flags | §2.3.4.2 |
| `PROJECT` stream: module list, name, help, description, ID, protection (CMG/DPB/GC), host extenders, workspace | §2.3.1 |
| `_VBA_PROJECT` header | §2.3.4.1 |
| Module streams: split at MODULEOFFSET, decompress, decode with PROJECTCODEPAGE | §2.3.4.3 |
| Data decryption (read side) | §2.4.3.3 |

Code pages: those `encoding_rs` implements (for example 874, 932, 936, 949,
950, 1250 to 1258, 10000, 20866, 28592 to 28605, 51932, 54936, 65001). Others
return `Error::UnsupportedCodePage`; they are never guessed. OEM pages such as
437 and 850 are not available.

## Verified against the specification

Tests reproduce these worked examples:

- §3.2.1, §3.2.2, §3.2.3: decompression of the documented byte arrays, and
  compression of the documented text to exactly the documented bytes.
- §3.1.2 (dir stream): the PROJECTINFORMATION, the four references
  (registered, registered, project, control) and the three module records are
  assembled byte by byte from the spec tables, parsed field by field, and
  re-serialized to identical bytes.
- §3.1.6 (PROJECT stream): parsed completely; a one-line patch changes only
  that line.
- §3.1.6 CMG/DPB/GC values decrypt to the expected state (unprotected,
  no password, visible).

Also tested: round trips over varied and >4096-byte inputs including
incompressible data, property tests (`proptest`), truncation of every prefix of
the `dir` stream, thousands of randomized corruptions of `dir`, the compound
file and individual streams (no panics), and resource limits.

Not verified: files produced by real Microsoft Office. No Office-generated
fixture was available; all fixtures are built from the specification. The
writer's output has not been opened by Office.

## Reading: limits and errors

All parsing is bounds-checked and returns a typed `Error`; size fields are
compared with the remaining data before any allocation. `Limits` (defaults):

| Limit | Default |
| --- | --- |
| input file | 256 MiB |
| any stream | 64 MiB |
| decompressed stream | 64 MiB |
| decompressed/compressed ratio | 1024 (valid data cannot exceed about 700) |
| compound file entries | 65536 |

Reading is strict about the `dir` grammar: an unexpected record id is
`Error::DirUnexpectedRecord`. It is lenient about content the model ignores
(reserved fields, redundant size fields, unknown `PROJECT` lines and sections,
`HelpFile2`), and prefers the Unicode half of a string record when present.
A module's kind comes from the `PROJECT` stream; if it has no entry, the
`dir` MODULETYPE gives standard or class.

## Preservation guarantees on write

`read_project` stores, in the model's `Preserved` slots, the original bytes of
the `dir`, `PROJECT` and `_VBA_PROJECT` streams, each module's performance
cache and compressed source, every other stream and storage (designers,
`PROJECTwm`, `PROJECTlk`, `__SRP_*`, signatures, unknown data) and the CLSID
and state bits of every storage.

**Exact (stream contents byte-identical):**

- An untouched project: every stream and storage, including caches and
  unknown data.
- Any stream whose modelled content did not change. Change is detected by
  comparing the model with a fresh parse of the original bytes, so ignorable
  differences in the original (reserved values, cookies) do not cause a
  rewrite.
- All streams the crate does not interpret, always.

**Behaviorally equivalent rewrite (bytes differ):**

- `dir` stream, when any of its modelled fields (including module offsets)
  changed. It is serialized canonically and recompressed: reserved fields take
  the values the spec requires on write, cookies are `0xFFFF`, and the compressed
  bytes are this crate's compressor output, which may differ from Office's.
- A module stream whose source changed: cache (see below) plus freshly
  compressed source. Source is re-encoded in the project code page; characters
  the page cannot hold are `Error::Unencodable`, never substituted.
- `PROJECT` stream, when name, description, help file or help context changed:
  only those lines are replaced (`Key="value"`), the rest is byte-identical.

**Stale caches.** The performance cache is compiled state. A host that trusts
a cache that no longer matches the source runs the old code. Default
`StaleCache::Invalidate`, when any module source changed: the edited module
loses its cache (MODULEOFFSET becomes 0), `_VBA_PROJECT` is replaced by the
7-byte header with version `0xFFFF` and no cache (§2.3.4.1: ignored on read,
must not be present on write), and `__SRP_*` streams are dropped. Other module
streams are left as they are. `StaleCache::Keep` keeps every cache byte and
changes only the edited module's compressed source; use it only if the
consumer rebuilds caches.

**Container.** The compound file is re-laid out (sector order, free space).
Stream contents, the version (3 or 4), storage CLSIDs and state bits are
preserved; timestamps are set to a fixed value so output is reproducible.

**Digital signatures.** Signature streams are preserved verbatim, but any edit
invalidates a signature. Nothing is re-signed or verified, and the content
hashes of §2.4.2 are not computed.

## Writing: supported edits

For a project read from a file: module source; module description, help
context, read-only and private flags; project name, description, help file,
help context, constants, version, LCIDs, platform and references.

Rejected with `Error::UnsupportedEdit`: adding, removing, renaming, reordering
or re-typing modules; removing a module's source; changing the project ID,
protection, host extenders or workspace.

A project not read from a file (built in memory) is generated from the model:
`dir`, `PROJECT` (unprotected unless flags are set), an empty `_VBA_PROJECT`
and one stream per module. Designer modules, passwords and `PROJECTwm` cannot
be generated. Such output follows the specification but is untested in Office.

## Not supported

- **Stripped or compiled-only projects**: a module stream with nothing after
  the cache is read with `has_source == false` and an empty source. Compiled
  code is not interpreted or decompiled. Source can be supplied for such a
  module by setting it.
- **Locked and password-protected projects**: CMG, DPB and GC are decoded to
  report `Protection`. Module source is not encrypted by these flags and is
  read normally. Passwords are neither verified nor recovered, and cannot be
  written.
- **Encrypted files** (for example an encrypted OOXML package) and
  **wrapper formats**: only a raw `vbaProject.bin` compound file is read.
  Extraction from OOXML zip packages and from `.doc`/`.xls` sub-storages is
  not implemented.
- `PROJECTwm` and `PROJECTlk` are preserved but not parsed, so renaming
  would not update them (renaming is rejected anyway).
- Designer contents (VBFrame, OFORMS streams) are preserved, not parsed.
- Records outside the grammar above in the `dir` stream are an error, not
  skipped.
