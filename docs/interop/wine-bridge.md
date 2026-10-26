# Wine bridge (`vba-wine`, `vba-wine-host`)

A fallback for `Declare` calls into Windows DLLs that `vba-winapi` does not
emulate: a native process drives a small Windows helper running under Wine.
Crates: `crates/windows/vba-wine` (client, protocol, helper dispatcher) and
`apps/vba-wine-host` (the helper executable). Nothing else depends on them.

## Security model (read first)

* **Wine is not a sandbox.** A DLL loaded in the helper runs with the full
  rights of the user that started Wine, can read and write that user's files
  (Wine maps the host filesystem as drive `Z:`), use the network and start
  processes. The bridge adds no confinement. Use an OS-level sandbox or a
  separate user if the DLLs are not trusted.
* **Nothing runs without an explicit grant and an explicit start.** The
  embedder must (1) choose the Wine and helper paths, (2) call
  `Session::spawn` itself, (3) hand the session to `WineProvider::new`, which
  fails unless the `vba-host` policy grants `Capability::NativeDeclare`, and
  (4) list the allowed exports. No code path starts Wine on its own, and
  `PATH` is read only if the embedder calls `find_wine_in_env`.
* **Capability mapping.** No capability was added to `vba-host`. The Wine
  bridge is `NativeDeclare`; each call re-checks
  `policy.check(NativeDeclare, "library!symbol")`, so a grant such as
  `"kernel32!GetTickCount"` (one procedure) or `"kernel32!*"` (one library)
  restricts it. `ProcessSpawn` is *not* implied or consulted:
  starting the helper is the embedder's act, outside VBA reach.
* **Only registered exports are reachable**, with a fixed signature (`WineExport`).
  The helper itself cannot tell a harmless export from a dangerous one; it will
  call whatever it is asked to, so the export table and policy are the control.

## What crosses the boundary

| Crosses | Does not cross |
| --- | --- |
| Library names, export names/ordinals | Raw pointers or addresses of either process |
| Integer and float scalars by value | Emulated-memory addresses (`NativeArg::Int` pointers are refused, see below) |
| Byte buffers, copied by value in and/or out | Shared memory, file descriptors, handles of the OS |
| Opaque ids for libraries/procedures, minted by the helper per request, mapped to session- and provider-scoped `vba-ffi` handles on the client | HMODULE / function addresses (kept in the helper's `Server` table) |
| `GetLastError` captured right after the call | Callbacks, function pointers, window messages |

`WineProvider` as a `vba-winapi::Provider` forwards only scalar-only exports:
`NativeArg::Int` is an integer or an address into the *emulated* address space,
and the bridge cannot know which, so passing it to a real process would be
meaningless and unsafe. Buffer-taking exports go through
`WineProvider::call_with_buffers`, with explicit `WireArg::BufIn/BufInOut/BufOut`.
Wiring `vba-ffi` marshaling plans to buffers automatically is not done.

## Protocol (version 1)

Frame: `u32` LE length + body (max 16 MiB, checked before allocation). Body:
tag, `u64` request id, fields; strings are `u16` length + UTF-8 (max 4096);
at most 60 arguments. Requests: Hello, Ping, LoadLibrary, GetProc (name or
ordinal), Call (convention, return kind, arguments), FreeLibrary, Shutdown.
Responses echo the id: Hello (version, bitness, OS, Wine version, helper name,
capability bits), Ok, Library, Proc, CallResult (value, last error, out
buffers) and Error with a kind: `ProtocolViolation`, `LibraryNotFound`,
`SymbolNotFound`, `CallFault`, `PermissionDenied`, `Unsupported`,
`InvalidHandle`, `VersionMismatch`. Hello must come first and carries the
protocol version; a mismatch is an error on both sides.

Client-side errors (`WineError`) add `Unavailable` (Wine/helper not configured
or missing; nothing was started), `Transport`, `Protocol` (malformed frame,
wrong id or response type, undeclared out buffer), `HelperCrashed`, `Timeout`,
`BadRequest` (rejected locally), `SessionClosed`. Transport, protocol, crash
and timeout errors kill the helper and close the session permanently; remote
errors such as `LibraryNotFound` leave it usable.

Through the `Provider` trait, which has no bridge error type, every
`WineError` becomes `WinApiError::Unsupported` whose reason starts with
`wine bridge <Kind>:`; callers wanting the distinct error use `Session` or
`WineProvider::call_with_buffers`. A bridge failure is never a success value.

## Operation

* One request in flight per session: `Session` methods take `&mut self`;
  `WineProvider` serializes behind a `Mutex`. Use several sessions for
  parallelism. Default timeout 10 s per request (configurable); on expiry the
  child is killed.
* Dropping a session (or its transport) kills and reaps the helper.
* The helper's stderr is discarded; stdout carries only frames.
* `WineConfig::wine(wine, helper)` runs `wine helper.exe` (optional
  `WINEPREFIX`, `WINEDEBUG=-all`); `WineConfig::direct(helper)` runs the helper
  natively (tests, or a real Windows host).
* The helper must match the declared target's bitness (checked per call; a
  32-bit `Declare` needs a 32-bit helper build, run with a 32-bit capable Wine).
