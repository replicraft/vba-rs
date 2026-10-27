//! The dynamic native call stage.
//!
//! Without the `native-call` cargo feature (the default) this stage validates
//! the request and reports [`NativeCallError::Unavailable`]; it never loads
//! anything and the crate contains no reachable `unsafe` code.
//!
//! With the feature, `call_prepared` (and [`call_native`] for declarations
//! whose parameters are all `ByVal` scalars) really loads the library
//! (`libloading`) and performs the call (`libffi`):
//!
//! * a [`NativePermission`] issued by the host is required in every
//!   configuration, and no VBA-reachable API creates one;
//! * the *validated descriptor* decides every argument class; the marshaling
//!   plan decides which arguments are memory;
//! * emulated-memory addresses are never passed as real pointers. Each memory
//!   argument is copied into a real, aligned buffer owned by the call stage for
//!   exactly the duration of the call, the real address is passed, and the
//!   buffer is copied back into emulated memory afterwards;
//! * the declared target must be the target this host can actually load
//!   ([`host_target`]) and the declared convention must be one this host can
//!   dispatch, otherwise the call is refused with a distinct error.

use crate::{NativeArg, NativeReturn, ValidatedDeclare};
use thiserror::Error;
use vba_profile::{Bitness, OperatingSystem, Target};

/// Why the native call stage did not run a call.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeCallError {
    /// The permission token does not allow this library.
    #[error("host grant does not allow native library `{library}`")]
    PermissionDenied {
        /// Requested library.
        library: String,
    },
    /// The declaration's target is not the machine's, so it cannot be run.
    #[error("declaration targets {declared:?} but the host is {host:?}")]
    TargetMismatch {
        /// Declared target.
        declared: Target,
        /// Host target.
        host: Target,
    },
    /// The declared target ABI differs from the ABI this host can load.
    #[error("declared target ABI {declared:?} differs from the host's loadable ABI {loadable:?}")]
    AbiMismatch {
        /// Declared target.
        declared: Target,
        /// The only target this host can load libraries for.
        loadable: Target,
    },
    /// The declared calling convention cannot be dispatched on this host.
    #[error("calling convention {declared:?} cannot be dispatched: {reason}")]
    ConventionMismatch {
        /// Declared convention.
        declared: crate::CallConv,
        /// Why.
        reason: &'static str,
    },
    /// The number of supplied arguments differs from the declaration.
    #[error("declaration has {expected} parameters but {got} arguments were supplied")]
    ArgumentCount {
        /// Declared parameter count.
        expected: usize,
        /// Supplied argument count.
        got: usize,
    },
    /// An argument does not have the machine class its parameter requires.
    #[error("argument {index} does not fit its declared class: {reason}")]
    ArgumentClass {
        /// Zero-based argument index.
        index: usize,
        /// What is wrong.
        reason: String,
    },
    /// The declaration cannot be called through this entry point.
    #[error("unsupported native call: {reason}")]
    Unsupported {
        /// What is not supported.
        reason: &'static str,
    },
    /// The library could not be loaded.
    #[error("cannot load native library `{library}`: {reason}")]
    LibraryNotFound {
        /// Requested library.
        library: String,
        /// Loader message.
        reason: String,
    },
    /// None of the entry candidates exists in the library.
    #[error("native library `{library}` has no entry point {symbols}")]
    SymbolNotFound {
        /// Library.
        library: String,
        /// Candidates tried, comma separated.
        symbols: String,
    },
    /// Ordinal entry points (`Alias "#n"`) cannot be resolved here.
    #[error("ordinal entry points are not supported by the native call stage")]
    OrdinalUnsupported,
    /// Emulated memory could not be read or written around the call.
    #[error("emulated memory fault around the native call: {0}")]
    Memory(String),
    /// The callee replaced the pointer held in a `ByRef String` cell.
    #[error("the callee changed the string pointer of argument {index}; refusing to follow it")]
    PointerCellChanged {
        /// Zero-based argument index.
        index: usize,
    },
    /// No native backend is available: `NativeCallUnavailable`.
    #[error("NativeCallUnavailable: {reason}")]
    Unavailable {
        /// Human-readable reason.
        reason: &'static str,
    },
}

/// A host-issued grant to call native code in specific libraries.
///
/// Only host code constructs this; the `Declare` machinery merely demands one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePermission {
    libraries: Vec<String>,
}

impl NativePermission {
    /// Grants calls into exactly these libraries (case-insensitive names).
    pub fn grant<I: IntoIterator<Item = String>>(libraries: I) -> Self {
        NativePermission {
            libraries: libraries
                .into_iter()
                .map(|l| l.to_ascii_lowercase())
                .collect(),
        }
    }

