# vba-rs

A cross-platform implementation of Visual Basic for Applications (VBA) in Rust, focused on
full compatibility. It reads VBA projects (`vbaProject.bin`, exported `.bas`/`.cls`/`.frm`
files), analyses and runs the code, and can be embedded in other applications through a
host contract. It does not depend on, or know about, any particular embedding application.

## Status

Early (version 0.1.0). The measured state is generated from the conformance suite and
published in [docs/tooling/conformance.md](docs/tooling/conformance.md):

| Measure | Count |
| --- | ---: |
| Inventory entries (statements, functions, operators, objects, spec sections) | 708 |
| implemented | 507 |
| partial | 161 |
| unsupported | 6 |
| not-started | 34 |
| Conformance programs | 2082 |
| Programs recorded as known deviations | 177 |

How to read this honestly:

* A status only says what the listed programs check. It is not a claim of full
  compatibility.
* No expected value has been confirmed by running real VBA yet. The programs' expectations
  come from the language specification (MS-VBAL), Microsoft's reference pages, derivation,
  or are the implementation's own behavior (`unverified`). A Windows harness for checking
  against real VBA exists but has never been run
  ([tests/oracle](tests/oracle/README.md)).
* Compiled-only projects (modules with no source text) can be inspected but are not
  analysed or run.
* Not supported: forms and other UI need an embedding application that provides them.

## Quick start

Requires a Rust toolchain that satisfies `rust-version` in [Cargo.toml](Cargo.toml).

```sh
cargo build --release -p vba-cli          # builds target/release/vbars

cat > Hello.bas <<'EOF'
Option Explicit
Public Sub Main()
    Debug.Print "Hello from VBA"
End Sub
EOF

vbars check Hello.bas                     # syntax and semantic diagnostics
vbars run Hello.bas                       # runs Main; Debug.Print goes to stdout
vbars inspect path/to/vbaProject.bin      # list modules (add --json for machine output)
cat Hello.bas | vbars check -             # - reads standard input
vbars --help                              # all commands and the exit codes
```

Other commands: `lex`, `parse`, `fmt-check`, `profiles`, `completions SHELL` (bash, zsh,
fish, powershell, elvish) and `man` (a roff manual page). `check`, `parse`, `fmt-check`
and `profiles` accept `--json`. Diagnostics are colored only on a terminal and never when
`NO_COLOR` is set (`--color always|never` overrides).

Exit codes: 0 success; 1 findings or bad input; 2 usage error (and, for `run`, an
unhandled run-time error); 3 permission denied; 4 resource limit or cancelled; 5
unsupported feature or internal error; 6 compiled-only project.

To embed the engine in an application, start with the
[embedding guide](docs/architecture/embedding-guide.md).

## Security model in brief

VBA from a project file is untrusted. The full statement, with the tests behind each claim
and the list of what is not guaranteed, is in
[docs/architecture/security-model.md](docs/architecture/security-model.md).

* Loading a project compiles it and runs nothing (no `Auto_Open`, no `Class_Initialize`).
  The embedding application must explicitly trust a session before anything runs.
* Seven capabilities (file read, file write, process spawn, network, COM creation, native
  `Declare`, UI automation) are all denied by default. A grant can be limited to a path,
  program, ProgID, host or action. `vbars run` grants nothing unless given `--grant`.
* Runs are bounded by step, memory and call-depth limits that the program cannot lift;
  `vbars run` applies finite limits unless `--unlimited` is given.
* Hostile input files are rejected with errors instead of crashing the process.
* Limits are accounting, not a sandbox: use operating-system limits to contain hostile
  code. See "Not guaranteed" in the security model.

## Crate map

| Group | Crates |
| --- | --- |
| `crates/foundation/` | `vba-core`, `vba-source`, `vba-diagnostics`, `vba-profile` |
| `crates/language/` | `vba-lexer`, `vba-syntax`, `vba-ast`, `vba-sema`, `vba-ir`, `vba-pcode-ir`, `vba-lsp-core` |
| `crates/runtime/` | `vba-value`, `vba-stdlib`, `vba-vm`, `vba-runtime`, `vba-scripting` |
| `crates/model/`, `crates/formats/` | `vba-project`, `vba-format`, `vba-pcode` |
| `crates/hosts/` | `vba-host` (the contract an embedder implements) |
| `crates/interoperability/` | `vba-automation`, `vba-ffi`, `vba-typelib` |
| `crates/windows/` | `vba-winapi`, `vba-wine` (behind cargo features) |
| `crates/api/` | `vba-rs` (the `vba` facade embedders depend on) |
| `apps/` | `vba-cli` (`vbars`), `vba-lsp`, `vba-wine-host` |
| `tooling/` | `vba-winapi-gen` |

The [architecture overview](docs/architecture/overview.md) explains the dependency
direction.

## Documentation

See [docs/README.md](docs/README.md) for the index. Contributing: [CONTRIBUTING.md](CONTRIBUTING.md).
Security reports: [SECURITY.md](SECURITY.md). Changes: [CHANGELOG.md](CHANGELOG.md).

## License

Apache-2.0, see [LICENSE](LICENSE). Third-party notices are in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
