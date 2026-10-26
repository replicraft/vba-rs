# vba-ffi: the `Declare` ABI layer

Crate: `crates/interoperability/vba-ffi`. By default there is no `unsafe`, no
host pointers and no library loading. The optional `native-call` feature adds a
real call stage (see "Native call stage (feature)"). Default-feature behavior is
tested in `tests/ffi.rs`; the feature is tested in `tests/native_call.rs`.

## Implemented

| Area | What it does | Provenance / confidence |
| --- | --- | --- |
| Signature descriptor (`DeclareSpec` -> `ValidatedDeclare`) | Library, alias, `PtrSafe`, charset, calling convention, parameters (`ByVal`/`ByRef`), scalar return. Only validated descriptors reach later stages. | MS-VBAL 5.2.3.5 external procedure declarations. |
| Target semantics | Pointer width, `LongPtr`/`LongLong`, `PtrSafe` requirement and entry decoration come from the `vba-profile` `Target`, never the host. 64-bit requires `PtrSafe`; `LongLong` only on 64-bit; Win16 rejected. | MS-VBAL 3.3.2 (LongLong), VBA7 `PtrSafe` rule. |
| Rejected combinations | `ByVal` UDT, `ByVal` array, `ByVal` Variant, non-scalar return, bad `#n` alias, > 60 parameters, UDT laid out for another target. | MS-VBAL; Variant/array limits are vba-rs policy where stated. |
| Entry selection | `#n` alias = ordinal; explicit alias is exact; otherwise ANSI tries `Name`, `NameA`; Unicode tries `NameW`, `Name`; 32-bit stdcall appends `_Name@N`. | Observed VBA behavior, not specified by MS-VBAL. Medium confidence. |
| Calling convention | Recorded; on 64-bit both collapse (`effective_conv` is `None`). | x64 single convention. |
| UDT layout | Natural alignment capped at 8, nested types align to their widest member, size rounded to alignment, zero padding, little-endian, `String * n` inline ANSI padded with spaces, `LongPtr` = target pointer width. Encode/decode with range checks. | Community-documented VBA behavior (Appleman); not in MS-VBAL. Medium confidence. |
| Code pages (`codec`) | US-ASCII 20127, ISO-8859-1 28591, Windows-1252, UTF-8 65001. Unmappable text is rejected or replaced with an explicit default byte. **Best-fit tables are not reproduced.** | Code page tables; `WideCharToMultiByte` documentation. |
| Emulated memory | Integer addresses (above 0xFFFF) into bounds-checked regions; 32-bit targets get a 4 GiB space, 64-bit 2^47; allocation budget; `memmove` copy that validates both ranges before writing. | vba-rs design. |
| Marshaling plan | `ByVal` scalar by value; `ByRef` scalar through a temporary cell with write-back; `String` (both modes) becomes a NUL-terminated ANSI buffer and the **whole** buffer is converted back; `ByRef String` passes the address of a cell holding the buffer pointer; `As Any` (string, bytes, `ByVal Long`); UDT and arrays as pointers to encoded buffers with write-back. Return narrowing (a `DWORD` 0xFFFFFFFF declared `Long` is -1). | Declare statement documentation; ByVal-String write-back and ByRef-String pointer-to-pointer are widely documented behavior, medium confidence. |
| Handles | Opaque `Handle` (private fields), minted only by `HandleTable`; each token records its provider; another provider presenting it (as `Handle` or bare integer), a foreign table, a closed or unknown token are all rejected. Tokens never reused. | vba-rs design. |
| Last error | `LastErrorStore` per `LogicalThread`; the host's value is never read. | `GetLastError` documentation. |
| Native call stage | Default build: `call_native` requires a host-issued `NativePermission`, checks the declared target equals the claimed host, then returns `NativeCallError::Unavailable` ("NativeCallUnavailable"). With feature `native-call`: really loads and calls, see below. | Section below. |

## Not implemented (fails visibly)

* Variant arguments: `FfiError::Unsupported("Variant arguments")`.
* Variable-length `String` members inside UDTs, `Object` parameters, `ByVal` UDT/Variant.
* Fixed-length string write-back with DBCS code pages.
* Arrays of non-scalar, non-UDT elements; multi-dimensional descriptor (SAFEARRAY) passing.
* Any dynamic call into native code in the default build (feature
  `native-call` off).

