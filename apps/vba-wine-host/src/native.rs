//! The real Windows call path (cfg(windows) only).
//!
//! UNVERIFIED: this module type-checks for `x86_64-pc-windows-msvc` but has
//! never been executed, under Wine or on Windows. Treat every behavior here as
//! a claim to be tested, not a fact.
//!
//! Limits, all reported as explicit errors rather than approximated:
//! * integer-class arguments and returns only (`F32`/`F64` need per-position
//!   register selection that a generic trampoline cannot do) -> `Unsupported`;
//! * at most [`MAX_CALL_ARGS`] arguments;
//! * a fault inside the called function (access violation, abort) kills this
//!   process; the client then reports `HelperCrashed`. There is no SEH guard.
//! * `GetLastError` is read right after the call; the Rust runtime could in
//!   principle clobber it in between, though nothing runs there.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_void};
use vba_wine::helper::Backend;
use vba_wine::protocol::{
    CAP_NATIVE_CALLS, HelloInfo, PROTOCOL_VERSION, ProcRef, RemoteError, RemoteErrorKind, RetKind,
    WireArg, WireConv, WireRet,
};

/// Most arguments the trampoline supports.
pub const MAX_CALL_ARGS: usize = 16;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    fn GetModuleHandleA(name: *const c_char) -> *mut c_void;
    fn GetLastError() -> u32;
    fn SetLastError(code: u32);
}

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
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr());
        if ntdll.is_null() {
            return String::new();
        }
        let p = GetProcAddress(ntdll, c"wine_get_version".as_ptr());
        if p.is_null() {
            return String::new();
        }
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
        let m = unsafe { LoadLibraryA(c.as_ptr()) };
        if m.is_null() {
            return Err(os_err(
                RemoteErrorKind::LibraryNotFound,
                &format!("LoadLibrary({name}) failed"),
            ));
        }
        Ok(m as usize as u64)
    }

    fn get_proc(&mut self, lib: u64, proc: &ProcRef) -> Result<u64, RemoteError> {
        let module = lib as usize as *mut c_void;
        let (p, what) = match proc {
            ProcRef::Name(n) => {
                let c = CString::new(n.as_str()).map_err(|_| fault("symbol contains NUL"))?;
                // SAFETY: `module` came from LoadLibraryA via the Server's id
                // table (never from the client); `c` is NUL-terminated.
                (unsafe { GetProcAddress(module, c.as_ptr()) }, n.clone())
            }
            ProcRef::Ordinal(o) => {
                if *o == 0 {
                    return Err(fault("ordinal 0 is invalid"));
                }
                // SAFETY: an ordinal is passed as a pointer whose value is the
                // ordinal (high word zero), per the GetProcAddress contract.
                (
                    unsafe { GetProcAddress(module, usize::from(*o) as *const c_char) },
                    format!("#{o}"),
                )
            }
        };
        if p.is_null() {
            return Err(os_err(
                RemoteErrorKind::SymbolNotFound,
                &format!("GetProcAddress({what}) failed"),
            ));
        }
        Ok(p as usize as u64)
    }

    fn call(
        &mut self,
        proc: u64,
        conv: WireConv,
        ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        if matches!(ret, RetKind::F32 | RetKind::F64) {
            return Err(RemoteError::new(
                RemoteErrorKind::Unsupported,
                "floating-point returns are not supported by the generic trampoline",
            ));
        }
        if args.len() > MAX_CALL_ARGS {
            return Err(fault(format!("more than {MAX_CALL_ARGS} arguments")));
        }
        let mut words = Vec::with_capacity(args.len());
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
                WireArg::F32(_) | WireArg::F64(_) => {
                    return Err(RemoteError::new(
                        RemoteErrorKind::Unsupported,
                        "floating-point arguments are not supported by the generic trampoline",
                    ));
                }
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
        let (raw, last_error) = unsafe {
            SetLastError(0);
            let raw = invoke(conv, proc as usize, &words);
            (raw, GetLastError())
        };
        let value = match ret {
            RetKind::Void => WireRet::Void,
            _ => WireRet::Int(raw as i64),
        };
        Ok((value, last_error))
    }

    fn free_library(&mut self, lib: u64) -> Result<(), RemoteError> {
        // SAFETY: `lib` came from LoadLibraryA via the Server's id table.
        if unsafe { FreeLibrary(lib as usize as *mut c_void) } == 0 {
            return Err(os_err(RemoteErrorKind::CallFault, "FreeLibrary failed"));
        }
        Ok(())
    }
}
