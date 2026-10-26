//! The real Windows call path (cfg(windows) only).
//!
//! Verification status is in docs/interop/wine-bridge.md: the x86-64 path has been
//! executed under Wine (exact configuration there); the 32-bit path and
//! genuine Windows hosts have not.
//!
//! Limits, all reported as explicit errors rather than approximated:
//! * floating-point arguments and returns work on x86-64 only (see
//!   [`invoke_float`]); on 32-bit they are `Unsupported`;
//! * at most [`MAX_CALL_ARGS`] arguments;
//! * a fault inside the called function (access violation, abort) kills this
//!   process; the client then reports `HelperCrashed`. There is no SEH guard.
//! * `GetLastError` is read right after the call; the Rust runtime could in
//!   principle clobber it in between, though nothing runs there.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char};
use vba_wine::helper::Backend;
use vba_wine::protocol::{
    CAP_NATIVE_CALLS, HelloInfo, PROTOCOL_VERSION, ProcRef, RemoteError, RemoteErrorKind, RetKind,
    WireArg, WireConv, WireRet,
};
use windows_sys::Win32::Foundation::{FreeLibrary, GetLastError, SetLastError};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};

/// Most arguments the trampoline supports.
pub const MAX_CALL_ARGS: usize = 16;

fn os_err(kind: RemoteErrorKind, what: &str) -> RemoteError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    RemoteError {
        kind,
        code,
        message: format!("{what} (Win32 error {code})"),
    }
}

fn fault(msg: impl Into<String>) -> RemoteError {
    RemoteError::new(RemoteErrorKind::CallFault, msg)
}

macro_rules! ty {
    ($_i:literal) => {
        usize
    };
}

/// Transmutes `addr` to a function of the arity of the listed indices with
/// ABI `$abi`, and calls it with the corresponding words.
macro_rules! trampoline {
    ($abi:literal, $addr:expr, $w:expr; $($n:literal => [$($i:literal),*]),* $(,)?) => {
        match $w.len() {
            $(
                $n => {
                    // SAFETY: see `invoke`; the caller guarantees `addr` is a
                    // procedure address obtained from GetProcAddress and that
                    // the word count matches the declared parameter count.
                    let f: unsafe extern $abi fn($(ty!($i)),*) -> usize =
                        unsafe { std::mem::transmute::<usize, _>($addr) };
                    unsafe { f($($w[$i]),*) }
                }
            )*
            _ => unreachable!("argument count checked by caller"),
        }
    };
}

/// Calls `addr` with integer-class `words`.
///
/// # Safety
/// `addr` must be the address of a function whose real signature takes exactly
/// `words.len()` pointer-sized integer-class arguments in the given
/// convention and returns an integer-class value, and every word that is a
/// pointer must point to memory that stays valid and large enough for the
/// duration of the call. Nothing here can check that; the caller (the VBA
/// `Declare` author, via the embedder's export table) is responsible.
unsafe fn invoke(conv: WireConv, addr: usize, words: &[usize]) -> usize {
    macro_rules! all_arities {
        ($abi:literal) => {
            trampoline!($abi, addr, words;
                0 => [], 1 => [0], 2 => [0, 1], 3 => [0, 1, 2], 4 => [0, 1, 2, 3],
                5 => [0, 1, 2, 3, 4], 6 => [0, 1, 2, 3, 4, 5],
                7 => [0, 1, 2, 3, 4, 5, 6], 8 => [0, 1, 2, 3, 4, 5, 6, 7],
                9 => [0, 1, 2, 3, 4, 5, 6, 7, 8], 10 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                11 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                12 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                13 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
                14 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                15 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
                16 => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            )
        };
    }
    match conv {
        // `system` is stdcall on x86 Windows and the single convention on x64.
        WireConv::Stdcall => all_arities!("system"),
        WireConv::Cdecl => all_arities!("C"),
    }
}

