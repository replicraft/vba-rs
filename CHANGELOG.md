# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project intends to follow
[Semantic Versioning](https://semver.org/) once it has a stable release.

## Unreleased

### Stability of the 0.x series

vba-rs is at `0.1.0` and is distributed as a source dependency pinned to a revision (every package
has `publish = false`). Until `1.0`:

* The `vba` facade (`vba::*`, `vba::automation`, `vba::runtime`, `vba::prelude`, `vba::thread`)
  is the supported surface. A breaking change of it bumps the **minor** version and is listed
  here under "Breaking changes", in the entry that makes it. Patch versions add and fix without
  breaking it.
* `vba::raw::*` and the individual `vba-*` crates carry no promise of their own: they change
  with the release they are pinned to.
* Result, error and description types are `#[non_exhaustive]` (`Error`, `RunStatus`, `LimitKind`,
  `Capability`, `ModuleKind`, `ResourceLimits`, `ErrInfo`, `MemberInfo`, `ParamInfo`, ...): match
  enums with a wildcard arm and build structs with constructors, so that adding a variant or a
  field is not a break.
* Behavior marked **(unverified)** in the documentation was not compared with a real host and may
  change in any release; the measured state of the language is `docs/tooling/conformance.md`.
* The deny-by-default security posture (no ambient authority, `trust()` before any run, finite
  default limits) is not relaxed in 0.x; defaults may become stricter, which is listed here.

### Breaking changes

All breaking changes of the facade made so far in the `0.x` series; each is also described in the
entry that made it:

* `Error::Io` holds a `std::io::Error` (was `String`), so `source()` leads to the cause. Match
  `Error::Io(e)` and use `e.kind()`; `to_string()` is unchanged.
* `ResourceLimits`, `ScopeLimits`, `ErrInfo`, `MemberInfo`, `ParamInfo`, `ModuleKind`, `VfsError`
  and `LowerError` are `#[non_exhaustive]`: use their constructors and `with_*` methods, and a
  wildcard arm when matching.
* `Instance::from_session` applies the 60-second `UNTRUSTED_CALL_TIMEOUT` per call
  (`default_call_timeout(None)` opts out).
* `Engine::new()` starts from `ResourceLimits::untrusted()` (100 million steps, 256 MiB, call depth
  1000) and a 60-second allowance per call; code that relied on no limits calls `.unlimited()`.
* `vbars run` exits 7 (was 2) for an unhandled VBA run-time error.
* `Policy::grant_prefixes` scopes are matched per capability (whole path segments, exact ProgID or
  `Namespace.*`, `library!procedure`, ...); a scope of another form matches nothing.

### Fixed

- `ByRef` write-back from a late-bound host member is converted to the declared type of the
  caller's variable like an assignment (error 13 or 6, trappable); a `Long` variable can no longer
  hold a `String`. Host-model parameters convert an assigned value to their declared type. Programs
  that make no such call keep their step counts (the step digest of the conformance corpus is
  unchanged).
- The `Error` of a failing nested run (a form event raised by a host member) lists the frames of
  the nested run first, then a host-boundary marker (`StackFrame::host_boundary`), then the outer
  frames; `procedure`, `module` and `line` describe the innermost frame.
- `vba-sema`: a member of a fully described library type that is called with more arguments than it
  declares, and whose result type is described and has no default member, is a compile error
  ("wrong number of arguments"); before, the surplus went to a default member that did not exist
  and nothing reported it.

### Added

- `vba-conformance` (`tooling/vba-conformance`): the conformance corpus runner as a reusable crate
  with a pluggable engine (`run_corpus(dir, factory, &Options)`), a `REQUIRES` program header and an
  implied `grant:<capability>` requirement so a restricted host skips instead of failing. See
  `docs/tooling/conformance.md`.
- `Instance::with_output` (borrows the print buffer; `output` still copies),
  `InstanceThread::close_and_cancel`, `Project.Module.Proc` names in `run`/`call` (the project name
  is ignored), and `runtime::LoadError` as the `source()` of `runtime::Error::Load`.
- `vba-host-model` and `vba::host_model`: describe an embedding application's object model once
  (`HostLibrary`, `HostClass`, `HostMember`, `HostParam`, `HostEnum`, `HostConst`, `HostGlobal`;
  validated by `check()` with typed `ModelError`s) and derive from it the sema `TypeProvider`, the
  `Dispatch` objects (`HostObject`, `Handlers`, `HostImpl`), global functions, `Engine::host_library`,
  the language-server data and a stable JSON form (`to_json_string`, `from_json_str`). `vba-lsp`
  reads it through `vba.hostLibraries`. See `docs/architecture/host-model.md`.
- `vba-sema`: `TypeProvider::catalog` and `documentation` (default: nothing), `Layers`; `Let`
  through the default member of a library type (`rs.Fields("a") = 5`).
- `vba-host`: `Host::call_function`, the hook for global functions an embedding application provides.
- `vba-lsp-core`: `CompletionItem::documentation`; completion, hover and signature help for the names
  and documentation of described libraries.

- `vba`: `thread::InstanceThread`, an `Instance` on a dedicated thread driven through a command
  channel (`spawn`, `call`, `submit` and `Reply`, `cancel_token`, `close`; `std` only, commands run
  in submission order, a panicking command stops the thread), with `ThreadError`. This is the pattern
  for an application with a multi-threaded runtime, documented in the embedding guide.
- `vba`: `VERSION`, `Project::open_with` (size-limited reading), `Debug` for `Engine`, `Instance`
  and `HostContext`, `#[must_use]` on builders and queries, `source()` for `Error::Format`,
  `Error::Model` and `Error::Io`.
- `docs/architecture/adapter-checklist.md` and the tested example `adapter_skeleton`: an ordered
  checklist and a skeleton for the adapter between an embedding application and vba-rs.
- Packages carry `publish = false`, `readme`, `keywords` and `categories`; `dependencies.md` gains the
  measured dependency graph of an embedding application (38 crates by default, no duplicate versions,
  no C compiler unless `native-call`) and a licence audit.
- Language and semantic analysis: declarations after procedures, forward references to enums,
  `Global Enum` in classes, ordinal aliases, arrays declared `As New` instantiating their elements on
  use, `ByRef` `Variant` parameters writing back to variables of other types, trailing `Static` on
  procedures, Unicode line terminators and space characters in UTF-8 sources, and further spec-strict
  rules (`WithEvents`, event handlers, function result types, optional parameters, module cycles,
  public type name uniqueness; warnings unless `Engine::strict_spec(true)`).
- Run time: `Error` without argument and `Err.Raise` with an uncleared `Err`, the `MidB` statement,
  `LSet` between user-defined types, `Get`/`Put` of dynamic arrays, `Len` of a user-defined type with a
  dynamic array member, `For Each` over a class with an enumerator member, calls through a library
  interface type, assignment to an element of an array nested in an array, error 9 for indexing a
  scalar Variant, error 10 for `ReDim`/`Erase` of an array locked by a `ByRef` element.
- Standard library: `StrConv` with `vbLinguisticCasing`, `DateValue`/`IsDate` for dates without a
  year, `DDB` with a fractional period, `Like` ranges in text mode with accented letters.
- Compiled code (p-code): the verified layouts of VBA 6 and 7, 32 and 64 bits are lowered and checked
  against the platform kind, and more statements lower to IR (`On Error`, `Resume`, `GoTo`, optional
  parameters, `End`, Date and hexadecimal literals, class-like modules). The compiled cache is read for
  analysis only; it is never executed.
- `vba`: `Engine::constant`/`constants` (host constants usable in `Const`, `Case` and array
  bounds), `Engine::ui` (`MsgBox`/`InputBox` hooks) and the re-exports `UiHooks`,
  `MsgBoxRequest`, `InputBoxRequest`; `MemObject::reentrant`.
- The VBA library enumeration types (`VbMsgBoxResult`, `VbVarType`, `VbCompareMethod`, ...) are
  valid in `As` clauses.
- A generic stand-in host for a database-style embedding application
  (`crates/api/vba-rs/tests/db_host/`) and the "Known limits" and section 13 of the embedding
  guide.

- `vba`: editor services for an embedding application: `attach_debugger` (breakpoints by module and line, step into/over/out, `DebugContext` with frames, locals, module variables and `Me`), `Instance::eval` (Immediate window, also in a paused frame), `apply_edits` (hot edit with a documented state policy), `callable_procedures`, `Engine::debug_sink`. All opt-in; step accounting is unchanged when unused.
- `vba-host`: `Budget::suspend_deadline` stops the wall-clock deadline clock while a host waits for a person.
- `vba-sema`: `Options::friend`.
- `vba-ir`: `Stop` is an instruction (`InstKind::Stop`) so a debugger can continue after it.
- `vbars`: every file argument accepts `-` for standard input.
- `vbars`: `--json` for `check`, `parse`, `fmt-check` and `profiles` (schema version 1).
- `vbars`: `--color auto|always|never`; colors are used only on a terminal and never when
  `NO_COLOR` is set.
- `vbars completions SHELL` and `vbars man` generate shell completions and the manual page.
- `vbars --help` documents the exit codes.
- `vba`: `Engine::call_timeout` and a 60-second wall-clock allowance per call by default
  (`Engine::unlimited()` removes it); `vbars run --timeout SECONDS` with the same default.
- Root `README.md`, `CONTRIBUTING.md`, `SECURITY.md` and this changelog.
- File I/O: `Lock`/`Unlock` ranges, `Open ... Lock` clauses and shared contents between file
  numbers of one file, `Write #` with `Spc`, `Tab` and a trailing separator, `Width` breaking
  of long `Print` items, `Get`/`Put` of `Decimal` and error Variants, static checks of the
  operands of `Input`, `Line Input`, `Get` and `Put`.

- `AddressOf` yields a session-bound callback token (never a native address) that the emulated
  `Declare` shims `EnumWindows`, `EnumChildWindows`, `SetTimer`/`KillTimer` (delivered at
  `DoEvents`) and `qsort` call back into VBA, under the step and native-stack budgets and the
  `Declare` grant. `vba-sema` rejects an `AddressOf` operand that is not a `Sub` or `Function` of
  a standard module.

- `ResourceLimits` has size budgets for the work before a run: `max_source_bytes`, `max_modules`,
  `max_analysis_nodes` and `max_ir_instructions`, set by `ResourceLimits::untrusted()`. A project
  over a budget is refused by the session builder with `Error::Limit(LimitKind::SourceSize |
  ModuleCount | AnalysisSize | ProgramSize)` (exit code 4 in `vbars run`) before anything runs.
  Lower-level entry points: `vba_sema::AnalysisLimits` (`Options::limits`, `Analysis::limit_exceeded`,
  diagnostic `SEM098`), `vba_ir::LowerLimits` and `lower_with_limits`.
- `vba-lsp-core`: `FormatLimits` and `View::try_format` / `try_format_range` refuse oversized
  documents and absurd indent widths with `FormatError`.
- `Budget::time_left()` for host members that block, and `Instance::call_timeout()`.
- `vbars`: exit code 7 for an unhandled run-time error of `run` (see Changed).

### Changed

- `IsNull`, `IsEmpty`, `IsNumeric`, `IsDate` and `VarType` read the default member of an object
  argument when it has one; `DispError::raise(0, ..)` is raised as error 5.
- Breaking: `ResourceLimits`, `ScopeLimits`, `ErrInfo`, `MemberInfo`, `ParamInfo`, `ModuleKind`,
  `VfsError` and `LowerError` are `#[non_exhaustive]`, so fields and variants can be added without
  another break. Build limits with `ResourceLimits::default()` / `unlimited()` / `untrusted()` plus
  the `with_*` methods, scope limits with `ScopeLimits::new()` plus `with_*`, and the Automation
  descriptions with the constructors and `with_*` methods of `ErrInfo`, `ParamInfo` and `MemberInfo`.
  `match` on `ModuleKind` and `VfsError` needs a wildcard arm.
- Breaking: `Instance::from_session` now has the 60-second `UNTRUSTED_CALL_TIMEOUT` allowance per
  call, like `Engine::new()`: a hand-built session may sit on a host without any limit, and an
  instance should not inherit "no deadline" by accident. Use
  `default_call_timeout(None)` to opt out.
- Breaking: `vbars run` exits 7 (was 2) for an unhandled VBA run-time error, so that 2 means only a
  command-line usage error. The exit codes are one table for every command, in `vbars --help`, the
  manual page, the README and `docs/runtime/runtime.md`. 4 (limit, timeout, cancelled) is unchanged.
- A host member that blocks cannot be interrupted, but the deadline and the cancel token are now
  checked as soon as it returns, before its results are stored and before the next statement.

### Fixed

- A call timeout too large to be an `Instant` (for example `Duration::MAX` in `CallLimits::timeout`)
  is no deadline instead of a panic. The facade is exercised against malformed project bytes and
  streams, absurd names, arguments and limits, forged object handles and calls after a fatal error
  (`tests/no_panic.rs`): all return errors.
- `vba`: the output of a call is cut from the shared buffer without cloning it twice per call and
  without slicing at a stale offset.
- Rustdoc builds without warnings for the whole workspace.
- Performance, with unchanged results: Collection keys are indexed, class members are found by module
  and name, frames are reused, plain-value operators and loops run in place, number and date formats
  avoid copies (numbers in `docs/runtime/vm.md`).
- `Like` with an invalid pattern raises only when matching reaches it; `Variant` comparisons with
  `Decimal`, `Currency` and numeric text follow the effective type; `Date` and `Single` overflow
  promote to `Double` when a `Variant` operand allows it; blanks between a sign and the digits of a
  numeric string are accepted; a `Byte` array converts to a fixed-length `String`.
- Unix: `stat`, `Dir`, `FileLen`, `FileDateTime`, `GetAttr` and directory listings of the real file
  system read through a verified directory handle, like `Open`, `Kill`, `MkDir` and `Name`, so a
  directory swapped for a link after the policy check cannot show metadata outside the grant.
- The formatter runs in linear time on files that are mostly comment lines or full of keywords
  (it was quadratic).

- File I/O: `Len` outside 1 to 32767 in `Open`, a file number that does not convert to
  Integer, `SetAttr` on an open file and an unmarked `Date` read by `Input #` raise the
  documented errors.
- Cancellation and the time allowance of a call are honoured inside long library calls and
  after large charges, not only every 1024 instructions or at `DoEvents`.

### Notes

- Initial development version 0.1.0: VBA lexer, parser, semantic analysis, interpreter,
  standard library, `vbaProject.bin` reader and writer, host contract with deny-by-default
  capabilities, the `vba` facade, the `vbars` tool and a language server. See
  [docs/tooling/conformance.md](docs/tooling/conformance.md) for what is measured.
