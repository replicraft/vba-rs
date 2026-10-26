# vba-scripting

Native `Scripting.Dictionary` and `Scripting.FileSystemObject` (with `TextStream`, `File`,
`Folder`, `Files`, `SubFolders`), written against the automation traits of `vba-automation`.
They are ordinary host objects in the object table, so late binding, default members, `For Each`,
`TypeName` and `Err.Source` work as for any other host object.

## Wiring

* `ScriptingHost` decorates a `vba_host::Host`. `vba-runtime`'s `SessionBuilder` always wraps the
  embedder's host in one, so `CreateObject("Scripting.Dictionary")` and
  `CreateObject("Scripting.FileSystemObject")` work in every `Session` (and in the `vba-rs`
  engine and the CLI).
* Creation goes through the inner host's `create_object`: the `ComCreate` grant is checked for the
  ProgID (and `Network` for a remote server), and a factory the embedder registered for the same
  ProgID wins. Only the inner host's error 429 ("no such class") falls through to the native class.
* File access goes through the `ScriptFs` trait. `vba-runtime` implements it over the session's
  `FileSystem` (`ScriptFsAdapter`), so a `FileSystemObject` sees the same virtual file system as
  `Open`, applies `FileRead` for reads/listings/`stat` and `FileWrite` for anything that creates,
  changes or deletes, honours prefix-scoped grants (a grant for `/data` or `/data/` covers the
  folder itself) and cannot reach a path the policy refuses. `..` segments are resolved before the
  check, so `/ok/../x` is checked as `/x`.
* Early binding (`Dim d As New Scripting.Dictionary`) needs a type library and is not provided.

## Dictionary

From the Learn pages (Dictionary object, `Add`, `Exists`, `Items`, `Keys`, `Remove`, `RemoveAll`,
`Item`, `Key`, `Count`, `CompareMode`):

* `Add` of an existing key, `Remove`/`Key` of a missing key and changing `CompareMode` of a
  dictionary that holds data are errors.
* Reading `Item` of a missing key creates it (item `Empty`); assigning a missing key creates it.
* Keys may be anything except an array. `Items`/`Keys` return zero-based arrays; `For Each`
  visits the keys.

**Unverified** (not on the pages): error numbers (457 duplicate key, 32811 not found, 5 otherwise),
key equality (numeric subtypes equal by value, a number never equals a string, strings exact in
`vbBinaryCompare` and by the en-US text collation in `vbTextCompare`, objects by identity, `Empty`
is `""`), insertion-ordered `Keys`/`Items`, and `Null` keys being error 5. Object keys/items are
reference counted (also VM-owned class instances, see `docs/automation.md`).

## FileSystemObject and friends

Provided: `BuildPath`, `CopyFile`, `CopyFolder`, `CreateFolder`, `CreateTextFile`, `DeleteFile`,
`DeleteFolder`, `FileExists`, `FolderExists`, `GetAbsolutePathName`, `GetBaseName`,
`GetDriveName`, `GetExtensionName`, `GetFile`, `GetFileName`, `GetFolder`, `GetParentFolderName`,
`GetTempName`, `MoveFile`, `MoveFolder`, `OpenTextFile`; `TextStream` (`Read`, `ReadAll`,
`ReadLine`, `Skip`, `SkipLine`, `Write`, `WriteLine`, `WriteBlankLines`, `Close`, `AtEndOfLine`,
`AtEndOfStream`, `Line`, `Column`); `File`/`Folder` (`Name` (assignable), `Path`, `ParentFolder`,
`Size`, `Type`, `DateCreated`, `DateLastAccessed`, `DateLastModified`, `Attributes`, `ShortName`,
`ShortPath`, `Copy`, `Delete`, `Move`, and `OpenAsTextStream` / `Files`, `SubFolders`,
`IsRootFolder`, `CreateTextFile`); the `Files`/`SubFolders` collections (`Count`, `Item` by name,
`For Each`).

**Not provided** (the member is absent, so a call is error 438; recorded by the XFAIL program
`file-io/fso_drives_not_provided`): `Drives`, `GetDrive`, `DriveExists`, `GetSpecialFolder`, `Drive`
properties. A virtual file system has no drives or Windows folders to describe.

**Decisions and unverified rules**

* Paths are the virtual file system's canonical absolute paths (forward slashes, `c:/dir` with a
  drive), so `GetAbsolutePathName` and the `Path` properties do not look like `C:\dir`.
  `BuildPath` inserts `\`, or `/` when the path has forward slashes only.
* Error numbers (the pages say only "an error occurs"): 53 file not found, 76 path not found, 58
  file already exists, 70 permission denied (policy, folder opened as a file, read-only file
  without `Force`), 62 input past end (also `ReadAll` of an empty file), 54 for a read from a
  write stream, a write to a read stream or any use after `Close`, 5 for bad arguments.
* ASCII is Windows-1252; Unicode is UTF-16LE with a byte order mark; `TristateUseDefault` is
  taken as ASCII. A write stream keeps its text in memory and writes it on `Close` (or when the last
  reference goes away; an error then has nowhere to go, so close explicitly). `ForWriting`
  empties the file when it is opened.
* One timestamp: `DateCreated` and `DateLastAccessed` equal `DateLastModified`. `ShortName` and
  `ShortPath` equal `Name` and `Path`. `Type` is `"<EXT> File"` or `"File folder"` (Windows asks
  the shell for names like `"Text Document"`). `GetTempName` is a counter (`rad00001.tmp`).
  After assigning `Name` the object still designates the old path.
* Wildcards (`*`, `?`) in the last component are accepted by `CopyFile`, `MoveFile`, `DeleteFile`,
  `CopyFolder`, `MoveFolder` and `DeleteFolder`; a destination with a trailing separator, or an
  existing folder, means "into".

## Tests

`crates/runtime/vba-scripting/tests/host.rs` (policy, precedence, call helpers), unit tests in
`fs.rs`, and the conformance programs `collections/dictionary_*`, `collections/filesystemobject_*`
and `file-io/fso_*`.
