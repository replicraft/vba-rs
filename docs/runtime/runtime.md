# vba-runtime

`crates/runtime/vba-runtime` orchestrates a VBA program on top of `vba-vm`: it
loads a project, applies the trust gate, provides the file system behind the
file statements and gives the embedder by-name invocation and a classified
error type. It has no dependency on any host application.

```rust
let mut session = Session::builder()
    .host(host)                    // permissions, limits, objects, UI (default: headless, deny all)
    .vfs(MemFs::new())             // or RealFs::new()
    .debug_sink(sink)
    .build_project(&project)?;     // parse -> sema -> IR -> VM; nothing runs
session.trust();                   // the embedder's explicit decision
session.run_procedure("Module1.Main", vec![])?;
```

## Session

- **Loading** (`build_project`, `build_sources`): parse, semantic analysis with
  the injected `TypeProvider`, IR lowering and verification, then a `Vm`.
  Errors are `Error::Load` with the rendered diagnostics; warnings are kept
  (`Session::warnings`). Project conditional-compilation constants and the
  project name (`Err.Source` default) come from the `VbaProject`.
- **Compiled-only projects.** `build_project` first classifies the project
  (`VbaProject::source_availability`). `CompiledOnly` projects are always
  refused with `Error::CompiledOnly` naming the project and the modules
  lacking source, before any analysis. `Mixed` projects are refused unless
  the builder was given `allow_partial()`; then the modules without source are
  left out of the program, source that refers to them fails to compile
  (`Error::Load`), and `run_procedure` naming such a module, or a bare name
  found nowhere (it might live in a missing module), returns
  `Error::CompiledOnly`. P-code is never executed. See
  [compiled-projects.md](compiled-projects.md).
- **Trust gate.** A session never runs anything by itself, not even
  `Auto_Open`, `Workbook_Open` or `Class_Initialize` of default instances.
  `run_procedure`, `raise_event` and `shutdown` return `Error::NotTrusted` until
  the embedder calls `Session::trust`.
- **Invocation by name** (`Application.Run`-like): `Proc` (standard modules,
  case-insensitive) or `Module.Proc`. Ambiguous bare names, private procedures,
  procedures of class modules without a default instance and properties are
  rejected with distinct errors. A procedure of a document or designer module
  runs on the module's default instance. `raise_event("Workbook", "Open", args)`
  runs `Workbook_Open` of a document module (`Ok(None)` when nothing handles
  it).
- **Calls with arguments** (`Session::call(&Target, name, positional, named)`):
  values, `Arg::ByRef` variables whose final values come back in `RunOutput::byref`
  (converted to the declared parameter type, error 13 when impossible; `ByVal`
  parameters and `ParamArray` elements are reported unchanged), omitted (`Missing`)
  and named arguments (448 unknown name, 446 given twice, 449 required missing,
  450 too many). `Target::Project` resolves a name like `run_procedure`;
  `Target::Object` looks in the object's own module (`Sub`, `Function`,
  `Property Get`, public only).
- **Instances.** `create_object(module, host_base)` makes a new instance of a class,
  document or designer module (own module state, `Class_Initialize`);
  `default_instance` returns the borrowed predeclared one; `destroy_object` gives
  back the creator's reference (`Class_Terminate` runs when it was the last).
  A *host base* object receives the members the module does not define
  (`Me.X`, `Me!X`, `Me("X")`, listed unqualified names), late bound.
  `raise_event_on(obj, source, event, ..)` runs `<source>_<event>` of that
  instance whatever its visibility; `has_procedure`, `get_member`, `set_member`.
- **Module variables.** `get_global`/`set_global` read and assign public variables
  and constants of standard modules (`Module.Name`, or `Name` when unique) with the
  VBA conversions; `modules()`, `list_public_members()`, `modules_defining(name)`
  describe modules, procedures with parameter lists and variables, private event
  handlers included.
- **Errors.** `ErrorReport` carries `Err`'s fields, module, procedure, `Erl` and
  source line and the structured call stack (`frames`, innermost first),
  captured at the raise point; `Session::last_error()` gives the same for the
  error `Err` currently holds (also when the program handled it). The default
  `Err.Source` of an unhandled error is the project name.
