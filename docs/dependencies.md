# External dependencies

Every third-party dependency of the workspace, in the order of the
`[workspace.dependencies]` groups in the root `Cargo.toml`. Versions live only
there; crates inherit them with `.workspace = true`.

Source text is bytes in a Windows code page, so no library that assumes UTF-8
text is used on the language path. Observable order (diagnostics, output,
listings) must never depend on hash order or on thread scheduling.

## In use

| Dependency | Role | Used by |
| --- | --- | --- |
| `thiserror` | `Error` derive for library error enums | vba-cli, vba-format, vba-ffi, vba-ir, vba-project, vba-runtime, vba-winapi, vba-wine |
| `bitflags` | Typed flag sets with `|`, `contains`, `intersects` | vba-automation (`InvokeFlags`), vba-syntax (parser block terminators) |
| `indexmap` | Hash map with insertion-ordered iteration, so order is structural | vba-lsp-core (open-document table) |
| `cfb` | Compound File Binary container reading and writing | vba-format |
| `encoding_rs` | Windows code page decoding and encoding | vba-format, vba-lsp-core, vba-stdlib |
| `uuid` | GUIDs in project and type-library records | vba-format |
| `libffi` | Dynamic native calls for `Declare` (feature `native-call`) | vba-ffi |
| `libloading` | Loading native libraries (feature `native-call`) | vba-ffi |
| `windows-sys` | Win32 bindings for the Wine helper | vba-wine-host |
| `annotate-snippets` | Source-snippet rendering of diagnostics (CLI only; `vba-diagnostics` stays dependency-free) | vba-cli |
| `clap` | Command-line parsing | vba-cli |
| `lsp-server`, `lsp-types` | Language server protocol transport and types | vba-lsp |
| `serde`, `serde_json` | LSP message payloads | vba-lsp |

### Test-only

| Dependency | Role | Used by |
| --- | --- | --- |
| `assert_cmd` | End-to-end tests of the `vbars` binary | vba-cli |
| `expect-test` | Snapshot tests | vba-ast, vba-ir, vba-syntax |
| `proptest` | Property tests | most crates |
| `tempfile` | Temporary directories | vba-cli, vba-runtime |
| `rust_decimal`, `jiff` | Independent oracles for `Currency`/`Decimal` and date arithmetic. Dev-dependencies only; never shipped | vba-value |

## Evaluated and not adopted

| Candidate | Decision and reason |
| --- | --- |
| `slotmap` | `ObjectTable` (vba-automation) and `HandleTable` (vba-ffi) are not slot arenas: handles are monotonic and never reused (`ObjectHandle(1), (2), ...`; tokens `0x100 + 4n`), and the VM partitions the handle space by a high bit. A slotmap reuses slots under a version counter, changes the numeric representation and eventually wraps, which weakens "a stale handle is never aliased by a later object". |
| `smallvec` | Measured on the call path (small `Vec<BoundArg>` argument lists in vba-vm, release build): 1.30 s vs 1.32 s on a call-heavy program (`Fib(29)`), 0.58 s vs 0.58 s on the mixed program. Within noise, so the extra dependency was not kept. |
| `rustc-hash` | No hot map in the interpreter loop: the VM's maps are touched only on late-bound calls and class default instances, and the compile-time maps are not performance relevant. Nothing to gain; every observable-order map is already sorted explicitly. |
| `memchr` | Lexer and scanners are not shown to be bound by byte search; skipped until profiling shows otherwise. |
| `miette` | Heavier (themes, terminal detection, a diagnostic trait on every error type) than needed; `annotate-snippets` is used instead. |
| `rayon` | Forbidden: parallel iteration risks observable-order differences. |
| `tokio` | Forbidden in core crates: no async runtime in a deterministic interpreter. |
| `rowan` | Forbidden: `vba-syntax` has its own lossless, byte-exact tree that keeps source bytes untouched. |
| `logos` | Forbidden: the lexer is byte-based, context sensitive (line continuations, `Rem`, date literals, code pages) and does not fit a UTF-8 derive-based lexer. |
| `rust_decimal`, `jiff` (shipped) | VBA `Currency`/`Decimal` and `Date` have exact, VBA-specific rounding and range rules implemented in vba-value; both crates are kept only as test oracles. |
| `salsa` | Incremental computation framework; the language server rebuilds a project per generation, which is simple and deterministic at the current scale. |
| `ropey` | Rope text buffer; documents are byte buffers in a code page, not UTF-8 text, and edits are whole-document. |
| `line-index` | UTF-8 oriented; `vba-source::LineIndex` works on bytes and handles CR, LF and CRLF as VBA does. |
| `binrw` | Out of scope for this pass (vba-format has its own owner). |
