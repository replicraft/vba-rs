# Adapter checklist

An ordered list for an embedding application that writes its adapter: the one module of
the application that knows about vba-rs and hides it from the rest. Do the steps in
order; each says what to decide, what the facade gives you and how to check it. The
runnable skeleton that follows the same numbering, compiled and tested with the crate, is
`crates/api/vba-rs/examples/adapter_skeleton.rs` (`cargo run -p vba-rs --example
adapter_skeleton`); the details of every API are in the [embedding guide](embedding-guide.md)
and the [public API](public-api.md).

The rule behind the list: **everything that crosses the seam is plain data you own** (stream
paths and bytes in, values and your own error type out). No `vba` type appears in the
signatures of the rest of the application, so a breaking change of the `0.x` series costs one
module.

## 1. Load the project from the application's storage

* [ ] Read the project as `(stream path, bytes)` rows from your table
  (`VBA/dir`, `VBA/_VBA_PROJECT`, `VBA/<module>`, `PROJECT`, ...) into a `ProjectStreams` and
  call `Project::from_streams(&streams, &ReadLimits)`. No container is opened by vba-rs.
* [ ] Choose `ReadLimits` for the sizes you accept (the default is for hostile input:
  256 MiB container, 1024:1 decompression ratio). Reading never executes anything.
* [ ] If you hold a file instead, `Project::open_with` (size-limited) or `from_bytes_with`.
* [ ] Decide what a project without source (compiled only) means for you: an
  `Engine` refuses it with `kind() == "compiled-only"`; there is no fallback to the compiled
  code.
* [ ] Treat every read error as an error of the stored data, not as a bug: `Error::Format`,
  never a panic (pinned by `tests/no_panic.rs`).

## 2. Choose the security defaults

* [ ] Start from `Engine::new()` and **add** grants; do not start from a permissive policy.
  The defaults: deny-all `Policy` (files, `Shell`, `CreateObject`, `Declare`, network, UI
  automation all fail as VBA error 70), in-memory file system, fixed clock, empty environment,
  `ResourceLimits::untrusted()` (100 million steps, 256 MiB, call depth 1000) and a 60-second
  wall-clock allowance per call.
* [ ] For each capability the code needs, grant it with the narrowest scope
  (`grant_prefixes(Capability::FileRead, ["/data/exports"])`, `ComCreate` for `"Demo.*"` or an
  exact ProgID); scope forms are per capability, see the [security model](security-model.md).
* [ ] Leave `real_files()`, `system_environment()` and the `native-call`, `winapi` and `wine`
  features off unless a requirement names them. None of the features grants authority; each
  only makes code available.
* [ ] Write the security defaults into one `Settings` value the application can show and log.
* [ ] Decide `strict_spec` (default off: spec-only rules are warnings) and the language
  `Profile` (`Engine::profile`) the code is written for.

## 3. Analyse before you run

* [ ] `Engine::analyze(&project)` returns the diagnostics and runs nothing. Refuse (or show
  to the author) a project with `has_errors()`; `rendered()` is plain text per diagnostic.
* [ ] Use the **same** `Engine` configuration (host globals, `host_members`, module scopes) for analysis and
  loading, or names you provide will be reported as undeclared.
* [ ] Decide whether warnings block anything (`Instance::warnings` after load).

## 4. Describe the host objects

