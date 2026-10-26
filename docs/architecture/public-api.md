# Public API: the `vba` facade

`crates/api/vba-rs` is the stable entry point for embedders. The package is
`vba-rs`; the library is imported as `vba`. The other crates stay available
through `vba::raw::*` for needs the facade does not cover, but only the items
exported by the facade itself are the semver promise.

```toml
[dependencies]
vba-rs = { path = "crates/api/vba-rs" }   # features: winapi, native-call, wine (all off)
```

## The common flow

```rust
use vba::{Engine, Project};

let project = Project::from_source("Module1", "Public Sub Main()\n  Debug.Print \"hi\"\nEnd Sub\n");
let engine = Engine::new();                    // deny-all policy, no host objects, fixed clock
assert!(!engine.analyze(&project).has_errors()); // diagnostics, nothing runs
let mut instance = engine.load(&project)?;      // compiled, UNTRUSTED: no user code has run
instance.trust();                               // the embedder's explicit decision
let out = instance.run("Main", vec![])?;        // Outcome { value, status, output }
assert_eq!(out.output, "hi\n");
instance.shutdown()?;                           // Class_Terminate of module objects, files closed
# Ok::<(), vba::Error>(())
```

`Engine::run(&project, "Proc", args)` does load, trust, run and shutdown in one
call; calling it *is* the trust decision. Compiled doctests in the crate root
cover: running a standard-module function, embedding a custom host object,
denied permissions, resource limits and cancellation, and reading and editing a
project with preservation.

### Surface

| Item | What it is |
| --- | --- |
| `Project` | modules plus the metadata and preserved bytes of a `vbaProject.bin`; `from_source`, `from_bytes`, `from_streams`, `open`, `add_module`, `add_source_file`, `set_module_source`, `to_bytes`, `to_streams`, `version`, `analyze`; `model()` exposes `vba_project::VbaProject` |
| `Report` | the diagnostics of an analysis (`has_errors`, `errors`, `warnings`, `codes`, `rendered`) |
| `Engine` | builder: `profile`, `policy`/`grant`/`grant_prefixes`, `limits`/`max_steps`/`max_memory`/`unlimited`, `class`/`dispatch_class`, `global`/`dispatch_global`, `with_file`, `real_files`, `system_environment`, `command_line`, `cancel_token`, `session_builder`; then `analyze`, `load`, `run` |
| `Instance` | one loaded session with its `Debug.Print` buffer: `trust`, `run`, `raise_event`, `output`, `shutdown`, `session()` |
| `Outcome` | `value`, `status` (`Completed`, `Ended`, `Stopped`) and the text printed by that call |
| `CancelToken` | `Send + Sync + Clone`; `cancel`, `reset`, `is_cancelled` |
| `Error` | `#[non_exhaustive]`: `Format`, `Model`, `Source`, `Io`, `Run(vba::runtime::Error)`; `kind()` gives a stable short name |
| Re-exports | profiles (`Profile`, `default_profile`, `find_profile`), values (`Variant`, `Currency`, `Date`, `Decimal`, `VbaString`, `VbaArray`), host contract (`Host`, `Policy`, `Capability`, `ResourceLimits`, `HeadlessHost`, `ScriptedHost`), runtime (`Session`, `SessionBuilder`, `MemFs`, `RealFs`, `SystemHost`, `ShellHook`), project/format (`VbaProject`, `Module`, `ModuleKind`, `Reference`, `SysKind`, `ProjectVersion`, `ReferenceHint`, `ProjectStreams`, `ReadMode`, `ReadLimits`, `WriteOptions`, `WriteReport`, `StaleCache`), diagnostics (`Diagnostic`, `Severity`) |
| `vba::automation` | the traits an embedder implements to expose host objects: `Dispatch`, `DispId`, `DispParams`, `InvokeFlags`, `MemberInfo`, `ParamInfo`, `ObjectTable`, `DispError`, `ErrInfo`, plus `testing::MemObject` |
| `vba::runtime` | the whole `vba-runtime` crate, for its `Error` taxonomy and file-system types |
| `vba::prelude` | the handful of names most embedders need |
| `vba::raw::*` | `ast`, `automation`, `core`, `diagnostics`, `ffi`, `format`, `host`, `ir`, `lexer`, `profile`, `project`, `runtime`, `sema`, `source`, `stdlib`, `syntax`, `value`, `vm`, and with features `winapi`, `wine` |

Host objects are `Dispatch` implementations registered by ProgID
(`Engine::dispatch_class("Demo.Greeter", || Greeter)`) or by global name
(`dispatch_global`). `CreateObject` additionally needs `Capability::ComCreate`.

## Embedding with a container you read yourself

vba-rs does not open database containers. If your application reads a store
that holds VBA (an Access database keeps the VBA streams as rows of a system
table), extract the streams and pass them in:

```rust
use vba::{Project, ProjectStreams, ReadLimits, WriteOptions};

// `rows` is whatever your container reader yields: (stream path, bytes).
let mut streams = ProjectStreams::new();
for (path, bytes) in rows {
    streams.insert(path, bytes); // "VBA/dir", "VBA/_VBA_PROJECT", "VBA/Module1", "PROJECT", ...
}
if let Ok(mut project) = Project::from_streams(&streams, &ReadLimits::default()) {
    // project.version() gives facts; project.analyze(..) / Engine run the code.
    let _ = project.set_module_source("Module1", "Sub A()\r\nEnd Sub\r\n");
    if let Ok((out, report)) = project.to_streams(&WriteOptions::default()) {
        for path in &report.changed_streams {
            match out.get(path) {
                Some(bytes) => { let _ = bytes; /* write the row */ }
                None => { /* delete the row */ }
            }
        }
    }
}
```

