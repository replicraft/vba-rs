# vba-rs

A standalone, strict implementation of Visual Basic for Applications (VBA) in Rust: a library
workspace, a command-line tool (`vbars`) and a language server. It runs on any platform and
does not depend on Office or on any embedding application.

## The compatibility contract

* **Faithful to the language specification.** Behavior follows MS-VBAL and the Microsoft
  language reference. Where the documents are silent, the behavior is recorded as
  `derived` or `unverified`, never presented as fact.
* **Measured, not claimed.** Compatibility is whatever the conformance suite checks, and the
  report says which expectations are documented and which are only the implementation's own
  behavior. See [docs/tooling/conformance.md](docs/tooling/conformance.md).
* **Deny by default.** A program can compute and use `Debug.Print`. Files, processes, the
  network, object creation, native calls and UI automation each need an explicit grant from
  the embedder (or `--grant` on the command line), optionally scoped to a path prefix.
* **No automatic execution.** Opening, reading, analysing or converting a project never runs
  user code. Running is an explicit act, and imported code stays untrusted until the embedder
  trusts it.

## Status

Counts below come from the generated [conformance report](docs/tooling/conformance.md) and
from `cargo test --workspace`.

* **Works.** 329 of 336 language-reference entries (statements, functions, operators, data
  types, objects, language elements) are `implemented`, 1 is `partial` (`StrConv` with
  an odd byte count), and 6 are `unsupported`. 679 conformance programs run through the
  public facade; 5 of them are recorded known deviations that must keep failing.
  Alongside: the lexer, parser, semantic analysis, IR and VM; the MS-OVBA reader and writer
  with byte-preserving round trips; type-library reading; native `Scripting.Dictionary` and
  `Scripting.FileSystemObject`; a language server core. `cargo test --workspace` runs
  1,147 tests pass (6 more are ignored: they need local material or a Wine installation).
* **Unverified.** An entry marked `implemented` means its listed programs pass, not that
  the construct is fully verified. Of the 336 entries, 140 rest on documentation examples,
  71 on reasoning from documented semantics and 125 only on the implementation's own
  behavior (no document backs the expected value). The Wine bridge and the native `Declare`
  path are behind cargo features, off by default; what has been run is listed in
  [docs/interop/wine-bridge.md](docs/interop/wine-bridge.md). Decoding of compiled code
  (p-code) is verified only for one layout (VBA 7.0, 32-bit); other layouts are implemented
  from text.
* **Unsupported.** `Load`/`Unload` and `UserForm` (they need a host that provides forms),
  `FileSystemObject.Drives`/`Drive`, and `AddressOf` without the native `Declare` bridge.
  Compiled-only projects (no source) are recognised and preserved but not executed.
  The full list is the Gaps table in the conformance report.

## Repository layout

| Path | Contents |
| --- | --- |
| `apps/vba-cli` | `vbars`, the command-line tool |
| `apps/vba-lsp` | language server binary over `vba-lsp-core` |
| `apps/vba-wine-host` | helper executable for the Wine bridge |
| `crates/api/` | `vba-rs`, the facade embedders depend on (imported as `vba`) |
| `crates/foundation/` | core types, source files, diagnostics, language profiles |
| `crates/language/` | lexer, syntax tree, AST, semantic analysis, IR, editor analysis |
| `crates/runtime/` | values, standard library, VM, sessions, `Scripting` objects |
| `crates/model/`, `crates/formats/` | project model, MS-OVBA format, p-code decoder |
| `crates/hosts/` | the host contract: capabilities, policy, limits |
| `crates/interoperability/` | automation traits, native-call layout, type libraries |
| `crates/windows/` | Win32 `Declare` shims and the Wine bridge (feature-gated) |
| `tooling/` | developer tools that generate checked-in data |
| `tests/` | conformance programs (`tests/vba/`) and the inventory |
| `docs/` | documentation, starting at [docs/README.md](docs/README.md) |

## Build and test

```sh
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Regenerate the conformance report and inventory columns after changing a conformance
program or the inventory:

```sh
VBA_BLESS=1 cargo test -p vba-rs --test conformance -- --test-threads=1
```

## Using `vbars`

```sh
cargo run -p vba-cli -- <command>        # or install the `vbars` binary

vbars profiles                            # list language profiles
vbars inspect Book.bin                    # modules, references, source state of a vbaProject.bin
vbars inspect --json Book.bin             # machine-readable form
vbars check Module1.bas Class1.cls        # syntax and semantic diagnostics, exit 1 on errors
vbars check --profile vba6-excel-win32 *.bas    # analyse under another profile
vbars run Module1.bas                     # run Main; nothing but computation and Debug.Print
vbars run --entry Module1.Report --arg 42 Module1.bas
vbars run --grant file-read=/data --grant file-write=/tmp/out Module1.bas
vbars run --memfs --max-steps 1000000 Module1.bas
```

`run` exit codes: 0 success, 1 bad input or does not compile, 2 unhandled run-time error,
3 permission denied, 4 resource limit or cancelled, 5 unsupported feature or internal error,
6 compiled-only project. Capabilities: `file-read`, `file-write`, `process-spawn`, `network`,
`com-create`, `native-declare`, `ui-automation`.

## Embedding

```rust
use vba::{Engine, Project};

let project = Project::from_source("Module1", "Public Sub Main()\n  Debug.Print \"hi\"\nEnd Sub\n");
let engine = Engine::new();                       // deny-all policy
assert!(!engine.analyze(&project).has_errors());  // diagnostics only, nothing runs
let mut instance = engine.load(&project)?;        // compiled but untrusted
instance.trust();                                 // the embedder's explicit decision
let out = instance.run("Main", vec![])?;
assert_eq!(out.output, "hi\n");
# Ok::<(), vba::Error>(())
```

Next: [docs/architecture/overview.md](docs/architecture/overview.md),
[docs/architecture/public-api.md](docs/architecture/public-api.md) (the facade, host objects,
limits, cancellation) and [docs/architecture/host.md](docs/architecture/host.md) (the contract
an embedder implements).

## Documentation

[docs/README.md](docs/README.md) indexes every document by area: architecture, language,
runtime, formats, interop and tooling.

## License

Apache-2.0, see [LICENSE](LICENSE). Third-party material and its licenses are listed in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