- **Re-entrancy.** `HostContext` (`Ops`) is the session's operations for a host
  member that is running a call made by VBA; see [vm.md](vm.md#re-entrancy).
- **`reset()`** discards module state, objects (no `Class_Terminate`), `Err`,
  files and the budget counters; the trust decision is kept.
- **Isolation.** Each session owns its VM: module variables, default
  instances, `Err`, the random generator and open files are per session.
- `shutdown` releases module-level objects (running `Class_Terminate` in
  declaration order) and closes files, writing them back.

## Error taxonomy

| `Error` | Who is responsible |
| --- | --- |
| `Load` | the project does not compile (rendered diagnostics) |
| `CompiledOnly { project, modules }` | the project (or the module a call needs) has no source text; compiled state is never run |
| `NotTrusted`, `NoSuchProcedure`, `AmbiguousProcedure`, `NotCallable`, `NoSuchMember`, `AmbiguousMember`, `NotAccessible` | the embedder's request |
| `Reentrancy` | a `HostContext` call that cannot start (stale handle, depth limit) |
| `Vba` | an unhandled error raised by the program or the language (with procedure, line, stack) |
| `Host` | an unhandled error reported by a host hook or host object |
| `PermissionDenied` | a capability the policy refused (error 70) |
| `Bridge` | the `Declare` bridge could not make a call |
| `Limit` | steps, memory, call depth, string length or array size |
| `Cancelled` | the host's idle or `poll_cancel` hook asked to stop |
| `Unsupported` | a feature this implementation lacks |
| `Internal` | a broken invariant (a bug) |

Only `Vba`, `Host` and `PermissionDenied` start as trappable VBA errors
(`On Error` sees them as numbers such as 70 or the host's number); the others
end the run regardless of `On Error`.

## File system layer

All file statements and functions (`Open`, `Close`, `Print #`, `Write #`,
`Input #`, `Line Input #`, `Get`, `Put`, `Seek`, `Lock`, `Unlock`, `Width`,
`Name`, `Kill`, `MkDir`, `RmDir`, `ChDir`, `ChDrive`, `CurDir`, `Dir`,
`FileCopy`, `FileLen`, `FileDateTime`, `GetAttr`, `SetAttr`, `FreeFile`, `EOF`,
`LOF`, `Loc`, `Seek()`, `Input()`, `FileAttr()`, `Reset`) run against a `Vfs`:

- `MemFs`: in memory, case-insensitive and case-preserving, drive roots and `/`,
  deterministic timestamps (2000-01-01 unless `set_time`).
- `RealFs`: the real file system, opt-in. Paths are resolved with symbolic
  links *before* the policy is asked (`Vfs::canonical`), so a link inside a
  granted directory cannot reach outside it. Hidden/system attributes cannot be
  set.

Paths are normalized against a virtual working directory: backslashes become
slashes, `.` and `..` are resolved lexically (`..` stops at the root), drive
letters are lower-cased; a network path keeps `//server/share`. `CurDir` returns that normalized form (`/dir`,
`c:/dir`), or `/` while the working directory is not one the policy covers. `ChDir` changes only the virtual directory, never the process's.

**Permissions** (`vba_host::Policy`, deny by default, whole-path-segment scopes
on the normalized path): reading needs `FileRead` (`Open` for `Input`, `Binary`/
`Random` with read access, `Dir`, `ChDir`, `FileLen`, `GetAttr`, `FileDateTime`,
`FileCopy` source); anything that creates, changes or deletes needs `FileWrite`
(`Open` for `Output`/`Append`/write access, `Kill`, `MkDir`, `RmDir`, `Name`,
`FileCopy` target, `SetAttr`). `Binary`/`Random` without `Access` need both.
Operations on a directory itself check the path with a trailing slash, so the
grant `/data/` covers `ChDir "/data"`. Denial is error 70 with the resource in
`Err.Source`. A path that leaves a scoped grant through `..` is denied because
the policy sees the normalized path.

**Semantics** (documented VBA behavior, not verified against Office):

- Open files are loaded into memory and written back at `Close`, `Close` with no
  arguments, `End`, `Reset` and when the file system is dropped; file numbers
  that open the same path share one buffer, so a `Put` on one is seen by a
  `Get` on another; the memory is
  charged to the host budget and a file larger than `max_file_bytes` (64 MiB) is
  error 61. Text is Windows-1252 with CR LF line ends.
- `Open`: `Input` needs the file (53); `Output` truncates; `Append` positions at
  the end; `Binary`/`Random` create a missing file when writing. A second `Open`
  of a path that is open for `Output` or `Append`, or for either of those while
  it is open, or of an open file number, is error 55; `Input`, `Binary` and
  `Random` channels may share a path when their `Lock` clauses agree (MS-VBAL
  5.4.5.1): a clause that denies the access (read, write) of the other channel
  is error 70, in either direction. File numbers outside 1..511 are 52, and a
  `Len` outside 1 to 32767 is error 59 (the text names no number; `Binary`
  ignores `Len`).
- `Print #` uses the `Debug.Print` layout (leading/trailing space on numbers,
  14-column zones, `Tab`, `Spc`, `Width`). `Write #` writes `"text"`, numbers
  without padding, `#TRUE#`/`#FALSE#`, `#NULL#`, dates as `#yyyy-mm-dd#`,
  `#hh:mm:ss#` or both, `#ERROR n#`, and a comma between items; embedded quotes
  are not escaped.
- `Input #` skips leading blanks and line breaks, reads quoted or comma/line
  delimited tokens and converts them to the target type (numbers like `Val`);
  into a `Variant` the token decides the subtype. `Line Input #` accepts CR, LF
  and CR LF. Reading past the end is error 62.
- `Get`/`Put` layout: scalars little endian, `Boolean` -1/0 in two bytes,
  `String * n` space padded ANSI, variable strings raw in binary mode and with a
  two-byte length in random mode, `Variant` with a two-byte type tag, UDTs and
  fixed arrays as the concatenation of their members; random records are padded
  to `Len` (error 59 if larger). Unknown `Variant` tags are error 321.
- `Dir` supports `*` and `?`, the `vbDirectory`/`vbHidden`/`vbSystem`
  attributes, and a plain name to test existence. `Kill` accepts wildcards.
- `Lock`/`Unlock` (MS-VBAL 5.4.5.4, 5.4.5.5) keep a list of locked ranges per
  file number: records on a `Random` file, bytes on a `Binary` file; `To n`
  starts at 1 and a single number locks that unit. A start below 1 or above the
  end, or an `Unlock` whose range equals no locked one, is error 5 (the text
  says "an error"; the number is **unverified**). `Unlock` without a range
  clears every lock of the file number, `Close` drops them, and on `Input`,
  `Output` and `Append` files the range is ignored (the whole file). Another
  file number on the same file gets error 70 from `Get`, `Put`, `Input`,
  `Line Input`, `Print` and `Write` that touch a locked unit; there is no
  other process to lock against.
- `Width` (MS-VBAL 5.4.5.7): a `Print` item that does not fit on the rest of
  the line starts a new line, one longer than the width is broken into lines of
  that width; `Write` breaks at the width character by character.
- `Write #` (MS-VBAL 5.4.5.9): `Spc(n)` writes spaces and `Tab(n)` moves to the
  column; a comma follows an item unless it ends the list without a separator;
  a list ending in `;` or `,` does not end the line.

- `Input(n, #f)` (**doc**) reads `n` characters (bytes, Windows-1252) from a file
  opened for `Input` or `Binary` (error 54 otherwise, an assumption) and raises
  error 62 when fewer remain. `FileAttr(f, 1)` returns the mode (Input 1, Output 2,
  Random 4, Append 8, Binary 32); return type 2 is error 5 (**unverified** number).
  `Reset` closes every open file and writes the buffers.

**Not implemented:** `ChDrive` to another drive (error 68), `Get`/`Put` of
`Decimal` and of variants holding arrays or objects, UNC paths. `Lock` clauses
and `Lock` ranges only restrict other file numbers of the same session, never
other processes.

## Scripting classes

Every `Session` answers `CreateObject("Scripting.Dictionary")` and
`CreateObject("Scripting.FileSystemObject")` with the native objects of `vba-scripting`
(`docs/language/scripting.md`), through a `ScriptingHost` that wraps the embedder's host. The
`FileSystemObject` uses the session's `FileSystem` (`ScriptFsAdapter`), so its file access has the
same policy checks and scoped grants as the file statements.

## Other services

- `Shell` needs the `ProcessSpawn` grant for the command line **and** a
  `ShellHook` in the builder; without one it is an `Unsupported` error even when
  granted. `StdShell` (opt-in) splits the command line like a Windows command
  line (white space separates, double quotes group) with the same reader the
  permission check uses and starts the program directly, detached, on every
  platform: no `sh -c` and no `cmd /C`, so shell syntax in the line has no
  effect and a scoped `ProcessSpawn` grant names the program that runs.
- `Command` returns the configured command line (empty by default).
- `Environ`, `Now`, `Date`, `Timer` come from the injected
  `vba_stdlib::Host`: a fixed clock (2000-01-01) and an empty environment by
  default; `SystemHost` (opt-in) reads the real UTC clock and environment.
- `MsgBox` / `InputBox` go to the host's `UiHooks` (headless: error 70).
- `CreateObject` / `GetObject` go to the host (`ComCreate`, `FileRead`
  checks first); `Declare` as in `docs/runtime/vm.md`.
- `Debug.Print` goes to the `DebugSink`.

## CLI

`vbars` (crate `vba-cli`). Wherever a file is expected, `-` reads standard input (a standard
module named `Stdin`; diagnostics name it `<stdin>`). `--color auto|always|never` controls
ANSI colours in diagnostics: `auto` colours only a terminal and never when `NO_COLOR` is set.
The `--json` outputs carry `schema_version` 1 and 1-based `line`/`column` positions.
`vbars --help` lists the exit codes. A command-line usage error exits 2, the same code `run`
uses for an unhandled run-time error; scripts that need to tell them apart should run
`vbars run` with arguments that parse.

| Command | Result |
| --- | --- |
| `lex FILE [--trivia] [--profile ID]` | tokens with positions, lexer diagnostics; exit 1 on errors |
| `parse FILE [--tree \| --json] [--profile ID]` | summary, tree or JSON, parser diagnostics; exit 1 on errors |
| `check FILE... [--profile ID] [--json]` | semantic analysis of the files (or a `vbaProject.bin`) as one project; exit 1 on errors |
| `fmt-check FILE [--json]` | losslessness of the syntax tree and whitespace findings; exit 1 on findings |
| `profiles [--json]` | the language profiles |
| `completions SHELL` | shell completion script (bash, zsh, fish, powershell, elvish) |
| `man` | the manual page in roff |
| `inspect FILE` | project/module listing (unchanged) |
| `run FILE... [--entry P] [--arg V]... [--grant CAP[=PATH]]... [--profile ID] [--memfs] [--max-steps N] [--max-memory B] [--max-call-depth N] [--timeout S] [--unlimited] [--system-env] [--allow-shell] [--command T]` | runs a procedure |

`run` is an explicit opt-in and denies everything by default: with no
`--grant` the program computes and prints (`Debug.Print` to standard output).
Capabilities are `file-read`, `file-write`, `process-spawn`, `network`,
`com-create`, `native-declare`, `ui-automation`; `=SCOPE` limits a grant, and
what a scope is depends on the capability: a path for the file capabilities (an
existing directory gets a trailing slash; matched by whole path segments), a
program for `process-spawn` (`notepad` also covers `notepad.exe`, never
`notepad-x.exe`), a ProgID or `Namespace.*` for `com-create`,
`library!procedure` or `library!*` for `native-declare`, an action
(`SendKeys`, `AppActivate`, `AppActivate:title`) for `ui-automation`, a host
for `network`. Malformed scopes are rejected. The program runs with finite
default limits (100000000 steps, 256 MiB, call depth 1000); `--max-steps`,
`--max-memory` and `--max-call-depth` change them and `--unlimited` removes
the step, memory and time limits. The run (and the shutdown after it) may also take at most 60
wall-clock seconds; `--timeout SECONDS` changes that and `--timeout 0` removes it. The file system is
the real one unless `--memfs`. Exit codes: 0 success (also `Stop`), 1 bad input
or the project does not compile, 2 unhandled run-time error (VBA or host),
3 permission denied, 4 resource limit or cancelled, 5 unsupported feature,
bridge failure or internal error, 6 the project is compiled-only or a call
needs a module without source (`Error::CompiledOnly`). `check` and `parse` of a
project with any module lacking source also exit 6 (not 1: the input is
well-formed but there is nothing to analyse); `run --allow-partial` accepts a
mixed project (never a fully compiled-only one). `check` and `run` accept `--strict-spec`, which
turns the spec-strict warnings (see `docs/language/spec-coverage.md`) into errors. `inspect` always succeeds on a
readable project and prints `Source availability:` plus, per module, whether
source is present, empty, compiled-only (with the cache size) or absent
without a cache, and a note for ambiguous cases.

## Tests

`tests/fileio.rs` (about 35 programs over the in-memory file system: text,
binary and random access, directory functions, error numbers),
`tests/permissions.rs` (deny by default, read-only, prefix scoping, traversal,
backslashes, symbolic links with the real file system, memory accounting),
`tests/session.rs` (trust gate, by-name invocation, isolation, loading, the
error taxonomy, `Shell`, environment, shutdown) and `tests/properties.rs`
(`Write`/`Input`, `Print`/`Line Input`, `Put`/`Get` round trips; arbitrary file
contents never panic; path normalization invariants).