* Build the helper for Windows, for example
  `cargo build -p vba-wine-host --target x86_64-pc-windows-gnu` (or `-msvc`,
  `i686-` for 32-bit). Producing the PE on a machine without a Windows or
  mingw SDK is possible (see "Verified under Wine").

## Implemented and tested (on Linux/macOS CI)

* Protocol encode/decode round trips, truncation at every byte, trailing
  bytes, unknown tags, hostile counts/lengths, bad UTF-8, oversized frames,
  pseudo-random garbage (no panics).
* Client against an in-process mock helper: hello, load/resolve/call/free,
  in/out buffers, last error, handle scoping across sessions and after free,
  local validation, 32-bit argument width, timeout, crash, version mismatch,
  malformed/mismatched/misplaced responses, Hello-first enforcement.
* `WineProvider`: grant requirement, scoped grants, export-table validation,
  Registry integration, visible failures.
* Client against the real `vba-wine-host` binary in protocol-only mode:
  hello, ping, `Unsupported` for library operations, version mismatch, helper
  exit, hang with timeout and kill, garbage output, missing files, kill on drop.
* The Windows code path type-checks and passes `cargo clippy --all-targets -- -D
  warnings` for `x86_64-pc-windows-msvc` and `i686-pc-windows-msvc`,
  cross-checked from macOS (the i686 build is checked only).
* Wine-gated tests (`crates/windows/vba-wine/tests/wine_live.rs`) run only
  when `VBA_WINE` (path to `wine`) and `VBA_WINE_HELPER` (path to a Windows
  build of the helper) are set, optionally `VBA_WINE_PREFIX`; otherwise each
  test returns immediately and the default `cargo test --workspace` is
  unaffected.

## Verified under Wine

Executed once, by hand, on this configuration:

* Host: Apple Silicon (arm64) macOS 14. The bundled Wine Stable 10.0
  (`wine_get_version` reports it) is an **x86_64 Mach-O executable, so it ran
  under Rosetta 2 emulation**; the helper is a 64-bit PE and the DLLs are Wine's
  builtin PE DLLs. Nothing was run on genuine Windows, on real x86 hardware, or
  with a 32-bit helper.
* The helper was built as `x86_64-pc-windows-msvc` with `rust-lld` (no
  Microsoft SDK, no mingw): import libraries for `kernel32`, `ntdll`, `userenv`,
  `ws2_32`, `dbghelp`, `msvcrt` and `ucrtbase` were generated from the export
  tables of Wine's own PE DLLs with `llvm-dlltool`, `-C panic=abort`, and a
  ~15-line C object supplying the entry point, the TLS directory and one
  data symbol in place of the MS CRT. This is a test-only build recipe, not a
  supported release configuration; a normal MSVC or mingw build was not tried.