/// Calls `addr` with arguments that may include `f32`/`f64`, x86-64 only.
///
/// The Win64 convention selects the register of argument `i < 4` by its
/// type (`rcx, rdx, r8, r9` or `xmm0..xmm3`), which a generic trampoline
/// cannot know. A *variadic* call, however, makes the caller place every
/// `double` among the first four arguments in both the integer and the XMM
/// register of its position (the callee of a variadic function cannot know
/// the types). Passing every word as an `f64` with the same bits therefore
/// fills both registers of each position, so a non-variadic callee finds an
/// integer in `rcx..r9` or a float in `xmm0..xmm3`, whichever it reads. An
/// `f32` is passed as its bits in the low half of the word (the callee reads
/// only the low 32 bits of the XMM register). Arguments beyond the fourth go
/// to the stack as 8-byte slots in both conventions.
///
/// # Safety
/// As for [`invoke`]; additionally the real signature must only use
/// integer-class, `f32` and `f64` parameters, and `ret` selects the register
/// (`rax` or `xmm0`) the result is read from.
#[cfg(target_arch = "x86_64")]
unsafe fn invoke_float(conv: WireConv, addr: usize, words: &[u64], ret: RetKind) -> WireRet {
    // Win64 has one convention, so `conv` does not matter here.
    let _ = conv;
    macro_rules! call_as {
        ($r:ty, $($n:literal => [$($i:literal),*]),* $(,)?) => {
            match words.len() {
                0 => {
                    // SAFETY: see function contract; no arguments.
                    let f: unsafe extern "C" fn() -> $r = unsafe { std::mem::transmute::<usize, _>(addr) };
                    unsafe { f() }
                }
                $(
                    $n => {
                        // SAFETY: see function contract; variadic call through a
                        // pointer is how the dual register placement is obtained.
                        let f: unsafe extern "C" fn(f64, ...) -> $r =
                            unsafe { std::mem::transmute::<usize, _>(addr) };
                        unsafe { f(f64::from_bits(words[0]) $(, f64::from_bits(words[$i]))*) }
                    }
                )*
                _ => unreachable!("argument count checked by caller"),
            }
        };
    }
    macro_rules! arities {
        ($r:ty) => {
            call_as!($r,
                1 => [], 2 => [1], 3 => [1, 2], 4 => [1, 2, 3], 5 => [1, 2, 3, 4],
                6 => [1, 2, 3, 4, 5], 7 => [1, 2, 3, 4, 5, 6], 8 => [1, 2, 3, 4, 5, 6, 7],
                9 => [1, 2, 3, 4, 5, 6, 7, 8], 10 => [1, 2, 3, 4, 5, 6, 7, 8, 9],
                11 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                12 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                13 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
                14 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                15 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
                16 => [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            )
        };
    }
    match ret {
        RetKind::F64 => WireRet::Float(arities!(f64)),
        RetKind::F32 => WireRet::Float(f64::from(arities!(f32))),
        RetKind::Int => WireRet::Int(arities!(usize) as i64),
        RetKind::Void => {
            let _ = arities!(usize);
            WireRet::Void
        }
    }
}

fn int_value(ret: RetKind, raw: usize) -> WireRet {
    match ret {
        RetKind::Void => WireRet::Void,
        _ => WireRet::Int(raw as i64),
    }
}

/// Loads real libraries in this process.
pub struct Native;

impl Native {
    pub fn new() -> Self {
        Native
    }
}

fn wine_version() -> String {
    // SAFETY: the name literals are NUL-terminated; `wine_get_version`, when
    // present in ntdll, is `const char *wine_get_version(void)` returning a
    // static NUL-terminated string.
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        if ntdll.is_null() {
            return String::new();
        }
        let Some(p) = GetProcAddress(ntdll, c"wine_get_version".as_ptr().cast()) else {
            return String::new();
        };
        let f: unsafe extern "C" fn() -> *const c_char = std::mem::transmute(p);
        let s = f();
        if s.is_null() {
            String::new()
        } else {
            CStr::from_ptr(s).to_string_lossy().into_owned()
        }
    }
}

impl Backend for Native {
    fn info(&self) -> HelloInfo {
        HelloInfo {
            version: PROTOCOL_VERSION,
            bitness: if cfg!(target_pointer_width = "32") {
                32
            } else {
                64
            },
            os: "windows".to_string(),
            wine_version: wine_version(),
            helper: format!("vba-wine-host {}", env!("CARGO_PKG_VERSION")),
            caps: CAP_NATIVE_CALLS,
        }
    }

    fn load_library(&mut self, name: &str) -> Result<u64, RemoteError> {
        let c = CString::new(name).map_err(|_| fault("library name contains NUL"))?;
        // SAFETY: `c` is a valid NUL-terminated string for the call.
        let m = unsafe { LoadLibraryA(c.as_ptr().cast()) };
        if m.is_null() {
            return Err(os_err(
                RemoteErrorKind::LibraryNotFound,
                &format!("LoadLibrary({name}) failed"),
            ));
        }
        Ok(m as usize as u64)
    }

