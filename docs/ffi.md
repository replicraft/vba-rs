# vba-ffi: the `Declare` ABI layer

Crate: `crates/interoperability/vba-ffi`. No `unsafe`, no host pointers, no
library loading. Everything below is tested in `tests/ffi.rs`.

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
| Native call stage | `call_native` requires a host-issued `NativePermission` naming the library, checks the declared target equals the host, and then returns `NativeCallError::Unavailable` ("NativeCallUnavailable"). Feature `native-call` (off by default) only changes the reported reason. | Decision below. |

## Not implemented (fails visibly)

* Variant arguments: `FfiError::Unsupported("Variant arguments")`.
* Variable-length `String` members inside UDTs, `Object` parameters, `ByVal` UDT/Variant.
* Fixed-length string write-back with DBCS code pages.
* Arrays of non-scalar, non-UDT elements; multi-dimensional descriptor (SAFEARRAY) passing.
* Any actual dynamic call into native code.

## Decision: no libloading/libffi

A real call needs `unsafe` (loading a library, transmuting a symbol, a variadic
trampoline) that cannot be exercised portably or safely in this repository's
test environment (no Windows library to call on CI hosts, and the declared
target frequently differs from the host). Rather than ship an untested unsafe
path, the validated descriptor, entry candidates and marshaled argument vector
are produced in full and the final step is an explicit error. The
`vba-ffi` crate therefore keeps `unsafe_code = "forbid"`. When a backend is
added it must live behind `native-call`, take the `NativePermission`, check the
descriptor's target against the host, and carry SAFETY comments.

## Known gaps / assumptions to verify on real VBA

* UDT alignment and `ByRef String` pointer-to-pointer semantics are
  community-documented, not specified by MS-VBAL.
* `Err.LastDllError` capture after each `Declare` call is the runtime's job
  (it reads `LastErrorStore`); it is not wired here.