## Native call stage (feature)

Feature `native-call`, off by default. Dependencies: `libloading` (library load,
symbol lookup) and `libffi` (the `middle` API: a `Cif` built from the argument
classes at run time). libffi was chosen over a hand-written transmute
trampoline because the descriptor decides the classes at run time, and libffi
does the per-position integer/SSE/stack assignment (including `f32`/`f64` and
narrow integer returns) that a generic trampoline cannot.

Entry points: `call_prepared(permission, decl, memory, &prepared)` for any
marshaled call (then `PreparedCall::finish` decodes the written-back memory),
and `call_native(permission, decl, host, args)` for declarations whose
parameters are all `ByVal` scalars (anything taking memory is refused there
with `Unsupported`).

Checks, in order: host-issued `NativePermission` for the library name
(`PermissionDenied`); declared target must equal the target this host can load
(`host_target()`: Windows on Windows, otherwise the Unix-ABI target
`OperatingSystem::MacOs`, with the host's pointer width), else `AbiMismatch`;
declared convention must be dispatchable, else `ConventionMismatch`; argument
count (`ArgumentCount`) and per-argument machine class (`ArgumentClass`: a
`Byte` must fit 8 bits, a `Single` needs `NativeArg::F32`); library
(`LibraryNotFound`) and entry candidates (`SymbolNotFound`); `#n` ordinals are
`OrdinalUnsupported`.

Memory model: emulated addresses are never passed as pointers. The marshaling
plan (`PreparedCall::native_plan`) says which arguments are values, memory
regions or `ByRef String` cells. Every region is copied into a 16-byte-aligned
buffer owned by the call stage, the real address is passed, and after the call
each buffer is copied back into emulated memory. A `ByRef String` cell is
rebuilt to hold the real text address; if the callee overwrites that pointer
the call fails with `PointerCellChanged` instead of following it. If the call
fails nothing is written back.

`unsafe` is confined to the private `dispatch` module
(`#![allow(unsafe_code)]`, SAFETY comments). The crate sets
`unsafe_code = "deny"` (not `forbid`, which cannot be relaxed per module) in its
own `[lints]`. Without the feature that module is not compiled and the crate
contains no `unsafe`.

Exercised on this host (macOS arm64, 64-bit Unix ABI), through descriptors in
`tests/native_call.rs`: `strlen` (ByVal String, `LongPtr` return), `abs` and
`getpid` (`Long`), `sqrt` (`Double`), `fabsf` (`Single`), `frexp` (`ByRef Long`
write-back), `strcpy` into a `ByRef As Any` byte buffer and into a pre-sized
`ByVal String`, `strtol` rewriting a `ByRef String` cell (rejected); missing
library, missing symbol, wrong argument count and class, denied permission,
Windows-x64 and 32-bit declarations refused with `AbiMismatch`.
Run with `cargo test -p vba-ffi --features native-call`.

Not exercised / gaps:

* Windows hosts and the Windows x64/x86 ABIs (never run on Windows; Windows
  DLLs under Wine are the `vba-wine` bridge's job, see wine-bridge.md).
* 32-bit hosts. 32-bit `stdcall` is refused with `ConventionMismatch` on every
  host because only libffi's platform default ABI is selected; x86 stdcall
  dispatch is not implemented. The refusal rule is unit-tested; no real 32-bit
  call was run.
* Linux was not run (tests would use `libc.so.6`/`libm.so.6`; only macOS was
  available).
* Variadic C functions (such as `snprintf`) cannot be declared in VBA and are
  not supported. UDT and array marshaling goes through the same region code but
  has no native test.
* A fault in the callee (bad pointer, abort) takes the process down; there is
  no guard.

## Known gaps / assumptions to verify on real VBA

* UDT alignment and `ByRef String` pointer-to-pointer semantics are
  community-documented, not specified by MS-VBAL.
* `Err.LastDllError` capture after each `Declare` call is the runtime's job
  (it reads `LastErrorStore`); it is not wired here.
