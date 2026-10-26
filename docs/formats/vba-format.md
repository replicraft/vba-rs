# vba-format: MS-OVBA reader and writer

Crate: `crates/formats/vba-format`. An independent implementation of the
parts of [MS-OVBA] needed to read and write a VBA project storage
(`vbaProject.bin`, a compound file), to find that storage inside host
documents, and to convert it to and from the `vba-project` model. Section
numbers below refer to MS-OVBA unless another specification is named.

## API

- `read_project(bytes, &Limits) -> Result<VbaProject, Error>` (strict) and
  `read_project_with(bytes, &Limits, ReadMode)` (`Strict` or `Tolerant`)
- `write_project(&VbaProject, &WriteOptions) -> Result<Vec<u8>, Error>` and
  `write_project_with_report`, which also returns a `WriteReport` (changed
  streams, signature streams, whether signatures are invalidated)
- `ProjectStreams`, `read_project_from_streams(&ProjectStreams, ReadMode,
  &Limits)` and `write_project_streams(&VbaProject, &WriteOptions) ->
  Result<(ProjectStreams, WriteReport), Error>`: the same reader and writer
  without a compound file (see "Embedding without a container")
- Module `container`: `extract_project`, `read_document`,
  `locate_ooxml_project`, `replace_ooxml_project` (VBA projects inside OOXML
  packages and legacy Office documents)
- `module_name_map`, `license_keys` (typed `PROJECTwm` / `PROJECTlk`) and the
  `signature` module (signature streams: structure only)
- `compress(&[u8]) -> Vec<u8>` and `decompress(&[u8], limit)` (§2.4.1)
- `codepage` (MBCS conversion via `encoding_rs` plus OEM tables) and `crypt`
  (§2.4.3, used to report protection state)
- `keys`: the `Preserved` keys this crate uses.

## Implemented

| Area | Sections |
| --- | --- |
| Compression and decompression | §2.4.1 |
| `dir` stream: PROJECTINFORMATION, PROJECTREFERENCES (REFERENCENAME, REFERENCEORIGINAL, REFERENCECONTROL, REFERENCEREGISTERED, REFERENCEPROJECT), PROJECTMODULES with MODULEOFFSET, MODULETYPE, read-only/private flags | §2.3.4.2 |
| `PROJECT` stream: module list, name, help, description, ID, protection (CMG/DPB/GC), host extenders, workspace | §2.3.1 |
| `_VBA_PROJECT` header | §2.3.4.1 |
| Module streams: split at MODULEOFFSET, decompress, decode with PROJECTCODEPAGE | §2.3.4.3 |
| Data encryption (read side, and write side for tests) | §2.4.3 |
| `PROJECTwm`, `PROJECTlk`: typed parsing | §2.3.3, §2.3.2 |
| Digital signature streams: structure only | MS-OSHARED §2.3.2.1, §2.3.2.2 |
| Host documents: OOXML, `.doc`, `.xls`, `.ppt` extraction; OOXML replacement | see "Host documents" |

## Code pages

Supported: those `encoding_rs` implements (for example 866, 874, 932, 936,
949, 950, 1250 to 1258, 10000, 20866, 28592 to 28605, 51932, 54936, 65001)
and the OEM single-byte pages **437, 737, 775, 850, 852, 855, 857, 858, 860,
861, 862, 863, 864, 865 and 869**, which come from small tables
(`src/codepage_tables.rs`). The tables are generated from Microsoft's mapping
files published at
<https://unicode.org/Public/MAPPINGS/VENDORS/MICSFT/PC/> (`CP<n>.TXT`) and
were cross-checked byte for byte against an independent implementation (the
Python standard library codecs) when generated. Bytes a file leaves undefined
(for example in 857, 864, 869) decode to U+FFFD and cannot be encoded.
Code page 858 is not published there; it is 850 with 0xD5 mapped to U+20AC,
checked against the same independent implementation (a test pins the single
difference). Code page 874 stays with `encoding_rs` (Windows-874). The unit
tests spot-check each table against entries read off the mapping files, check
that every defined byte round-trips, and that the pages are ASCII compatible
except where the file says otherwise (864, byte 0x25).

Not supported (`Error::UnsupportedCodePage`, never guessed): the Apple code
pages `encoding_rs` lacks (10006, 10010, 10017, 10029, 10079, 10081, 10082)
and 10007, because no Microsoft-published mapping was available to verify
them against (Apple's tables at unicode.org are Apple's, not Microsoft's, and
are not assumed identical); also 1361 (Johab), and the ISO-2022 and EBCDIC
families. A project in such a page cannot be read, because the `dir` stream
and module source need the page; the bytes are not lost, they are simply not
read.

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
  no password, visible), and re-encrypting the documented plaintext with the
  documented seed, project key and ignored bytes (0x07) reproduces the three
  documented hex strings exactly.

