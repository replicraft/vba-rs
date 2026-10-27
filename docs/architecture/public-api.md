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
call; calling it *is* the trust decision. Applications that host forms (instances
with their own state, a host object as `Me`, events with `ByRef` results such as
`Cancel`) use the richer `Instance` API; the whole flow is in the
[embedding guide](embedding-guide.md). Compiled doctests in the crate root
cover: running a standard-module function, embedding a custom host object,
denied permissions, resource limits and cancellation, and reading and editing a
project with preservation.

### Surface

| Item | What it is |
| --- | --- |
| `Project` | modules plus the metadata and preserved bytes of a `vbaProject.bin`; `from_source`, `from_modules`, `from_bytes`, `from_streams`, `open`/`open_with` (size-limited), `add_module`, `add_source_file`, `set_module_source`, `to_bytes`, `to_streams`, `version`, `analyze`; `model()` exposes `vba_project::VbaProject` |
| `Report` | the diagnostics of an analysis (`has_errors`, `errors`, `warnings`, `codes`, `rendered`) |
| `Engine` | builder: `profile`, `policy`/`grant`/`grant_prefixes`, `limits`/`max_steps`/`max_memory`/`unlimited`, `vm_config`, `class`/`dispatch_class`, `global`/`dispatch_global`, `host_members`, `with_file`, `real_files`, `system_environment`, `command_line`, `cancel_token`, `session_builder`; then `analyze`, `load`, `load_with`, `run` |
| `Instance` | one loaded session with its `Debug.Print` buffer. Running: `trust`, `run`, `call`, `raise_event`, `shutdown`. Instances: `create_object`, `default_instance`, `destroy_object`, `call_on`, `has_procedure`, `raise_event_on`, `get_member`, `set_member`, `release`, `deliver_pending_events`. State: `get_global`, `set_global`, `modules`, `list_public_members`, `modules_defining`, `last_error`. Lifecycle: `is_usable`, `reset_step_counter`, `reset`, `with_call_limits`, `cancel_token`; plus `output`, `session()`, `from_session` |
| `Arg`, `CallTarget`, `CallResult` | arguments (`val`, `by_ref`, `missing`, `named`), the target of a call (`Project` or `Object`) and its result `{ value, status, byref, output }` |
| `CallLimits` | `#[non_exhaustive]` per-scope allowances (`max_steps`, `max_memory`, `timeout`) for `Instance::with_call_limits` |
| `EventOutcome` | `{ handled, status, byref, output }` of `raise_event_on` |
| `HostContext` | the re-entrant handle a host member uses to call back into the running session (`HostContext::current(table)`) |
| `Outcome` | `value`, `status` (`Completed`, `Ended`, `Stopped`) and the text printed by that call |
| `ModuleSummary`, `ProcedureSummary`, `ParamSummary`, `VariableSummary`, `ProcKind` | project metadata for hosts that discover event handlers |
| `ErrorReport`, `VbaErrorInfo`, `StackFrame` | the details of an unhandled or handled VBA error (number, description, source, help, `LastDllError`, module, procedure, line, call stack) |
| `CancelToken` | `Send + Sync + Clone`; `cancel`, `reset`, `is_cancelled` |
| `Error` | `#[non_exhaustive]`, `Send + Sync`: `Format`, `Model`, `Source`, `Io`, `Run(vba::runtime::Error)`, `Poisoned`; implements `std::error::Error` with `source()` for `Format`, `Model` and `Io`; `kind()` gives a stable short name, `report()` and `error_info()` the VBA error details |
| Re-exports | profiles (`Profile`, `default_profile`, `find_profile`), values (`Variant`, `Currency`, `Date`, `Decimal`, `VbaString`, `VbaArray`), host contract (`Host`, `Policy`, `Capability`, `ResourceLimits`, `HeadlessHost`, `ScriptedHost`), runtime (`Session`, `SessionBuilder`, `MemFs`, `RealFs`, `SystemHost`, `ShellHook`), project/format (`VbaProject`, `Module`, `ModuleKind`, `Reference`, `SysKind`, `ProjectVersion`, `ReferenceHint`, `ProjectStreams`, `ReadMode`, `ReadLimits`, `WriteOptions`, `WriteReport`, `StaleCache`), diagnostics (`Diagnostic`, `Severity`) |
| `vba::automation` | the traits an embedder implements to expose host objects: `Dispatch`, `DispId`, `DispParams`, `InvokeFlags`, `MemberInfo`, `ParamInfo`, `ObjectTable`, `DispError`, `ErrInfo`, plus `testing::MemObject` |
| `vba::runtime` | the whole `vba-runtime` crate, for its `Error` taxonomy and file-system types |
| `vba::thread` | `InstanceThread` (an `Instance` on a dedicated thread behind a command channel: `spawn`, `call`, `submit`/`Reply`, `cancel_token`, `close`) and `ThreadError` |
| `vba::prelude` | the handful of names most embedders need |
| `vba::raw::*` | `ast`, `automation`, `core`, `diagnostics`, `ffi`, `format`, `host`, `ir`, `lexer`, `profile`, `project`, `runtime`, `sema`, `source`, `stdlib`, `syntax`, `value`, `vm`, and with features `winapi`, `wine` |

