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

## Known gaps

* Collation (`lstrcmp*`) and case tables beyond the cases above.
* Best-fit conversions and code pages other than the four listed.
* Reported Windows version is a constant, not derived from a profile.