Also tested: round trips over varied and >4096-byte inputs including
incompressible data, property tests (`proptest`), truncation of every prefix of
the `dir` stream, thousands of randomized corruptions of `dir`, the compound
file and individual streams (no panics), property tests that damage, truncate
and randomize OOXML packages, legacy documents and PowerPoint streams (no
panics, limits respected), and resource limits.

Not verified: files produced by real Microsoft Office. No Office-generated
fixture was available; all fixtures are built from the specifications. The
writer's output has not been opened by Office.

## Reading: limits and errors

All parsing is bounds-checked and returns a typed `Error`; size fields are
compared with the remaining data before any allocation. `Limits` (defaults):

| Limit | Default |
| --- | --- |
| input file | 256 MiB |
| any stream | 64 MiB |
| decompressed stream | 64 MiB |
| decompressed/compressed ratio (`dir`, modules) | 1024 (valid data cannot exceed about 700) |
| compound file entries | 65536 |
| ZIP archive entries | 65536 |
| ZIP entry / compressed PowerPoint storage ratio | 1024 |

Reading is strict about the `dir` grammar by default (`ReadMode::Strict`):
an unexpected record id is `Error::DirUnexpectedRecord`. In
`ReadMode::Tolerant` records whose id is outside the MS-OVBA grammar are kept
instead (see below); a known record in the wrong place is still an error. It
is lenient about content the model ignores (reserved fields, redundant size
fields, unknown `PROJECT` lines and sections, `HelpFile2`), and prefers the
Unicode half of a string record when present. A module's kind comes from the
`PROJECT` stream; if it has no entry, the `dir` MODULETYPE gives standard or
class.

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

**Unknown `dir` records (tolerant mode).** MS-OVBA fixes the `dir` records by
version, so a record id outside the grammar can only come from a newer or
non-conforming producer. `ReadMode::Tolerant` keeps each such record as an
opaque blob, assuming the shape every other `dir` record has (16-bit id,
32-bit size, payload; this shape is an assumption, it is not specified for
unknown ids). Blobs are stored in order, with an anchor (the number of known
records before them in their scope), in `Preserved` under `keys::DIR_UNKNOWN`
(records outside modules) and `keys::MODULE_DIR_UNKNOWN` (per module). If the
`dir` stream is not otherwise changed it is written back byte for byte; if it
is rewritten, every blob is written again at its anchor. Removing a
`Preserved` entry removes those records on the next rewrite. A truncated
unknown record is an error in both modes. On conforming input both modes give
identical models.

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

**Digital signatures.** Signature streams are preserved verbatim and never
re-signed or verified; the content hashes of §2.4.2 are not computed.
`write_project_with_report` returns a `WriteReport`: `changed_streams` (the
streams whose bytes differ from the project as read, including dropped ones),
`signature_streams`, and `signatures_invalidated`, which is true when the
project carries a signature stream and at least one stream changed. This is
deliberately conservative: it does not work out whether the changed streams
fall in the subset §2.4.2 hashes. Callers should tell the user that the
signature no longer matches. `write_project` itself stays silent.

## Writing: supported edits

For a project read from a file: module source; module description, help
context, read-only and private flags; project name, description, help file,
help context, constants, version, LCIDs, platform and references.

Rejected with `Error::UnsupportedEdit`: adding, removing, renaming, reordering
or re-typing modules; removing a module's source; **supplying source for a
module whose stored source is absent** (error names the module; see
[compiled-projects.md](compiled-projects.md)); editing any module's source
under `StaleCache::Invalidate` while another module is compiled-only (that
would force a recompilation it cannot survive; `StaleCache::Keep` is the
explicit alternative); changing the project ID, protection, host extenders or
workspace.

A project not read from a file (built in memory) is generated from the model:
`dir`, `PROJECT` (unprotected unless flags are set), an empty `_VBA_PROJECT`
and one stream per module. Designer modules, passwords and `PROJECTwm` cannot
be generated. Such output follows the specification but is untested in Office.

## Host documents (`container`)

