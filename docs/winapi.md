# vba-winapi: Win32 catalog and provider framework

Crate: `crates/windows/vba-winapi` (depends on `vba-ffi`). The source of truth
for status is `crates/windows/vba-winapi/catalog.csv`; tests keep it in step
with the code.

## Framework

* `Provider` trait (`id`, `exports`, `call`) and `Registry`. `Registry::resolve_declare`
  turns a validated `Declare` into a provider and exported symbol using the
  `vba-ffi` entry candidates. Library names are normalized (directory and
  `.dll` dropped, lower case); symbols are case sensitive like `GetProcAddress`.
* Failures to resolve are always `WinApiError::Unsupported { symbol, reason }`.
  vba-rs cannot know whether the real DLL exports a name, so it never claims
  "DLL not found". Ordinal aliases and non-Windows targets are `Unsupported`.
* `CallContext` carries everything a shim may touch: injected clock/sleep
  (`HostEnv`, `ManualClock`, `SystemClock`), injected `Identity`, reported
  `WindowsVersion` (Windows 10.0.19045 by default, host-overridable; the
  vba-profile target holds only OS family and bitness), the system ANSI code
  page (Windows-1252 by default), the emulated address space, the handle table
  and per-logical-thread last-error.
* Win32 failures reported by return value plus `GetLastError` are ordinary
  results; `WinApiError` is only for what the framework refuses to do. Memory
  faults (null, unmapped, out of range) are `WinApiError::Memory`, since real
  Windows would crash or corrupt memory.
* Tests: `tests/shims.rs` (table-driven, cases counted per symbol against the
  catalog), `tests/registry.rs` (catalog/registry exact match, affinity,
  Declare end-to-end, property tests).

## Implemented shims (all `native-shim`, portable to every host OS)

Per-function reference tests are the `tests` column in the catalog.

| Symbol | Behavior and source |
| --- | --- |
| kernel32 `GetTickCount`, `GetTickCount64` | Injected clock; 32-bit wrap for the first. sysinfoapi docs. |
| kernel32 `QueryPerformanceCounter`, `QueryPerformanceFrequency` | Write a `LARGE_INTEGER`, return TRUE. profileapi docs. |
| kernel32 `Sleep` | Injected; `INFINITE` is `Unsupported` instead of hanging. synchapi docs. |
| kernel32 `lstrlenA/W`, `lstrcpyA/W`, `lstrcatA/W` | NULL handling per winbase docs (length 0, result NULL); overflow is a memory fault. |
| kernel32 `lstrcmpA/W`, `lstrcmpiA/W` | Exact only for ASCII letters and digits under en-US word sort (digits < letters, lowercase before uppercase on ties for `lstrcmp`). Anything else: `Unsupported`. |
| user32 `CharUpperA/W`, `CharLowerA/W` | High word zero = single character, else in-place string. Uses Rust simple Unicode mappings and the context ANSI code page; characters without a one-to-one mapping stay unchanged. Windows tables may differ for rare characters (gap). |
| kernel32 `MultiByteToWideChar`, `WideCharToMultiByte` | Code pages 1252, 20127, 28591, 65001, `CP_ACP`, `CP_THREAD_ACP`. Size queries, `-1` lengths, errors 87/122/1113, `MB_ERR_INVALID_CHARS`, `WC_ERR_INVALID_CHARS`, `WC_NO_BEST_FIT_CHARS`, default char and used-default flag. Other code pages, composite/glyph flags, and best-fit substitution are `Unsupported`; US-ASCII bytes above 0x7F without `MB_ERR_INVALID_CHARS` are `Unsupported`. |
| kernel32 `GetLastError`, `SetLastError` | Emulated per logical thread. |
| kernel32 `GetVersion`, `GetVersionExA` | From the context. `OSVERSIONINFOA` (148) and `OSVERSIONINFOEXA` (156) written using the vba-ffi UDT layout; other sizes set error 87 (the documentation does not specify the code; choice recorded). |
| advapi32 `GetUserNameA`, kernel32 `GetComputerNameA` | Injected identity. Buffer-too-small: 0 with 122 and 111 respectively and the required size written back. |
| kernel32 `RtlMoveMemory`, `RtlFillMemory`, `RtlZeroMemory` | Bounds-checked, `memmove` semantics. `CopyMemory`/`FillMemory`/`ZeroMemory` are SDK macros, not exports, and are cataloged as not implemented; VBA reaches these through `Alias "RtlMoveMemory"` etc. |

