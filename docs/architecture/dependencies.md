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
| `thiserror` | `Error` derive for library error enums | vba-cli, vba-format, vba-ffi, vba-ir, vba-project, vba-runtime, vba-typelib, vba-winapi, vba-wine |
| `bitflags` | Typed flag sets with `|`, `contains`, `intersects` | vba-automation (`InvokeFlags`), vba-syntax (parser block terminators), vba-typelib (type, function, parameter and library flags) |
| `indexmap` | Hash map with insertion-ordered iteration, so order is structural | vba-lsp-core (open-document table), vba-scripting (`Scripting.Dictionary` key order) |
| `cfb` | Compound File Binary container reading and writing | vba-format |
| `encoding_rs` | Windows code page decoding and encoding | vba-format, vba-lsp-core, vba-stdlib, vba-typelib (type library names and strings) |
| `uuid` | GUIDs in project and type-library records | vba-format |
| `object` | Reader for PE images (resource directory of `TYPELIB` resources), features `read_core`, `pe`, `std` only (Apache-2.0 OR MIT) | vba-typelib |
| `rustix` | Directory-relative file system calls (`openat`, `mkdirat`, `unlinkat`, `renameat`, `fstatat`) for `RealFs` on Unix, so a path is walked one directory at a time without following links and the operation acts on the opened directory. Features `fs` and `std` only; safe API, so `vba-runtime` keeps `unsafe_code = "forbid"`. Already in the tree through `tempfile`. `cfg(unix)` only | vba-runtime |
| `libffi` | Dynamic native calls for `Declare` (feature `native-call`) | vba-ffi |
| `libloading` | Loading native libraries (feature `native-call`) | vba-ffi |
| `windows-sys` | Win32 bindings for the Wine helper (`cfg(windows)`), and Windows-header constants on every host: HRESULT/`DISP_E_*`/`TYPE_E_*`, `DISPID_*`, `DISPATCH_*`, `ERROR_*`, `CP_*`, `MB_*`/`WC_*`. Constants only: nothing is linked on non-Windows hosts | vba-wine-host, vba-automation, vba-winapi |
| `windows-metadata`, `windows-default` | Reader for Microsoft's Win32 metadata and the `Windows.Win32.winmd` it ships (MIT OR Apache-2.0). Dev tool only | vba-winapi-gen (never a library dependency) |
| `annotate-snippets` | Source-snippet rendering of diagnostics (CLI only; `vba-diagnostics` stays dependency-free) | vba-cli |
| `clap` | Command-line parsing | vba-cli |
| `clap_complete`, `clap_mangen` | `vbars completions` (shell completion scripts) and `vbars man` (manual page), generated from the same definition as `--help` | vba-cli |
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
| `cap-std` (sandboxed directory handles) | Not adopted. It would give the same containment as the walk in `RealFs`, but it brings a larger tree (`cap-primitives`, `io-lifetimes`, `ambient-authority`, `fs-set-times`) and its model, a capability root per grant, does not fit a policy that decides by resolved path. `rustix` alone is enough for a walk that follows no link. |
| `pelite`, `goblin` (PE resource walking for `vba-typelib`) | `object` was adopted. `pelite` (MIT) has the nicest resource API but contains `unsafe` in 30 source files and its last release is 0.10.0. `goblin` (MIT only) parses ELF/Mach-O/archives by default and pulls `scroll`, `plain` and `log`; its PE resource module is large (about 2000 lines) for a need of one resource type. `object` (Apache-2.0 OR MIT, same licence as the workspace) with `default-features = false` and `read_core`/`pe`/`std` adds only itself and `memchr` (already in the tree), reads PE32 and PE32+, has no `unsafe` in its PE reader (its byte-casting helpers are shared, audited code) and bounds checks every table read. |
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
| `windows-bindgen` output, the `windows` crate | Generated bindings and COM wrappers (`IDispatch`, `VARIANT`, `BSTR` wrappers) would put Windows-only types on the portable path. vba-automation models Automation without COM; VBA-visible types (`VarType`, `VbaError` numbers) keep VBA semantics defined in our crates. Only constants are taken from windows-sys. |
| `windows-result`, `windows-strings` | `apps/vba-wine-host` has no HRESULT or wide-string/BSTR code (it calls `LoadLibraryA` with `CString` and reads `GetLastError` codes), so there is nothing for them to replace. |