| Host | Where the project is | Status |
| --- | --- | --- |
| `vbaProject.bin` | the file itself | read and write |
| OOXML (`.docm .xlsm .pptm .dotm .xlam .xlsb` ...) | the part named by the `http://schemas.microsoft.com/office/2006/relationships/vbaProject` relationship of the `officeDocument` part (MS-OFFMACRO §2.1.1.2, §2.2.1.2, §2.3.1.2) | read; replace |
| Word `.doc` | storage `Macros` (MS-DOC §2.1.9) | read |
| Excel `.xls` | storage `_VBA_PROJECT_CUR` (MS-XLS §2.1.7.18) | read |
| PowerPoint `.ppt` | `VbaProjectStg` persist object, reached through `Current User`, the user edit chain, the persist directory, the `DocumentContainer` and its `VBAInfoAtom`; raw or zlib-compressed (MS-PPT §2.1.2 parts 1, 2 and 11, §2.4.10, §2.4.11, §2.10.40 to §2.10.42) | read |

Detection is by content (ZIP or compound file magic), not file name. OOXML
parts are found only through relationships: the package root `_rels/.rels`,
then the main part's relationships. Targets are resolved as OPC part names
(relative, absolute, `..`, percent-encoding, case-insensitive match) and
external targets are ignored. No `officeDocument` relationship, a missing
part, more than one `vbaProject` relationship, or a target outside the package
is an error (`NoVbaProject` or `Package`). The content type of the part is not
checked. MS-XLS names the storage `_VBA_PROJECT_CUR` and calls it "the VBA
Storage specified in MS-OVBA"; because MS-OVBA §2.2.2 requires a VBA storage
to be named `VBA`, it is read as the project root storage, and a
`_VBA_PROJECT_CUR` that holds a `dir` stream directly is returned as
`UnsupportedContainer` rather than guessed at.