## Not implemented

Everything else in the catalog marked `NOT IMPLEMENTED` (wide variants of
`GetVersionEx`/`GetUserName`/`GetComputerName`, wall-clock and process
functions, loader functions, file, registry, UI and shell APIs, `Beep`), and any
symbol not in the catalog. `Beep` is deliberately unsupported: a silent no-op
would be a fake success. `wine-fallback` rows are a plan only; no Wine
integration exists.

## Win32 function inventory

`crates/windows/vba-winapi/data/win32-functions.csv` lists every function
Microsoft's Win32 metadata declares as a DLL export. It is generated, never
hand-edited:

```sh
cargo run -p vba-winapi-gen --release   # rewrites the data file
```

`apps/vba-winapi-gen` is a dev tool (not a shipped library). It reads the
`Windows.Win32.winmd` bundled in the `windows-default` crate (version pinned in
the root `Cargo.toml`, `0.100.0` when generated) with `windows-metadata`, both
from Microsoft's windows-rs repository and licensed `MIT OR Apache-2.0`
(permissive, compatible with this repository's Apache-2.0). The metadata itself
ships inside that crate under the same license. Output is deterministic for a
given version.

Columns: `dll,symbol,base,charset,pair,params,ret,conv,arch`. `charset`/`pair`
carry the A/W pairing (`lstrcpyA` is `base=lstrcpy`, `charset=A`, `pair=AW`);
`params` is `name:direction:type` joined by `|` (direction `in`/`out`/`inout`/`-`,
`?` suffix for optional); `conv` is `system` (stdcall) or `C`; `arch` is `all`
or the architectures the metadata restricts the function to.
There is no `SetLastError` column: the metadata bundled with windows-rs does not
carry that flag, so none could be reported truthfully.

Size: 14 567 rows (14 494 distinct `dll!symbol`, 226 DLLs), 2.2 MB (about 310 KB
gzipped), under the 5 MB threshold, so it stays a single file.

Coverage (inventory vs implemented):

| | Count |
| --- | --- |
| Inventory functions (distinct `dll!symbol`) | 14 494 |
| Implemented shim exports (registry) | 30 |
| of which present in the inventory | 27 |
| Coverage of the inventory | 0.2 % |

The three implemented exports absent from the inventory are
`kernel32!RtlMoveMemory/RtlFillMemory/RtlZeroMemory`: real exports that the SDK
headers expose as macros, so the metadata omits them (as it does the
`CopyMemory`/`FillMemory`/`ZeroMemory` macros that the catalog lists as not
implemented). `tests/inventory.rs` names exactly these exemptions.

`catalog.csv` stays the implemented-shim catalog; `tests/inventory.rs` checks
that every cataloged DLL/symbol exists in the inventory, that declared
parameter counts and A/W variants agree with the metadata, and that every
registered shim is a known export. `vba_winapi::inventory::lookup(dll, symbol)`
exposes entries read-only. A `Declare` of an unimplemented function the
inventory knows fails with `Unsupported` whose reason adds `known Win32 export
per Microsoft's metadata, signature Sleep(dwMilliseconds: in u32) -> void`
(or names the DLL the metadata places it in); unknown names keep the plain
"no registered provider" reason.

## Known gaps

* Collation (`lstrcmp*`) and case tables beyond the cases above.
* Best-fit conversions and code pages other than the four listed.
* Reported Windows version is a constant, not derived from a profile.