* [ ] One `Dispatch` implementation per object the code can reach (`Application`, `DoCmd`,
  a recordset, a form's `Me`). Expose a global with `dispatch_global`, a creatable class with
  `dispatch_class` (it additionally needs the `ComCreate` grant), a form's controls with
  `host_members` and the host base (`create_object(module, Some(me))`).
* [ ] Every member has a `MemberInfo` with real `ParamInfo` (by value or by reference,
  optional, default); the interpreter binds and converts through them.
* [ ] Return `DispError` values with VBA error numbers the code can trap; do not panic in
  `invoke`. A host member that blocks cannot be interrupted: bound it yourself.
* [ ] A member that must call back into VBA while it runs returns `true` from
  `Dispatch::reentrant` and uses `HostContext::current(table)` (depth limit
  `VmConfig::max_reentrancy_depth`, default 4).
* [ ] Transactions, storage and permissions of the application are the host objects' business:
  the VM never touches them ([guide, section 11](embedding-guide.md#11-transactions-belong-to-the-host)).

## 5. Decide the threading model

* [ ] `Instance`, `Engine`, `Session` and host objects are not `Send`. If your runtime is
  multi-threaded, give each instance a dedicated thread: `vba::thread::InstanceThread` (std
  only; commands run in order; `call` blocks, `submit` returns a `Reply`).
* [ ] Build the `Engine`, the `Project` and the host objects **inside** the spawn closure.
  State shared with the rest of the application goes behind `Arc<Mutex<..>>` or a channel
  to your own service; the `Dispatch` wrappers stay on the interpreter thread.
* [ ] From async code, call `InstanceThread::call` where blocking is allowed (blocking pool)
  or `submit` and wait elsewhere; vba-rs has no runtime and no background thread.
* [ ] One instance per document/session: they share nothing. Decide the pool, the idle
  timeout and what happens to a queued command when its owner goes away (closing the handle
  drops queued commands; the running one finishes).
* [ ] Decide what a panic in your own command closure means: it stops the interpreter thread
  (`ThreadError::Panicked`); recreate the instance.

## 6. Trust

* [ ] A loaded instance refuses to run anything until `trust()`, not even
  `Class_Initialize`. Call it only after steps 2 and 3, as an explicit decision (signed
  project, an owner who opted in, an allow-list...). `Engine::run` is load + trust + run +
  shutdown in one call, which is that decision made implicitly.
* [ ] Log which project (hash, name) was trusted, by whom, with which `Settings`.

## 7. Limits and timeouts per call

* [ ] Set the engine-wide ceiling (`limits`, `max_steps`, `max_memory`, `call_timeout`,
  `vm_config` for string length, array size and nesting). Never use `unlimited()` for code
  you did not write.
* [ ] Bound **each** call with `Instance::with_call_limits(CallLimits::new().max_steps(..)
  .max_memory(..).timeout(..), |i| ..)`: a scope narrows the ceiling, never widens it, and
  does not spend the instance's own step counter.
* [ ] Limits are cooperative: checked every 1024 steps, at `DoEvents` and inside long
  library calls. One huge interpreter operation or a blocking host member is not interrupted:
  the memory and size limits are the guard for the first, your own deadline for the second.
* [ ] After a fatal fault (limit, cancellation, `End`) the frames of the run are discarded
  without running code. Check `is_usable()` and call `reset()` when module state matters;
  `reset_step_counter()` gives the next handler a fresh allowance.

## 8. Cancellation

* [ ] Keep a `CancelToken` (`Instance::cancel_token`, `InstanceThread::cancel_token`). It is
  `Send + Sync`; `cancel()` from any thread stops the running call at its next poll, with
  `Error::Run(Cancelled)` that `On Error` cannot swallow.
* [ ] The token stays set until `reset()`: reset it before the next call, and remember
  engines (and instances) created from one engine share one token unless you give each its own.
* [ ] A call that exceeds its time allowance also ends as `Cancelled`; tell the two apart by
  whether you cancelled.

## 9. Map errors to your own type

* [ ] One function `vba::Error -> YourError` at the seam. `Error` is `#[non_exhaustive]`: use
  `kind()` (stable short names, table in [public-api.md](public-api.md)) and keep a catch-all.
  `error_info()` gives number, description, source, help, module, procedure and line for
  `vba` and `host` errors; `report()` has the call stack; `detail()` of a `runtime::Error`
  renders both.
* [ ] Decide per kind what the user sees: `load` (show diagnostics), `vba`/`host` (the
  author's bug), `limit`/`cancelled` (try again smaller), `permission-denied` (show which
  capability), `internal`/`poisoned` (your support channel, step 15).
* [ ] `Error` implements `std::error::Error` with `source()`; keep it as the source of your
  error for logs.
* [ ] The panic policy: the facade returns errors for malformed projects, absurd arguments and
  misuse. A panic from it is a bug to report.

## 10. Events

* [ ] Find handlers with `Instance::modules()` / `modules_defining(name)` / `has_procedure`.
* [ ] Raise an event on one instance with `raise_event_on(object, "Form", "BeforeUpdate",
  vec![Arg::by_ref(Variant::Integer(0))])` and read the final `ByRef` values from
  `EventOutcome::byref` (`Cancel`). Events raised by host objects outside a call are delivered
  with `deliver_pending_events`.
* [ ] Decide the order and the transaction boundary around the handler (guide, section 11).
* [ ] Give every handler its own budget (`with_call_limits` or `reset_step_counter`).

## 11. Lifecycle of an instance

* [ ] Release what you own: an object value returned by a call (`CallResult::value`, a
  `ByRef` slot) carries a reference (`Instance::release`); `create_object` is paired with
  `destroy_object` (which runs `Class_Terminate`).
* [ ] `shutdown()` before dropping, when the code's terminators matter; dropping alone does
  not run them.
* [ ] `reset()` returns the instance to its initial state (and invalidates every object
  handle); decide when your application resets (after a fatal fault, between users).

## 12. State shared with host objects

* [ ] List what the host objects read and write outside the interpreter thread and give each
  a lock or a message. Nothing may block the interpreter thread for long.
* [ ] Output: `Debug.Print` text comes back per call in `Outcome::output`; decide where it is
  shown or stored (it counts against the memory limit).

## 13. Persistence and the compiled cache

* [ ] The interpreter always runs the **source**; the compiled p-code cache in a
  `vbaProject.bin` (`__SRP_n` streams, `_VBA_PROJECT` cache bytes) is never executed.
* [ ] After an edit (`Project::set_module_source`, `add_module`...) write back with
  `to_streams(&WriteOptions::default())`: store each stream of `WriteReport::changed_streams`
  that is in the returned set, **delete** each changed path that is not (an invalidated
  cache), leave every other row alone. An unedited project returns byte-identical streams and
  an empty list. The default `StaleCache::Invalidate` is the safe choice; `Keep` keeps caches
  that may no longer match the source.
* [ ] Write in one transaction of your own storage: a half-written set of streams is a broken
  project.
* [ ] Unsupported edits (adding or renaming modules of a project read from a file,
  compiled-only modules) return `Error::Format(UnsupportedEdit)`; handle it as a refusal.
* [ ] Never persist an `Instance`: there is no snapshot. Persist the project and the data your
  host objects own, and rebuild the instance.

## 14. Upgrade policy and pinning

* [ ] Depend on a **git revision** (or a path), never a branch: `vba-rs = { git = "...",
  rev = "<commit>" }`. The packages are `publish = false`.
* [ ] Commit your `Cargo.lock`; the lock file of the vba-rs repository is not used by
  dependents. Run `cargo tree -d` and your license/advisory checks on your lock.
* [ ] The `0.x` promise: a breaking change of the facade bumps the minor version and is listed
  in `CHANGELOG.md`; `vba::raw::*` carries the promises of its own crates only. Enums are
  `#[non_exhaustive]` (wildcard arm) and structs are built with constructors.
* [ ] Log `vba::VERSION` and the revision at startup; assert it in a test so an accidental
  bump fails your build rather than production.
* [ ] Upgrade as a change of its own: read the changelog, build, run your adapter tests and
  the checks of step 15, and look for behavior changes in the conformance report
  (`docs/tooling/conformance.md`) for the constructs your users' code uses.
* [ ] Behavior marked **(unverified)** in the docs was not compared with a real host and may
  change.

## 15. Test the adapter, run the conformance programs, report bugs

* [ ] Adapter tests: your own projects through the full path (streams in, analysis, load,
  trust, calls with limits, events, errors mapped, write-back). Use the skeleton's test as a
  template.
* [ ] A host contract check without your application: `ScriptedHost` and `HeadlessHost`
  (`vba::Host` implementations) and `vba::automation::testing::MemObject` stand in for the
  application's objects.
* [ ] **Conformance against your configuration.** The suite is a set of plain programs in
  `tests/vba/<area>/*.bas` with leading comment directives (`EXPECT`, `ENTRY`, `ERROR`,
  `GRANT`, `MAX_STEPS`, `CLOCK`; format at the top of
  `crates/api/vba-rs/tests/conformance.rs`). The harness in the repository builds its own
  `Engine::new()`; it has no hook for a foreign host. To check the language under *your*
  settings (profile, limits, grants, host globals present), write a loop like
  `run_program` in that file in your adapter's tests: for each program, add its modules to a
  `Project`, build your adapter's `Engine`, `load`, `trust`, `run(entry)`, then compare
  `Instance::output()` with the `EXPECT` lines and the error kind with `ERROR`. Programs
  with `GRANT` or `CLOCK` lines need those settings, so run the others and skip those. A
  program that fails only under your configuration shows a limit, grant or name that your
  settings changed.
* [ ] **Reporting a bug.** Reduce it to the smallest `.bas` module that shows it and say
  what real VBA would do. Check it with the CLI, which has the same engine and the
  deny-all defaults:
  `vbars run --entry Main repro.bas` (add `--grant`, `--max-steps`, `--timeout` as your
  adapter does; exit codes are in `vbars run --help`). Put the module, the command, the
  commit (`vba::VERSION` and the revision), the expected and the actual output (or the
  error with its call stack from `runtime::Error::detail()`) in the report. For a
  host-object problem, include the smallest `Dispatch` implementation and the call sequence
  as a test of the facade. An escape from a grant, a limit that did not hold or a panic on
  hostile input is a security report: use the private channel in `SECURITY.md`, not a public
  issue.
