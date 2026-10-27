# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project intends to follow
[Semantic Versioning](https://semver.org/) once it has a stable release.

## Unreleased

### Added

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