What crosses the seam is only stream paths and bytes; storage CLSIDs, state
bits, timestamps and sector layout are neither needed nor produced. A
write-back stores each changed stream that is in the returned set, deletes each
changed path that is not (invalidated `VBA/__SRP_n` caches), and leaves all
other streams as they are: an unedited project returns byte-identical streams
and an empty `changed_streams`. The same edits and refusals apply as for a
`vbaProject.bin` (`Error::Format(UnsupportedEdit)`, including for compiled-only
modules). Details are in [vba-format.md](../formats/vba-format.md).

`Project::version()` returns `ProjectVersion`: the `_VBA_PROJECT` version word,
platform, code page, LCID, reference library facts and `compiled_only`.
`profile_hint()` is a hypothesis table with a single entry and returns `None`
for everything else; it names a profile, it does not select one. Pass the
profile you choose to `Engine::profile`.

## Features

All features are off by default and **none grants authority**: each only makes
code available, and every use still requires an explicit `Policy` grant.

| Feature | Enables | Decision |
| --- | --- | --- |
| `winapi` | `raw::winapi`, the Win32 behavior catalog and portable shims | Off. The crate is pure (an injected context, no ambient authority) so it could be on, but the default dependency graph and API surface stay minimal and the facade's main flow does not need it. The VM already routes `Declare` through the same providers when the host grants `NativeDeclare`. |
| `native-call` | the dynamic native call stage of `Declare` (`libloading` + `libffi`, the only `unsafe` in the workspace) | Off. Needs the `NativeDeclare` capability *and* a host-issued permission. |
| `wine` | `raw::wine`, the Wine bridge client (implies `winapi`) | Off. Starts nothing by itself; Wine is not a sandbox. |

## Embedder contract

**Threading.** A session is single-threaded: `Instance`, `Session` and the
host objects are built on `Rc`/`RefCell` and are not `Send`. Create each
session on the thread that runs it. Independent sessions share nothing (module
variables, default instances, `Err`, the random generator and open files are
per session; `sessions_are_isolated` in `crates/api/vba-rs/tests/facade.rs`).
The only cross-thread handle is `CancelToken`.

**No ambient authority.** A program can reach nothing the embedder did not hand
over. `Engine::new()` has a deny-all `Policy`: files (`FileRead`/`FileWrite`,
with whole-segment path scopes applied to the normalized, symlink-resolved
path; the other capabilities have their own scope forms, see
`security-model.md`), `Shell` (`ProcessSpawn`), `CreateObject`/`GetObject` (`ComCreate`),
`Declare` (`NativeDeclare`), network and UI automation are all refused with
VBA error 70. The default file system is in memory; `real_files()` and
`system_environment()` are explicit opt-ins, and the real clock and environment
are not read otherwise. Nothing runs at load time and a session refuses
`run`/`raise_event` until `trust()`, not even `Class_Initialize` or
`Workbook_Open`. Embedder-side reads such as `Project::open` are the
embedder's own file access, not the program's.

**Determinism.** With the default engine the clock is fixed (2000-01-01), the
environment is empty, the file system is in memory and the random generator
is seeded by the language rules, so the same project, arguments and
configuration produce the same output (`runs_are_deterministic`). Using
`system_environment()` removes that guarantee.

**Limits and cancellation.** `ResourceLimits` bound steps, memory and call
depth; `VmConfig` bounds string length and array size. Exceeding one ends the
run with `runtime::Error::Limit`, which `On Error` cannot swallow. Cancelling
through `CancelToken` is cooperative: the session checks it at `DoEvents`. A
loop that never calls `DoEvents` cannot be interrupted by a token; give such
programs a step budget.

**Error taxonomy.** `Error::kind()` (and `runtime::Error::kind()`) return stable
names:

| Kind | Responsible party | Trappable by `On Error` |
| --- | --- | --- |
| `load` | the project does not compile (rendered diagnostics) | n/a |
| `not-trusted`, `no-such-procedure`, `ambiguous-procedure`, `not-callable` | the embedder's request | n/a |
| `vba` | an unhandled error raised by the program or the language (number, procedure, line, stack in the report) | yes while running |
| `host` | an unhandled error reported by a host hook or host object | yes |
| `permission-denied` | the policy refused a capability (error 70) | yes |
| `bridge` | the `Declare` bridge could not make a call | no |
| `limit` | steps, memory, call depth, string length or array size | no |
| `cancelled` | the host asked to stop | no |
| `unsupported` | a feature this implementation lacks (never silently succeeds) | no |
| `internal` | a broken invariant (a bug; please report) | no |
| `format`, `model`, `source`, `io` | project reading, editing and writing | n/a |

`Error` is `#[non_exhaustive]`; match with a wildcard arm.

**Preservation.** `Project::from_bytes` keeps every byte the model does not
interpret; `to_bytes` reproduces it. Supported edits are module source and
module/project metadata held in the `dir` stream; adding, removing or renaming
modules of a project read from a file returns a format error (see
`../formats/vba-format.md`).

## What the facade does not promise

It does not promise full VBA compatibility. How much is measured, and where it
is not, is in [conformance.md](../tooling/conformance.md).