Host objects are `Dispatch` implementations registered by ProgID
(`Engine::dispatch_class("Demo.Greeter", || Greeter)`) or by global name
(`dispatch_global`; the names are made known to the analysis, so `Option Explicit`
code can mention them). `CreateObject` additionally needs `Capability::ComCreate`.
A host object that must call back into VBA while it runs (open a form and run
its handlers, raise `BeforeUpdate` and read `Cancel`) returns `true` from
`Dispatch::reentrant` and uses `HostContext`; the semantics are in the
[embedding guide](embedding-guide.md#6-re-entrancy-a-host-member-that-calls-back).

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

**Threading.** A session is single-threaded: `Instance`, `Session`, `HostContext`
and the host objects are built on `Rc`/`RefCell` and are not `Send`, and neither is
`Engine` (it holds host-object factories). Create each session on the thread that
runs it. Independent sessions share nothing (module variables, default instances,
`Err`, the random generator and open files are per session;
`sessions_are_isolated` in `crates/api/vba-rs/tests/facade.rs`). `Error`,
`Project`, `Report`, `Diagnostic`, `CallLimits`, `Policy`, `ResourceLimits` and
`CancelToken` are `Send + Sync`; `Variant`, `Arg`, `Outcome`, `CallResult` and
`EventOutcome` are `Send` (all pinned by `tests/send_sync.rs`). The cross-thread
handle of a running session is `CancelToken`. For an application with a
multi-threaded runtime, `vba::thread::InstanceThread` runs one instance on a
dedicated thread of its own and takes commands over a channel (`std` only, commands
run in submission order); the pattern, the exact table and the rules are in the
[embedding guide](embedding-guide.md#9-threading).

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
depth; `VmConfig` bounds string length, array size, native nesting and host
re-entrancy depth (`max_reentrancy_depth`, default 4). Exceeding one ends the
run with `runtime::Error::Limit`, which `On Error` cannot swallow. The meter
belongs to the `Instance` and spans all its calls, nested ones included.
Cancelling through `CancelToken` is cooperative: the session polls it before
every call, at `DoEvents` and every 1024 steps, so a loop that never yields is
stopped too; a single long interpreter operation or a blocking host member is
not interrupted. After a fatal fault `Instance::is_usable` says whether another
call can start, `reset_step_counter` gives a new step allowance and `reset`
starts from scratch (see the [embedding guide](embedding-guide.md#8-limits-cancellation-and-the-state-after-a-fault)).

**Error taxonomy.** `Error::kind()` (and `runtime::Error::kind()`) return stable
names:

| Kind | Responsible party | Trappable by `On Error` |
| --- | --- | --- |
| `load` | the project does not compile (rendered diagnostics) | n/a |
| `not-trusted`, `compiled-only`, `no-such-procedure`, `ambiguous-procedure`, `not-callable`, `no-such-member`, `ambiguous-member`, `not-accessible` | the embedder's request | n/a |
| `reentrancy` | a call-back from a host member that is stale or too deeply nested | n/a |
| `poisoned` | the instance hit an internal error and refuses calls until `reset` | n/a |
| `vba` | an unhandled error raised by the program or the language (number, description, source, help, module, procedure, line, call stack in the report) | yes while running |
| `host` | an unhandled error reported by a host hook or host object | yes |
| `permission-denied` | the policy refused a capability (error 70) | yes |
| `bridge` | the `Declare` bridge could not make a call | no |
| `limit` | steps, memory, call depth, string length or array size | no |
| `cancelled` | the host asked to stop | no |
| `unsupported` | a feature this implementation lacks (never silently succeeds) | no |
| `internal` | a broken invariant (a bug; please report) | no |
| `format`, `model`, `source`, `io` | project reading, editing and writing | n/a |

`Error`, `RunStatus`, `LimitKind` and `Capability` are `#[non_exhaustive]`; match with a wildcard arm.

**Preservation.** `Project::from_bytes` keeps every byte the model does not
interpret; `to_bytes` reproduces it. Supported edits are module source and
module/project metadata held in the `dir` stream; adding, removing or renaming
modules of a project read from a file returns a format error (see
`../formats/vba-format.md`).

## What the facade does not promise

It does not promise full VBA compatibility. How much is measured, and where it
is not, is in [conformance.md](../tooling/conformance.md).

## Changes to limits and grants

* `ResourceLimits::unlimited()` (what `Default` returns) and `ResourceLimits::untrusted()` (100
  million steps, 256 MiB, call depth 1000) are new. `Engine::new()` starts from the second;
  `Engine::unlimited()` and `Engine::max_memory(bytes)` are new. Code that relied on an engine
  without limits calls `.unlimited()`.
* `Engine::call_timeout(Option<Duration>)` and `Instance::default_call_timeout`: every call from
  the embedding application has a 60-second wall-clock allowance by default; `.unlimited()` or
  `call_timeout(None)` removes it.
* `Policy::grant_prefixes` keeps its name and signature, but a scope is matched per capability:
  whole path segments for files, the program for `ProcessSpawn`, an exact ProgID or `Namespace.*`
  for `ComCreate`, `library!procedure` or `library!*` for `NativeDeclare`, an action for
  `UiAutomation` (`AppActivate` is checked as `AppActivate:title`), a host for `Network`. A scope of
  any other form matches nothing; `vba_host::scope::validate_scope` reports it.
* `Host::poll_cancel` (default: nothing) is called every `POLL_INTERVAL` steps; hosts that cancel in
  `idle` should implement it as well. `vba_stdlib::Ctx` has a `work` meter and `charge`;
  `StdError::WorkExceeded` is new (the enum is not exhaustive for callers matching it).
* `Vfs::set_memory_meter` (default: nothing) lets an in-memory file system charge `max_memory_bytes`.
* `VbaString` and `VbaArray` register their allocation with the thread's current `MemoryMeter`
  (`vba_core::memory`); outside a run nothing is registered.