* Passing, through the real `vba-wine` client against that PE: Hello (OS
  `windows`, 64-bit, Wine version string present), Ping, Shutdown;
  `LoadLibrary` by name with and without `.dll`; `GetProcAddress` by name;
  stdcall calls (`GetTickCount`, `lstrlenA`, `MultiByteToWideChar`,
  `WideCharToMultiByte` with an eight-argument signature and null pointers,
  `SetLastError`, `GetFileAttributesA`); cdecl calls (`strlen`, `abs`, variadic
  `sprintf` including a 16-argument call that spills to the stack); in and out
  buffers copied correctly; `GetLastError` captured (1234 from `SetLastError`,
  2 or 3 from a missing file, 0 after a call that sets none); missing library
  (`LibraryNotFound`, Win32 error 126), missing name (`SymbolNotFound`, 127) and
  missing ordinal; `FreeLibrary` invalidating procedure handles; and a helper
  that dies inside a call (`ExitProcess`) reported as `HelperCrashed`.
* Floating point now works on x86-64 (see below) and is verified for
  `pow(double,double)`, `ldexp(double,int)` (float then int positions),
  `sqrtf(float)` and `lround(double)` (integer return, float argument).

Findings: the helper's stdout stayed clean (Wine and MoltenVK write their
diagnostics to stderr, which the client discards). Wine creates symlinks from
the prefix to the user's real Desktop/Documents/etc. and `/Volumes` drive
letters; for the test prefix they were replaced by empty directories, but a
prefix made by default exposes them (and `Z:` always maps `/`), another reason
Wine is not a sandbox. An `int`/`DWORD` return leaves the upper half of `RAX`
unspecified and the bridge returns the full register, so callers must narrow to
the declared width (the tests mask to 32 bits).

### Float arguments and returns (x86-64 only)

Win64 puts argument `i < 4` in `rcx/rdx/r8/r9` *or* `xmm0..3` by its type. The
helper calls through a *variadic* function pointer with every argument passed as
an `f64` carrying the argument's 64 bits: for variadic calls Win64 places each
of the first four in both registers, and later ones in 8-byte stack slots, so a
non-variadic callee finds its integer or float where it expects it. `f32`
travels as bits in the low half. Returns are read from `rax` or `xmm0` by the
declared return kind. This is only used when a float is involved; on a 32-bit
helper floats remain `Unsupported`.

## Reproducing the live tests (macOS, no Windows SDK)

The Wine-gated tests need a Windows build of `vba-wine-host`. On a machine with
neither the Microsoft SDK nor mingw, the files in
`crates/windows/vba-wine/tests/support/` build one from Wine's own DLLs and run
the tests with one command.

**Prerequisites** (nothing is downloaded or installed by the scripts):

* Wine with 64-bit PE DLLs, default location `/Applications/Wine Stable.app`
  (override with `WINE_BIN`; `DLL_DIR` if its `lib/wine/x86_64-windows` is
  elsewhere). The recipe was exercised with Wine Stable 10.0, an x86-64 build
  running under Rosetta 2 on Apple Silicon.
* Rust with the `x86_64-pc-windows-msvc` target (`rustup target add`); the
  `rust-lld` that ships with the toolchain is the linker.
* `llvm-dlltool` (LLVM, e.g. Homebrew `llvm`; set `DLLTOOL` if not on `PATH`),
  `clang`, and `python3`.

**What the build replaces.** rustc's `*-pc-windows-msvc` output expects the MSVC
import libraries and C runtime, which are unavailable here:

* `make-implibs.sh` runs `gen_implibs.py`, which reads the export tables of
  Wine's PE DLLs (`kernel32`, `ntdll`, `userenv`, `ws2_32`, `dbghelp`, `msvcrt`,
  `ucrtbase`) into `.def` files (forwarded exports included, the loader resolves
  them) and `llvm-dlltool` turns them into import libraries. These stand in for
  the SDK libraries; at run time the helper binds to Wine's real DLLs.