    /// Whether `library` is covered.
    pub fn allows(&self, library: &str) -> bool {
        let l = library.to_ascii_lowercase();
        self.libraries.contains(&l)
    }
}

/// The target whose ABI this build can load libraries for.
///
/// Windows hosts map to [`OperatingSystem::Windows`]; every other host is a
/// Unix-ABI machine and maps to [`OperatingSystem::MacOs`], the only non-Windows
/// target VBA has. Bitness is the host's pointer width.
pub fn host_target() -> Target {
    Target {
        os: if cfg!(windows) {
            OperatingSystem::Windows
        } else {
            OperatingSystem::MacOs
        },
        bitness: if cfg!(target_pointer_width = "64") {
            Bitness::B64
        } else {
            Bitness::B32
        },
    }
}

/// Attempts a native call whose arguments are all passed by value.
///
/// Without the `native-call` feature this is always an error (see module
/// docs). With it, declarations that take memory arguments are refused here
/// and must go through `call_prepared`.
pub fn call_native(
    permission: &NativePermission,
    decl: &ValidatedDeclare,
    host: Target,
    args: &[NativeArg],
) -> Result<NativeReturn, NativeCallError> {
    let library = &decl.spec().library;
    if !permission.allows(library) {
        return Err(NativeCallError::PermissionDenied {
            library: library.clone(),
        });
    }
    if decl.target() != host {
        return Err(NativeCallError::TargetMismatch {
            declared: decl.target(),
            host,
        });
    }
    #[cfg(feature = "native-call")]
    {
        real::call_values(decl, args)
    }
    #[cfg(not(feature = "native-call"))]
    {
        let _ = args;
        Err(NativeCallError::Unavailable {
            reason: "built without the `native-call` feature",
        })
    }
}

/// Performs the call for a marshaled argument list and writes buffers back.
///
/// The caller then passes the returned value to
/// [`PreparedCall::finish`](crate::PreparedCall::finish), which decodes the
/// written-back emulated memory.
#[cfg(feature = "native-call")]
pub fn call_prepared(
    permission: &NativePermission,
    decl: &ValidatedDeclare,
    memory: &mut crate::Memory,
    prepared: &crate::PreparedCall,
) -> Result<NativeReturn, NativeCallError> {
    let library = &decl.spec().library;
    if !permission.allows(library) {
        return Err(NativeCallError::PermissionDenied {
            library: library.clone(),
        });
    }
    real::call_plan(decl, memory, &prepared.native_plan(decl))
}

#[cfg(feature = "native-call")]
mod real {
    use super::{NativeCallError, host_target};
    use crate::dispatch::{self, IntClass, RetClass, RetVal, Val};
    use crate::{CallConv, Memory, NativeArg, NativeArgPlan, NativeReturn, ParamKind, Passing};
    use crate::{EntryCandidate, ValidatedDeclare};
    use vba_profile::{Bitness, Target};

    /// A real, 16-byte-aligned buffer owned by the call stage.
    struct RealBuf {
        words: Vec<u128>,
        len: usize,
    }

    impl RealBuf {
        fn from_bytes(bytes: &[u8]) -> Self {
            let mut words = vec![0u128; bytes.len().div_ceil(16).max(1)];
            for (w, chunk) in words.iter_mut().zip(bytes.chunks(16)) {
                let mut b = [0u8; 16];
                b[..chunk.len()].copy_from_slice(chunk);
                *w = u128::from_ne_bytes(b);
            }
            RealBuf {
                words,
                len: bytes.len(),
            }
        }

        fn addr(&mut self) -> usize {
            self.words.as_mut_ptr() as usize
        }

        fn bytes(&self) -> Vec<u8> {
            let mut out: Vec<u8> = self.words.iter().flat_map(|w| w.to_ne_bytes()).collect();
            out.truncate(self.len);
            out
        }
    }

    fn check_abi(decl: &ValidatedDeclare) -> Result<(), NativeCallError> {
        let loadable = host_target();
        let declared = decl.target();
        if declared != loadable {
            return Err(NativeCallError::AbiMismatch { declared, loadable });
        }
        check_convention(declared, decl.spec().conv)
    }

