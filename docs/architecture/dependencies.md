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
| `serde`, `serde_json` | LSP message payloads (vba-lsp); the JSON form of a host library, through `serde_json::Value` only (vba-host-model) | vba-lsp, vba-host-model |

### Test-only

| Dependency | Role | Used by |
| --- | --- | --- |
| `assert_cmd` | End-to-end tests of the `vbars` binary | vba-cli |
| `expect-test` | Snapshot tests | vba-ast, vba-ir, vba-syntax |
| `proptest` | Property tests | most crates |
| `tempfile` | Temporary directories | vba-cli, vba-runtime, vba-conformance |
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

## What an embedding application pulls in

Measured with a throwaway crate outside the repository that depends on `vba-rs` by
path and uses only the documented facade API (Rust 1.99, macOS, dev profile without
debug info, `-j 3`, registry already cached). The consumer resolves its own lock file;
the lock file of this workspace is not used by dependents, so pin the versions that
matter to you in your own `Cargo.lock`.

| Features | Third-party crates | Duplicate versions | Cold build of the consumer | Needs besides `rustc` |
| --- | --- | --- | --- | --- |
| none (default) | 38 | none | about 11 s | nothing: no C compiler, no system library |
| `winapi` | 38 | none | about 4 s more | nothing |
| `wine` (implies `winapi`) | 38 | none | about 2 s more | nothing (the helper executable is a separate package, `apps/vba-wine-host`) |
| `native-call` | 44 (adds `libffi`, `libffi-sys`, `libloading`, `cc`, `shlex`, `find-msvc-tools`) | none | about 51 s more | a C compiler for `libffi-sys` (the only build step outside `rustc`) |

* The default build has build scripts only in well-known crates (`libc`, `rustix`,
  `getrandom`, `nix`, `crc32fast`, `proc-macro2`, `quote`, `thiserror`, `encoding_rs`,
  `rustversion`); none reaches the network or probes for system libraries.
* Fetching the crates from the registry needs network access once (or a vendored
  source or an offline registry cache); nothing is downloaded at build or run time.
* `serde`, `clap`, `lsp-*`, `tempfile`, `proptest` and the other tool and test
  dependencies are not in the graph of an embedding application: they belong to the
  `vbars` and language-server packages and to dev-dependencies.
* `unsafe` appears in the workspace only in `vba-ffi` behind `native-call`; every
  other crate forbids it. Dependencies have their own `unsafe` (`libc`, `rustix`,
  `windows-sys`...), as usual.

## Distribution policy

Every package in the workspace has `publish = false` (set once in
`[workspace.package]`; the two development tools set it themselves). vba-rs is consumed
as a git dependency pinned to a revision, or by path; it is not published to crates.io.
All packages share one version (`0.1.0`) and `rust-version = "1.99"`, edition 2024, and
are licensed Apache-2.0. Publishing later means removing `publish = false`, adding a
version to the path dependencies in `[workspace.dependencies]` and publishing in
dependency order; nothing else in the manifests needs to change (descriptions, license,
repository, readme, keywords and categories are already set).

## Reproducible builds and toolchain

Findings of the hygiene pass (Rust 1.99.0, the lock file of the commit that introduced
this section):

* `rust-toolchain.toml` pins `1.99.0` with `rustfmt` and `clippy`. It equals the workspace
  `rust-version = "1.99"`, so the pinned compiler is also the minimum supported one; raise
  both together. The file only affects builds inside this repository, not embedding
  applications.
* `Cargo.lock` is committed (it is not in `.gitignore`). `cargo build --locked`,
  `cargo metadata --locked`, `cargo clippy --locked` and `cargo doc --locked` succeed and
  leave the lock file unchanged; CI uses `--locked` everywhere, so a manifest change
  without a lock update fails the build. A dependent resolves its own lock file.
* Every package comes from crates.io (no repository or alternative-registry source) and no
  forbidden crate (`rayon`, `tokio`, `rowan`) is in the lock file; `deny.toml` enforces both.
* Duplicate versions among the 157 third-party packages, none in the graph of an embedding
  application with the default features (see the table above):
  `bitflags` 1.3.2 (via `fluent-uri` and `lsp-types`; language server only) next to 2.x;
  `syn` 2 and 3 (`jiff-static`, a dev-only test oracle, still uses 2);
  `getrandom` 0.2/0.3/0.4, `rand` 0.8/0.9/0.10, `rand_core` 0.6/0.9/0.10 and `rand_chacha`
  0.3/0.9 (all from `proptest` and `tempfile`, dev-dependencies); `r-efi` 5/6 (UEFI
  targets only). They cost build time only; `deny.toml` reports them as warnings.
* `cargo-deny` and `cargo-audit` were still **not** run locally (not installed, no
  network). `deny.toml` and `.github/workflows/audit.yml` run them weekly and on lock-file
  changes. The licence list in `deny.toml` was written from the audit below and
  cross-checked against `cargo metadata --all-features`, but the first CI run is the real
  validation of the file syntax and of the advisory check.

## Licence audit

Method: the `license` field of the manifest of every package in `Cargo.lock`
(`cargo metadata --all-features`, so it includes dev-dependencies and the packages of
every target platform, 157 third-party packages), read from the local registry cache.
`cargo-deny` and `cargo-audit` are **not installed** in the environment that produced
this section (they run in CI, see above), so the following were **not checked**: security advisories (RUSTSEC),
yanked releases, the license text actually shipped in each package (a manifest can
disagree with its files), and `deny.toml`-style bans. Re-run them in continuous
integration; the results below are a manual reading, valid for the lock file of the
commit that introduced this section.

| License expression (as declared) | Packages | Note |
| --- | ---: | --- |
| `MIT OR Apache-2.0` and the same options in other spellings (`Apache-2.0 OR MIT`, `MIT/Apache-2.0`, `Apache-2.0 / MIT`) | 120 | permissive, dual |
| `MIT` | 16 | includes `cfb`, `zip`, `quick-xml`, `nix`, `lsp-types` |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 5 | `rustix`, `linux-raw-sys`, WASI crates |
| `Unlicense OR MIT` | 4 | `memchr`, `jiff*` |
| `Apache-2.0` | 2 | `borsh-derive`, `dissimilar` (test or tool graph only) |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | 2 | `r-efi` (UEFI targets only); the permissive options apply |
| `BSD-2-Clause OR Apache-2.0 OR MIT` | 2 | `zerocopy*` |
| `(Apache-2.0 OR MIT) AND BSD-3-Clause` | 1 | `encoding_rs`: the BSD-3-Clause part is mandatory, so binary redistributions must reproduce its notice |
| `ISC` | 1 | `libloading` (feature `native-call`) |
| `0BSD OR MIT OR Apache-2.0`, `MIT OR Zlib OR Apache-2.0`, `Zlib`, `(MIT OR Apache-2.0) AND Unicode-3.0` | 4 | `adler2`, `miniz_oxide`, `zlib-rs`, `unicode-ident` |

Result: no copyleft licence is *required* by any package (the only copyleft appearance is
an optional alternative of `r-efi`); no package without a license field; every package
comes from the crates.io registry (no git or unknown-registry dependency). The
obligations that remain are attribution ones: keep the license notices of the crates you
redistribute in binary form (`encoding_rs` additionally carries the BSD-3-Clause and
Unicode-3.0 texts). `THIRD-PARTY-NOTICES.md` records the one body of data derived into
this repository (the Win32 function inventory).