* `startup.c`, compiled with `clang --target=x86_64-pc-windows-msvc`, supplies
  what the CRT start-up object normally provides: the `mainCRTStartup` entry
  point (calls `main`, then `ExitProcess`), the TLS directory (`_tls_used`,
  `_tls_index` and the `.tls` bracketing markers, without which `std` thread
  locals fault at start) and one `type_info` vftable symbol referenced by `std`.
* The helper is built with `-C panic=abort`, `/nodefaultlib`, and `msvcrt.lib`
  plus `ucrtbase.lib` linked explicitly.

**One command.** `crates/windows/vba-wine/tests/support/run-tests.sh` (all
outputs go to the gitignored `target/wine-tests/`, or `WORK`):

1. generates the import libraries and the start-up object (once);
2. creates a dedicated Wine prefix (`WORK/prefix`) and replaces the symlinks
   Wine makes to the user's real Desktop/Documents/etc. and `/Volumes` by empty
   directories, removing drive letters other than `C:` (`Z:` -> `/` always
   remains: Wine is not a sandbox);
3. builds `vba-wine-host` for `x86_64-pc-windows-msvc` in a temporary target
   directory (`CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`), copies
   `vba-wine-host.exe` to `WORK`, and deletes the target directory (set
   `KEEP_TARGET=1` to keep it);
4. runs `cargo test -p vba-wine --test wine_live` with `VBA_WINE` (the `wine`
   binary), `VBA_WINE_HELPER` (the built `.exe`) and `VBA_WINE_PREFIX` (the
   prefix) set. These three variables are the whole interface of the tests;
   without them each test returns immediately.

**Expected result:** `test result: ok. 8 passed` (about 10 s after the first,
slower prefix creation).

**Caveats.** Wine here runs under Rosetta 2, so this says nothing about real x86
hardware or Windows. The toolchain recipe (generated import libraries, hand-made
start-up object, `panic=abort`) is test-only: the resulting `.exe` is not a
release build and is not meant to be distributed. A normal MSVC or mingw build
of the helper is the supported route and works with the same environment
variables.

## Relation to the in-process native stage

`vba-ffi`'s `native-call` feature calls host libraries in-process and refuses
declarations whose target ABI differs from the host, so a Win64 `Declare` never
runs on macOS or Linux; running Windows DLLs is this bridge's job. What has
actually run: the in-process stage (macOS, libc/libm, see ffi.md), the
protocol-only helper, and the real `native` module under Wine (64-bit, Rosetta).
What has not: that module on genuine Windows, or a 32-bit helper.

## Unverified / not implemented

* Anything on genuine Windows, on x86 hardware without Rosetta, or on Linux Wine.
* The 32-bit helper (`i686`): only type-checked. The 32-bit stdcall/cdecl
  trampoline, `st0` float returns and 32-bit float arguments have never run.
* Builds other than the Wine-derived `rust-lld` recipe above (the stock
  `*-pc-windows-gnu`/`msvc` toolchains were not available here).
* Calls with more than 16 arguments are refused; 8-argument and 16-argument
  calls were exercised, intermediate arities only by construction.
* A fault inside the called DLL (access violation) takes the helper down; the
  client reports `HelperCrashed` (verified for `ExitProcess`, not for a real
  access violation). There is no structured exception guard: recovering from an
  SEH fault inside an arbitrary foreign call cannot be done soundly from Rust, so
  this stays a documented limit.
* No `ByRef String` pointer-to-pointer or UDT marshaling over the bridge, no
  callbacks, no automatic `vba-ffi` plan to `WireArg` conversion.
* `GetLastError` is read straight after the call; this worked in every case
  tried under Wine, but nothing prevents the Rust runtime from clobbering it.
* The `--inject` and `--advertise-version` helper switches exist only for the
  tests.