    /// 64-bit targets have one convention. On 32-bit, `stdcall` needs an x86
    /// ABI selection this stage does not implement; `cdecl` is the platform
    /// default.
    fn check_convention(declared: Target, conv: CallConv) -> Result<(), NativeCallError> {
        if declared.bitness == Bitness::B32 && conv == CallConv::Stdcall {
            return Err(NativeCallError::ConventionMismatch {
                declared: CallConv::Stdcall,
                reason: "32-bit stdcall dispatch is not implemented (only the platform default ABI is)",
            });
        }
        Ok(())
    }

    fn int_class(kind: &ParamKind, target: Target) -> Option<IntClass> {
        Some(match kind {
            ParamKind::Byte => IntClass::U8,
            ParamKind::Boolean | ParamKind::Integer => IntClass::I16,
            ParamKind::Long => IntClass::I32,
            ParamKind::LongLong | ParamKind::Currency => IntClass::I64,
            ParamKind::LongPtr if target.bitness == Bitness::B64 => IntClass::I64,
            ParamKind::LongPtr => IntClass::I32,
            _ => return None,
        })
    }

    /// Converts a by-value native argument to its machine value, as decided by
    /// the declared parameter kind (`Any` by value is a `Long`).
    fn value_of(
        index: usize,
        kind: &ParamKind,
        arg: &NativeArg,
        target: Target,
    ) -> Result<Val, NativeCallError> {
        let bad = |reason: &str| NativeCallError::ArgumentClass {
            index,
            reason: reason.to_string(),
        };
        match (kind, arg) {
            (ParamKind::Single, NativeArg::F32(f)) => Ok(Val::F32(*f)),
            (ParamKind::Double | ParamKind::Date, NativeArg::F64(f)) => Ok(Val::F64(*f)),
            (ParamKind::Any, NativeArg::Int(n)) => i32::try_from(*n)
                .map(Val::I32)
                .map_err(|_| bad("Any by value must fit a Long")),
            (k, NativeArg::Int(n)) => match int_class(k, target) {
                Some(IntClass::U8) => u8::try_from(*n).map(Val::U8).map_err(|_| bad("not a Byte")),
                Some(IntClass::I16) => i16::try_from(*n)
                    .map(Val::I16)
                    .map_err(|_| bad("not an Integer")),
                Some(IntClass::I32) => i32::try_from(*n)
                    .map(Val::I32)
                    .map_err(|_| bad("not a Long")),
                Some(IntClass::I64) => Ok(Val::I64(*n)),
                None => Err(bad("integer given for a non-integer parameter")),
            },
            _ => Err(bad("argument class does not match the parameter type")),
        }
    }

    fn ret_class(decl: &ValidatedDeclare) -> Result<RetClass, NativeCallError> {
        Ok(match &decl.spec().ret {
            None => RetClass::Void,
            Some(ParamKind::Single) => RetClass::F32,
            Some(ParamKind::Double | ParamKind::Date) => RetClass::F64,
            Some(k) => RetClass::Int(int_class(k, decl.target()).ok_or(
                NativeCallError::Unsupported {
                    reason: "non-scalar return type",
                },
            )?),
        })
    }

    pub(super) fn call_values(
        decl: &ValidatedDeclare,
        args: &[NativeArg],
    ) -> Result<NativeReturn, NativeCallError> {
        let params = &decl.spec().params;
        if params.len() != args.len() {
            return Err(NativeCallError::ArgumentCount {
                expected: params.len(),
                got: args.len(),
            });
        }
        if params
            .iter()
            .any(|p| !(p.passing == Passing::ByVal && (p.kind.is_scalar())))
        {
            return Err(NativeCallError::Unsupported {
                reason: "memory arguments must go through call_prepared",
            });
        }
        let plan: Vec<_> = args.iter().map(|a| NativeArgPlan::Value(*a)).collect();
        // Pure value calls never touch memory.
        call_plan(decl, &mut Memory::new(decl.target().bitness), &plan)
    }