For the legacy formats `extract_project` returns a new standalone compound
file with the same streams and storages (contents identical, container layout
new, CLSIDs and state bits and the host's compound file version kept), which
`read_project` reads. Writing a project back into a `.doc`, `.xls` or `.ppt`
is **not** implemented.

`replace_ooxml_project(package, new_vbaProject_bin, &Limits)`:

- Exact (verified by test): entry order; every entry except the project part
  is raw-copied, so its compressed bytes, name bytes, compression method,
  modification time, CRC, sizes, attributes, comment and extra fields are
  identical, and so is the archive comment.
- The project part keeps its name, compression method (deflate if the
  original method is not available), modification time and attributes; its
  compressed bytes, comment and extra fields are new.
- Not byte-exact: the ZIP structure around the entries (offsets, central
  directory, gaps or data before the first entry, ZIP64 records, data
  descriptors) is rewritten by the `zip` crate. For a package that already
  has the layout that crate writes, replacing the project with identical
  bytes reproduces the file exactly (tested); packages from other producers
  keep their entries but may differ in layout.
- Relationships, `[Content_Types].xml` and any signature part are not
  touched. A package without a VBA project cannot gain one this way
  (`NoVbaProject`).

Limits: input size, entry count, and for every ZIP entry or compressed
PowerPoint storage actually read: declared size, actual size, and the ratio of
decompressed to compressed size (`Limits::max_archive_ratio`). The old project
part is never inflated by `replace_ooxml_project`. Duplicate entry names are
rejected. Encrypted ZIP entries, encrypted Office documents
(`EncryptedPackage`) and encrypted PowerPoint files are `UnsupportedContainer`.

Not verified: no Office-generated `.docm`, `.xls` or `.ppt` file was
available; all fixtures are built from the specification text. The
relationship type of a signature part (`vbaProjectSignature`) is not in the
specifications consulted and is not followed;
`signature::read_signature_part` parses a signature part you supply.

## Auxiliary streams

- `PROJECTwm` (§2.3.3): `module_name_map(&project)` returns the NAMEMAP
  entries (MBCS bytes as stored, decoded MBCS name, UTF-16 name).
  `ProjectWm::to_bytes` reproduces any stream that parses.
- `PROJECTlk` (§2.3.2): `license_keys(&project)` returns version 1 streams as
  class id, license key bytes and the required flag. Other versions are
  errors. `to_bytes` writes the flag as 0 or 1.
- Both are preserved and written back unchanged whatever happens to the typed
  view; a malformed stream is an error from these functions only, never from
  `read_project`. Because module renames are rejected, the name map cannot go
  out of date.

## Protection

`CMG`, `DPB` and `GC` are decoded (§2.3.1.15 to §2.3.1.17, §2.4.3). The model's
`Protection` reports `user_protected`, `host_protected`, `vbe_protected`,
`has_password` and `visible`; `locked_for_viewing()` is true when any
protection bit is set or the project is marked not visible, `is_hidden()`
when `GC` is 0, and `notice()` gives a one-line message. **These flags are an
editor restriction, not security.** The module source is stored unencrypted
(only the three values above are obfuscated; §4.2 says this is "to obfuscate
sensitive information, not to secure it"); this crate reads it like any other
project, and readers must not treat the flags as access control. Passwords are
neither verified nor recovered. A CMG value that cannot be decoded leaves the
flags at their defaults; an undecodable DPB counts as a password being
present.

## Digital signatures

`signature::signatures(&project)` and `read_signature_part` find streams
named (ignoring leading control characters and case) `DigitalSignature`,
`DigitalSignatureEx` or `DigitalSignatureAgile`, and parse them as a
`DigSigBlob` / `DigSigInfoSerialized` (MS-OSHARED §2.3.2.1, §2.3.2.2): the
PKCS #7 `SignedData` and the serialized certificate store are returned as
opaque bytes. **The stream names are not defined in MS-OVBA or MS-OSHARED**;
they are the names Office uses. **Nothing is verified**: there is no PKCS #7
parsing, no certificate or chain validation, no timestamp validation and no
content hash (§2.4.2). `signature::is_signed` only means such a stream
exists. A stream that does not parse is reported with its error and is still
preserved.

## Embedding without a container: stream sets

vba-rs does not read database containers. An embedding application that
already reads, for example, an Access database extracts the VBA storage
itself and hands vba-rs a set of named streams. Access keeps these streams as
rows of a system table, so nothing that exists only in a compound file may be
required, and nothing here requires it.

**The flow.**

```text
container reader (the embedder's)  ->  ProjectStreams  ->  read_project_from_streams
                                                              |  (model, edits)
container writer (the embedder's)  <-  ProjectStreams  <-  write_project_streams
```

**What crosses the seam.** A `ProjectStreams` is an ordered map from stream
path to bytes. The paths are the ones in MS-OVBA: `VBA/dir`,
`VBA/_VBA_PROJECT`, `VBA/<module stream name>`, `PROJECT`, and optionally
`PROJECTwm`, `PROJECTlk`, `VBA/__SRP_n`, signature streams, designer streams
(`<Form>/f`, `<Form>/o`, ...) and anything else; unknown streams are kept and
returned unchanged. The bytes are the stream contents exactly as stored (the
compressed `dir`, the module stream with its performance cache followed by the
compressed source, and so on). Lookup is ASCII case-insensitive, like stream
lookup in a compound file; the spelling the embedder used is recorded and
written back. Separators are `/` (`\` is accepted, a leading `/` is dropped).
Iteration order is ascending by upper-cased path, so output is deterministic.
`dir`, `_VBA_PROJECT` and `PROJECT` are required; a module stream named by
`dir` that is missing is `Error::MissingStream`.

Not required and not produced: storage CLSIDs, state bits, timestamps,
sector layout, the compound-file version. A project read from a stream set
carries none of the `ovba:storage:` / `ovba:cfb-version` preserved entries.

**The same code path.** `read_project` opens the compound file, collects its
streams into a `ProjectStreams` and calls `read_project_from_streams`;
afterwards it adds only the container metadata. The compound-file writer
builds the same stream set that `write_project_streams` returns and then
assembles the file. Tests assert that, for every fixture, reading and writing
through a stream set equals reading and writing through a compound file.
Limits (`Limits::max_entries`, `max_stream_bytes`, `max_container_bytes` as the
total, and the decompression limits) apply to stream sets as well, and
malformed, truncated or missing streams are typed `Error`s, never panics.

**What a write-back must do.** `write_project_streams` returns the complete
stream set plus a `WriteReport`.

- A project not edited returns streams byte-identical to the ones read, and
  `changed_streams` is empty. The embedder need not write anything.
- `changed_streams` lists the paths whose bytes differ from the project as
  read, including streams that were dropped. For a project not read from
  streams (built in memory) it lists every stream.
- An embedder that writes back only what changed must: store the bytes from
  the returned set for each listed path that is in the set; **delete** each
  listed path that is not in the set (with the default
  `StaleCache::Invalidate`, an edit drops the `VBA/__SRP_n` compiled caches
  and zeroes the `_VBA_PROJECT` cache so the host recompiles); and leave every
  other stream alone.
- The same edits are supported and refused as for compound files
  (see "Writing: supported edits"), including the compiled-only rules:
  supplying source for a module that has none, or editing source with
  `StaleCache::Invalidate` in a project with a compiled-only module, is
  `Error::UnsupportedEdit`.
- `WriteReport::signatures_invalidated` still tells the embedder that an
  existing signature no longer matches what it signs.

## Version detection and its limits

`VbaProject::project_version()` (also `Project::version()` in the facade)
returns a `ProjectVersion` of facts:

- `version_word`: the 16-bit Version field of the `_VBA_PROJECT` header
  (§2.3.4.1), read after the two-byte reserved constant `0x61CC`. `None` if
  the stream is absent, shorter than four bytes or does not start with the
  constant. `0xFFFF` is the value this crate writes when there is no
  performance cache.
- `sys_kind` (Win16, Win32, Mac, Win64 per PROJECTSYSKIND), `code_page`, `lcid`.
- `reference_hints`: for each registered reference (and each control's
  twiddled and original library) the parsed `guid`, type library version
  (hexadecimal, as written), `lcid`, `path`, and `office_dir`, the number in an
  `Office<N>` path component if there is one. These are raw facts about text in
  the file; they say which machine last saved the reference, not which Office
  built the project.
- `compiled_only`: the project has modules and none has source.
- `profile_hint()`: a `vba-profile` profile identifier or `None`. The table has
  one entry: version word `0x0097` with 32-bit Windows gives
  `vba7.0-excel-win32`. **This is a hypothesis**, to be treated as confirmed
  only by projects from independently known Office builds. The word `0x0097`
  has been observed in a project referencing Office 2010 libraries on a 32-bit
  platform, which suggests VBA 7.0. The host in the returned name is the
  catalog's representative for that generation, not something the project
  states. Every other word, a missing word and `0xFFFF` give `None`; nothing
  is guessed or interpolated.

Limits: the version word is a property of the stream and is not documented by
MS-OVBA as an Office build number; a project can be saved by an application
that does not update it; a project whose cache was invalidated reads `0xFFFF`
and says nothing; the platform is the one the project declares, not the
machine's. Use the facts to inform a choice of profile, never to override one
the user made.

## `vbars inspect`

`vbars inspect <path> [--json]` accepts a `vbaProject.bin`, a `.bas` / `.cls`
/ `.frm` file, or a **directory of exported streams**: every regular file
below the directory is a stream whose path is its path relative to the
directory (`VBA/dir`, `VBA/_VBA_PROJECT`, `PROJECT`, `VBA/Module1`, ...).
Symbolic links are not followed, and file names must be UTF-8. The text report
adds `Version word`, `Profile hint` (`unknown` when there is none, otherwise
the name marked `(hypothesis)`) and the reference version hints.

`--json` prints one JSON object on stdout (keys sorted, pretty-printed,
deterministic). Errors still go to stderr as `error: ...` with exit code 1.
`schema_version` is `1`; fields are only ever added within a schema version.

| Key | Type | Meaning |
| --- | --- | --- |
| `schema_version` | number | layout version, currently `1` |
| `kind` | string | `project` or `source` |
| `input` | string | `compound-file` or `directory` (projects only) |
| `name`, `description` | string | project name and description |
| `code_page`, `lcid` | number | PROJECTCODEPAGE, PROJECTLCID |
| `platform` | object | `name` (for example `32-bit Windows`) and `value` (PROJECTSYSKIND) |
| `project_version` | object | `major`, `minor` of the project's own version |
| `version_word` | number or null | `_VBA_PROJECT` version word |
| `version_word_hex` | string or null | the same, as `0x0097` |
| `profile_hint` | string or null | see above; null when unknown |
| `reference_hints` | array | objects with `reference` (index), `name`, `libid`, `guid`, `type_lib_version` (`"2.5"`, hexadecimal) , `lcid`, `path`, `office_dir`; absent facts are null |
| `compiled_only` | bool | modules exist and none has source |
| `source_availability` | string | `full source`, `mixed` or `compiled-only` |
| `protection` | object | `user_protected`, `host_protected`, `vbe_protected`, `has_password`, `visible` |
| `references` | array | `name`, `kind`, `libid` |
| `modules` | array | `name`, `kind`, `has_source`, `source_availability`, `chars`, `lines`, `performance_cache_bytes` |

For `kind` = `source`: `module` with `name`, `kind`, `has_source`, `chars`,
`lines` and `export_header_lines` (null if none).

## Not supported

- **Stripped or compiled-only projects**: a module stream with nothing after
  the cache is read with `has_source == false` and an empty source, and
  classified by the model (`Module::source_availability`). Compiled code is
  not interpreted, decompiled or disassembled, and source is never supplied
  for such a module. Everything is preserved byte for byte on write. See
  [compiled-projects.md](compiled-projects.md).
- **Passwords**: neither verified, recovered nor written.
- **Writing into legacy documents** (`.doc`, `.xls`, `.ppt`), and **adding a
  project to a package** that has none.
- **Encrypted files** of any kind.
- Designer contents (VBFrame, OFORMS streams) are preserved, not parsed.
- Code pages listed above as unsupported.
- Signature verification, re-signing, and the contents hashes of §2.4.2.
