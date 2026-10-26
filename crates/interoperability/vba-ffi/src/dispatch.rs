//! The only module of this crate that contains `unsafe` code (feature
//! `native-call`).
//!
//! It does exactly two things: open a shared library and resolve a symbol
//! (`libloading`), and perform one foreign call whose argument and return
//! classes were derived from a validated `Declare` descriptor (`libffi`).
//! Everything else (permission, ABI refusal, marshaling, buffer ownership,
//! write-back) is safe code in `native.rs`.
//!
//! Pointers arrive here as plain addresses of buffers the caller owns and keeps
//! alive for the whole call; emulated-memory addresses never reach this module.
#![allow(unsafe_code)]

use libffi::middle::{Arg, Cif, CodePtr, Ret, Type};
use std::ffi::c_void;

/// The machine class of an integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntClass {
    U8,
    I16,
    I32,
    I64,
}

/// One argument value with its machine class.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Val {
    U8(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    /// The address of a real buffer owned by the caller of [`call`].
    Ptr(usize),
}

/// The class of the return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetClass {
    Void,
    Int(IntClass),
    F32,
    F64,
}

/// A return value, already normalized to the declared class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum RetVal {
    Void,
    Int(i64),
    F32(f32),
    F64(f64),
}

/// Why the library or symbol could not be resolved.
#[derive(Debug)]
pub(crate) enum ResolveError {
    Library(String),
    Symbol(String),
}

fn class_type(v: &Val) -> Type {
    match v {
        Val::U8(_) => Type::u8(),
        Val::I16(_) => Type::i16(),
        Val::I32(_) => Type::i32(),
        Val::I64(_) => Type::i64(),
        Val::F32(_) => Type::f32(),
        Val::F64(_) => Type::f64(),
        Val::Ptr(_) => Type::pointer(),
    }
}

/// Loads `library`, resolves the first of `symbols` that exists and calls it
/// once with `args` using the platform's default C ABI.
///
/// This is a crate-private function, deliberately not `unsafe` so that the
/// `unsafe` token stays inside this module. Its contract, upheld by
/// `native.rs`: the caller asserts that the resolved function really has the signature
/// described by `args` and `ret` (the host's permission grant is that
/// assertion), and that every `Val::Ptr` address points to a live, writable
/// buffer of the size the function expects, not accessed by anyone else until
/// this function returns.
pub(crate) fn call(
    library: &str,
    symbols: &[String],
    args: &[Val],
    ret: RetClass,
) -> Result<RetVal, ResolveError> {
    // SAFETY: loading a library runs its initializers; the host granted this
    // library by name through `NativePermission`.
    let lib = unsafe { libloading::Library::new(library) }
        .map_err(|e| ResolveError::Library(e.to_string()))?;
    let mut found = None;
    for s in symbols {
        // SAFETY: the symbol is only used as an opaque code address below; its
        // declared type here is a plain pointer.
        if let Ok(sym) = unsafe { lib.get::<*const c_void>(s.as_bytes()) } {
            found = Some(*sym);
            break;
        }
    }
    let Some(addr) = found else {
        return Err(ResolveError::Symbol(symbols.join(", ")));
    };

    // Pointer values need typed storage that outlives the argument list.
    let ptrs: Vec<*mut c_void> = args
        .iter()
        .map(|a| match a {
            Val::Ptr(p) => std::ptr::with_exposed_provenance_mut(*p),
            _ => std::ptr::null_mut(),
        })
        .collect();
    let list: Vec<Arg> = args
        .iter()
        .zip(&ptrs)
        .map(|(a, p)| match a {
            Val::U8(v) => Arg::new(v),
            Val::I16(v) => Arg::new(v),
            Val::I32(v) => Arg::new(v),
            Val::I64(v) => Arg::new(v),
            Val::F32(v) => Arg::new(v),
            Val::F64(v) => Arg::new(v),
            Val::Ptr(_) => Arg::new(p),
        })
        .collect();
    let ret_type = match ret {
        RetClass::Void => Type::void(),
        RetClass::Int(IntClass::U8) => Type::u8(),
        RetClass::Int(IntClass::I16) => Type::i16(),
        RetClass::Int(IntClass::I32) => Type::i32(),
        RetClass::Int(IntClass::I64) => Type::i64(),
        RetClass::F32 => Type::f32(),
        RetClass::F64 => Type::f64(),
    };
    let cif = Cif::new(args.iter().map(class_type), ret_type);
    let fun = CodePtr(addr.cast_mut());

    // libffi writes integer returns narrower than a machine word as a full
    // word, so integer results go through a zero-initialized 8-byte cell.
    let mut word: u64 = 0;
    let mut single: f32 = 0.0;
    let mut double: f64 = 0.0;
    // SAFETY: `cif` was built from exactly the classes of `args`/`ret`; the
    // caller guarantees the function matches them and that pointer arguments
    // are valid. The result cells are at least as large as the return class.
    unsafe {
        match ret {
            RetClass::F32 => cif.call_return_into(fun, &list, Ret::new(&mut single)),
            RetClass::F64 => cif.call_return_into(fun, &list, Ret::new(&mut double)),
            _ => cif.call_return_into(fun, &list, Ret::new(&mut word)),
        }
    }
    // The library stays loaded until after the call returned.
    drop(lib);
    Ok(match ret {
        RetClass::Void => RetVal::Void,
        RetClass::Int(IntClass::U8) => RetVal::Int(i64::from(word as u8)),
        RetClass::Int(IntClass::I16) => RetVal::Int(i64::from(word as i16)),
        RetClass::Int(IntClass::I32) => RetVal::Int(i64::from(word as i32)),
        RetClass::Int(IntClass::I64) => RetVal::Int(word as i64),
        RetClass::F32 => RetVal::F32(single),
        RetClass::F64 => RetVal::F64(double),
    })
}