    pub(super) fn call_plan(
        decl: &ValidatedDeclare,
        memory: &mut Memory,
        plan: &[NativeArgPlan],
    ) -> Result<NativeReturn, NativeCallError> {
        check_abi(decl)?;
        let spec = decl.spec();
        if spec.params.len() != plan.len() {
            return Err(NativeCallError::ArgumentCount {
                expected: spec.params.len(),
                got: plan.len(),
            });
        }
        let target = decl.target();
        let ret = ret_class(decl)?;
        let mem_err = |e: crate::MemoryError| NativeCallError::Memory(e.to_string());

        // Real buffers: owned here until after write-back.
        let mut bufs: Vec<RealBuf> = Vec::new();
        // For each argument: index into `bufs` (region), plus the cell buffer
        // index for string cells.
        let mut vals = Vec::with_capacity(plan.len());
        let mut regions: Vec<(usize, usize, u64, usize)> = Vec::new();
        let mut cells: Vec<(usize, usize, usize)> = Vec::new();
        for (i, (p, param)) in plan.iter().zip(&spec.params).enumerate() {
            match p {
                NativeArgPlan::Value(a) => {
                    vals.push(value_of(i, &param.kind, a, target)?);
                }
                NativeArgPlan::Region { addr, len } => {
                    let data = memory.read(*addr, *len).map_err(mem_err)?;
                    bufs.push(RealBuf::from_bytes(data));
                    let b = bufs.len() - 1;
                    regions.push((i, b, *addr, *len));
                    vals.push(Val::Ptr(0));
                }
                NativeArgPlan::TextCell {
                    text,
                    text_len,
                    cell: _,
                } => {
                    let data = memory.read(*text, *text_len).map_err(mem_err)?;
                    bufs.push(RealBuf::from_bytes(data));
                    let t = bufs.len() - 1;
                    regions.push((i, t, *text, *text_len));
                    // The cell holds the REAL address of the text buffer.
                    let text_addr = bufs[t].addr();
                    let width = usize::from(target.pointer_size());
                    bufs.push(RealBuf::from_bytes(&text_addr.to_le_bytes()[..width]));
                    cells.push((i, bufs.len() - 1, t));
                    vals.push(Val::Ptr(0));
                }
            }
        }
        // Fix up pointer values now that no buffer will be pushed or moved
        // (the heap blocks never move; only the `bufs` header could).
        for (i, b, _, _) in &regions {
            if !matches!(plan[*i], NativeArgPlan::TextCell { .. }) {
                vals[*i] = Val::Ptr(bufs[*b].addr());
            }
        }
        for (i, cell, _) in &cells {
            vals[*i] = Val::Ptr(bufs[*cell].addr());
        }
        let text_addrs: Vec<(usize, usize)> = cells
            .iter()
            .map(|(i, _, t)| (*i, bufs[*t].addr()))
            .collect();

        let mut symbols = Vec::new();
        for c in decl.entry_candidates() {
            match c {
                EntryCandidate::Symbol(s) => symbols.push(s),
                EntryCandidate::Ordinal(_) => return Err(NativeCallError::OrdinalUnsupported),
            }
        }
        // Contract of `dispatch::call`: the host granted this library by name
        // (checked by the callers); `vals` classes were derived from the
        // validated descriptor; every `Val::Ptr` is the address of a `RealBuf`
        // in `bufs`, which is alive, exclusively owned here and untouched until
        // the call returns. That the real function matches the declaration is
        // the permission holder's contract and cannot be verified.
        let result = dispatch::call(&spec.library, &symbols, &vals, ret).map_err(|e| match e {
            dispatch::ResolveError::Library(reason) => NativeCallError::LibraryNotFound {
                library: spec.library.clone(),
                reason,
            },
            dispatch::ResolveError::Symbol(symbols) => NativeCallError::SymbolNotFound {
                library: spec.library.clone(),
                symbols,
            },
        })?;

        // A ByRef String cell must still point at the same real text buffer.
        for ((i, cell, _), (_, want)) in cells.iter().zip(&text_addrs) {
            let width = usize::from(target.pointer_size());
            if bufs[*cell].bytes() != want.to_le_bytes()[..width] {
                return Err(NativeCallError::PointerCellChanged { index: *i });
            }
        }
        // Copy every real buffer back into emulated memory.
        for (_, b, addr, len) in &regions {
            let mut bytes = bufs[*b].bytes();
            bytes.truncate(*len);
            memory.write(*addr, &bytes).map_err(mem_err)?;
        }
        // Scalar ByRef cells are `Region`s as well, so they are covered above.
        Ok(match result {
            RetVal::Void => NativeReturn::Void,
            RetVal::Int(n) => NativeReturn::Int(n),
            RetVal::F32(f) => NativeReturn::Float(f64::from(f)),
            RetVal::F64(f) => NativeReturn::Float(f),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn convention_rules() {
            assert!(matches!(
                check_convention(Target::WIN32, CallConv::Stdcall),
                Err(NativeCallError::ConventionMismatch { .. })
            ));
            assert!(check_convention(Target::WIN32, CallConv::Cdecl).is_ok());
            assert!(check_convention(Target::WIN64, CallConv::Stdcall).is_ok());
            assert!(check_convention(Target::MAC64, CallConv::Cdecl).is_ok());
        }
    }
}