    fn get_proc(&mut self, lib: u64, proc: &ProcRef) -> Result<u64, RemoteError> {
        let module = lib as usize as windows_sys::Win32::Foundation::HMODULE;
        let (p, what) = match proc {
            ProcRef::Name(n) => {
                let c = CString::new(n.as_str()).map_err(|_| fault("symbol contains NUL"))?;
                // SAFETY: `module` came from LoadLibraryA via the Server's id
                // table (never from the client); `c` is NUL-terminated.
                (
                    unsafe { GetProcAddress(module, c.as_ptr().cast()) },
                    n.clone(),
                )
            }
            ProcRef::Ordinal(o) => {
                if *o == 0 {
                    return Err(fault("ordinal 0 is invalid"));
                }
                // SAFETY: an ordinal is passed as a pointer whose value is the
                // ordinal (high word zero), per the GetProcAddress contract.
                (
                    unsafe { GetProcAddress(module, usize::from(*o) as *const u8) },
                    format!("#{o}"),
                )
            }
        };
        let Some(p) = p else {
            return Err(os_err(
                RemoteErrorKind::SymbolNotFound,
                &format!("GetProcAddress({what}) failed"),
            ));
        };
        Ok(p as usize as u64)
    }

    fn call(
        &mut self,
        proc: u64,
        conv: WireConv,
        ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        if args.len() > MAX_CALL_ARGS {
            return Err(fault(format!("more than {MAX_CALL_ARGS} arguments")));
        }
        let uses_float = matches!(ret, RetKind::F32 | RetKind::F64)
            || args
                .iter()
                .any(|a| matches!(a, WireArg::F32(_) | WireArg::F64(_)));
        if uses_float && cfg!(not(target_arch = "x86_64")) {
            return Err(RemoteError::new(
                RemoteErrorKind::Unsupported,
                "floating-point arguments and returns need the x86-64 helper",
            ));
        }
        let mut words: Vec<usize> = Vec::with_capacity(args.len());
        for (i, a) in args.iter_mut().enumerate() {
            words.push(match a {
                WireArg::Int(v) => {
                    if cfg!(target_pointer_width = "32")
                        && i32::try_from(*v).is_err()
                        && u32::try_from(*v).is_err()
                    {
                        return Err(fault(format!("argument {i} does not fit 32 bits")));
                    }
                    *v as usize
                }
                // Bit patterns; only reachable on x86-64 (checked above), where
                // usize is 64 bits.
                WireArg::F32(f) => f.to_bits() as usize,
                WireArg::F64(f) => f.to_bits() as usize,
                // The Vec outlives the call; its heap block does not move.
                WireArg::BufIn(b) | WireArg::BufInOut(b) => b.as_mut_ptr() as usize,
                WireArg::BufOut(_) => return Err(fault("BufOut must be expanded by the server")),
            });
        }
        // SAFETY: `proc` is an address returned by GetProcAddress, mapped from
        // an opaque id by the Server (the client cannot supply addresses). That
        // the real function matches the declared arity/convention is the
        // embedder's export-table contract and cannot be verified here.
        // `words` holds integers and pointers into `args`' vectors, which are
        // alive and not otherwise borrowed until after the call returns.
        let (value, last_error) = unsafe {
            SetLastError(0);
            #[cfg(target_arch = "x86_64")]
            let value = if uses_float {
                let w: Vec<u64> = words.iter().map(|&x| x as u64).collect();
                invoke_float(conv, proc as usize, &w, ret)
            } else {
                int_value(ret, invoke(conv, proc as usize, &words))
            };
            #[cfg(not(target_arch = "x86_64"))]
            let value = int_value(ret, invoke(conv, proc as usize, &words));
            (value, GetLastError())
        };
        Ok((value, last_error))
    }

    fn free_library(&mut self, lib: u64) -> Result<(), RemoteError> {
        // SAFETY: `lib` came from LoadLibraryA via the Server's id table.
        if unsafe { FreeLibrary(lib as usize as windows_sys::Win32::Foundation::HMODULE) } == 0 {
            return Err(os_err(RemoteErrorKind::CallFault, "FreeLibrary failed"));
        }
        Ok(())
    }
}
