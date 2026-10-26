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

### Fixed

- Cancellation and the time allowance of a call are honoured inside long library calls and
  after large charges, not only every 1024 instructions or at `DoEvents`.

### Notes

- Initial development version 0.1.0: VBA lexer, parser, semantic analysis, interpreter,
  standard library, `vbaProject.bin` reader and writer, host contract with deny-by-default
  capabilities, the `vba` facade, the `vbars` tool and a language server. See
  [docs/tooling/conformance.md](docs/tooling/conformance.md) for what is measured.
