//! The argument marshaling plan.
//!
//! [`PreparedCall::prepare`] turns VBA argument values into native arguments
//! (integers, floats and addresses in the emulated [`Memory`]), and
//! [`PreparedCall::finish`] reads results back out after the call:
//!
//! * `ByVal` scalars are passed by value; `ByRef` scalars get a temporary cell
//!   whose address is passed and whose content is written back.
//! * `String` (both `ByVal` and `ByRef`) is converted to a NUL-terminated ANSI
//!   buffer; the whole buffer (not just up to the first NUL) is converted back
//!   afterwards, as VBA does. `ByRef String` passes the address of a
//!   pointer-sized cell holding the buffer address.
//! * `As Any` accepts a string (ANSI buffer), byte buffer (pointer) or, `ByVal`
//!   only, a `Long`.
//! * UDTs and arrays are passed as the address of an encoded buffer and read
//!   back.

use crate::{Codepage, FfiError, Memory, ParamKind, Passing, Unmappable, ValidatedDeclare};
use vba_profile::{Bitness, Target};
use vba_value::VbaString;

/// A VBA-side argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    /// Integer-like scalar (`Currency` as its scaled integer).
    Int(i64),
    /// `Single`, `Double`, `Date`.
    Float(f64),
    /// A VBA string.
    Str(VbaString),
    /// Raw bytes: a `Byte()` array, an encoded UDT or an `Any` buffer.
    Bytes(Vec<u8>),
    /// An array of scalars.
    Array(Vec<ArgValue>),
}

/// A native-side argument. Pointers are addresses of the emulated memory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeArg {
    /// Integer or address.
    Int(i64),
    /// 32-bit float.
    F32(f32),
    /// 64-bit float.
    F64(f64),
}

impl NativeArg {
    /// The integer/address value, if this is one.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            NativeArg::Int(v) => Some(*v),
            _ => None,
        }
    }
}

/// How one native argument must be presented to a real call.
///
/// This is derived from the marshaling plan, so the call stage never has to
/// guess which integers are really emulated-memory addresses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeArgPlan {
    /// Passed by value (an integer or float class value, never an address).
    Value(NativeArg),
    /// An emulated-memory region of `len` bytes; a real call passes the
    /// address of a real copy and copies it back afterwards.
    Region {
        /// Emulated address of the region.
        addr: u64,
        /// Length in bytes.
        len: usize,
    },
    /// A `ByRef String`: a pointer-sized cell that holds the address of a
    /// text region of `text_len` bytes (terminating NUL included).
    TextCell {
        /// Emulated address of the cell.
        cell: u64,
        /// Emulated address of the text.
        text: u64,
        /// Text length including the terminating NUL.
        text_len: usize,
    },
}

/// What a native function returned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeReturn {
    /// No value (`Sub`).
    Void,
    /// Integer-class value (unnarrowed; see [`narrow_return`]).
    Int(i64),
    /// Floating-point value.
    Float(f64),
}

/// Result of [`PreparedCall::finish`].
#[derive(Debug, Clone, PartialEq)]
pub struct CallOutcome {
    /// Argument values after write-back, in declaration order.
    pub args: Vec<ArgValue>,
    /// Return value narrowed to the declared type; `None` for `Sub`.
    pub ret: Option<ArgValue>,
}

#[derive(Debug)]
enum Slot {
    Value,
    Scalar {
        kind: ParamKind,
        addr: u64,
    },
    Text {
        addr: u64,
        len: usize,
    },
    Buffer {
        addr: u64,
        len: usize,
    },
    Array {
        elem: ParamKind,
        count: usize,
        addr: u64,
    },
}

/// A call whose arguments have been marshaled into emulated memory.
#[derive(Debug)]
pub struct PreparedCall {
    args: Vec<NativeArg>,
    slots: Vec<Slot>,
    originals: Vec<ArgValue>,
    temps: Vec<u64>,
    codepage: Codepage,
}

fn mismatch(index: usize, reason: &str) -> FfiError {
    FfiError::ArgumentMismatch {
        index,
        reason: reason.to_string(),
    }
}

fn encode_scalar(kind: &ParamKind, v: &ArgValue, target: Target) -> Result<Vec<u8>, String> {
    let range = |lo: i64, hi: i64, w: usize| match v {
        ArgValue::Int(n) if (lo..=hi).contains(n) => Ok(n.to_le_bytes()[..w].to_vec()),
        ArgValue::Int(n) => Err(format!("{n} out of range for {kind:?}")),
        _ => Err(format!("{kind:?} needs an integer value")),
    };
    match kind {
        ParamKind::Byte => range(0, 255, 1),
        ParamKind::Boolean | ParamKind::Integer => range(i16::MIN.into(), i16::MAX.into(), 2),
        ParamKind::Long => range(i32::MIN.into(), i32::MAX.into(), 4),
        ParamKind::LongLong | ParamKind::Currency => range(i64::MIN, i64::MAX, 8),
        ParamKind::LongPtr => {
            if target.bitness == Bitness::B64 {
                range(i64::MIN, i64::MAX, 8)
            } else {
                range(i32::MIN.into(), i32::MAX.into(), 4)
            }
        }
        ParamKind::Single => match v {
            ArgValue::Float(f) => Ok((*f as f32).to_le_bytes().to_vec()),
            _ => Err("Single needs a float value".into()),
        },
        ParamKind::Double | ParamKind::Date => match v {
            ArgValue::Float(f) => Ok(f.to_le_bytes().to_vec()),
            _ => Err("Double/Date needs a float value".into()),
        },
        _ => Err(format!("{kind:?} is not a scalar")),
    }
}

fn decode_scalar(kind: &ParamKind, b: &[u8]) -> ArgValue {
    let signed = |b: &[u8]| {
        let mut buf = if b.last().is_some_and(|x| x & 0x80 != 0) {
            [0xFF; 8]
        } else {
            [0; 8]
        };
        buf[..b.len()].copy_from_slice(b);
        i64::from_le_bytes(buf)
    };
    match kind {
        ParamKind::Byte => ArgValue::Int(i64::from(b[0])),
        ParamKind::Single => {
            ArgValue::Float(f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        }
        ParamKind::Double | ParamKind::Date => {
            ArgValue::Float(f64::from_le_bytes(b.try_into().expect("8 bytes")))
        }
        _ => ArgValue::Int(signed(b)),
    }
}

fn native_of(kind: &ParamKind, v: &ArgValue, target: Target) -> Result<NativeArg, String> {
    let bytes = encode_scalar(kind, v, target)?;
    Ok(match kind {
        ParamKind::Single => NativeArg::F32(f32::from_le_bytes(bytes.try_into().expect("4"))),
        ParamKind::Double | ParamKind::Date => {
            NativeArg::F64(f64::from_le_bytes(bytes.try_into().expect("8")))
        }
        _ => match decode_scalar(kind, &bytes) {
            ArgValue::Int(n) => NativeArg::Int(n),
            _ => unreachable!("integer kinds decode to Int"),
        },
    })
}

/// Narrows a raw integer return value to the declared type, the way the
/// caller of a native function sees it (for example a `DWORD` of
/// `0xFFFFFFFF` declared `As Long` becomes -1).
pub fn narrow_return(kind: &ParamKind, raw: i64, target: Target) -> Result<ArgValue, FfiError> {
    let v = match kind {
        ParamKind::Byte => i64::from(raw as u8),
        ParamKind::Boolean | ParamKind::Integer => i64::from(raw as i16),
        ParamKind::Long => i64::from(raw as i32),
        ParamKind::LongPtr if target.bitness != Bitness::B64 => i64::from(raw as i32),
        ParamKind::LongLong | ParamKind::LongPtr | ParamKind::Currency => raw,
        _ => return Err(FfiError::Unsupported("non-integer return narrowing")),
    };
    Ok(ArgValue::Int(v))
}

impl PreparedCall {
    /// Marshals `values` for `decl`, allocating temporaries in `memory`.
    pub fn prepare(
        decl: &ValidatedDeclare,
        memory: &mut Memory,
        values: &[ArgValue],
        codepage: Codepage,
    ) -> Result<PreparedCall, FfiError> {
        let params = &decl.spec().params;
        if params.len() != values.len() {
            return Err(FfiError::ArgumentMismatch {
                index: values.len().min(params.len()),
                reason: format!("expected {} arguments, got {}", params.len(), values.len()),
            });
        }
        let mut call = PreparedCall {
            args: Vec::new(),
            slots: Vec::new(),
            originals: values.to_vec(),
            temps: Vec::new(),
            codepage,
        };
        for (i, (p, v)) in params.iter().zip(values).enumerate() {
            if let Err(e) = call.prepare_one(i, p.kind.clone(), p.passing, v, decl.target(), memory)
            {
                call.release(memory);
                return Err(e);
            }
        }
        Ok(call)
    }

    fn temp(&mut self, memory: &mut Memory, data: &[u8]) -> Result<u64, FfiError> {
        let a = memory.alloc_bytes(data)?;
        self.temps.push(a);
        Ok(a)
    }

    fn prepare_one(
        &mut self,
        i: usize,
        kind: ParamKind,
        passing: Passing,
        v: &ArgValue,
        target: Target,
        memory: &mut Memory,
    ) -> Result<(), FfiError> {
        let ptr_bytes =
            |a: u64| -> Vec<u8> { a.to_le_bytes()[..usize::from(target.pointer_size())].to_vec() };
        match kind {
            k if k.is_scalar() => match passing {
                Passing::ByVal => {
                    let n = native_of(&k, v, target).map_err(|r| mismatch(i, &r))?;
                    self.args.push(n);
                    self.slots.push(Slot::Value);
                }
                Passing::ByRef => {
                    let bytes = encode_scalar(&k, v, target).map_err(|r| mismatch(i, &r))?;
                    let addr = self.temp(memory, &bytes)?;
                    self.args.push(NativeArg::Int(addr as i64));
                    self.slots.push(Slot::Scalar { kind: k, addr });
                }
            },
            ParamKind::String | ParamKind::Any if matches!(v, ArgValue::Str(_)) => {
                let ArgValue::Str(s) = v else { unreachable!() };
                let mut bytes = self
                    .codepage
                    .encode(s.as_units(), Unmappable::Reject, true)?
                    .bytes;
                let len = bytes.len();
                bytes.push(0);
                let addr = self.temp(memory, &bytes)?;
                let by_ref_string = kind == ParamKind::String && passing == Passing::ByRef;
                let arg = if by_ref_string {
                    self.temp(memory, &ptr_bytes(addr))?
                } else {
                    addr
                };
                self.args.push(NativeArg::Int(arg as i64));
                self.slots.push(Slot::Text { addr, len });
            }
            ParamKind::String => return Err(mismatch(i, "String needs a string value")),
            ParamKind::Any => match v {
                ArgValue::Bytes(b) if !b.is_empty() => {
                    let addr = self.temp(memory, b)?;
                    self.args.push(NativeArg::Int(addr as i64));
                    self.slots.push(Slot::Buffer { addr, len: b.len() });
                }
                ArgValue::Int(n) if passing == Passing::ByVal && i32::try_from(*n).is_ok() => {
                    self.args.push(NativeArg::Int(*n));
                    self.slots.push(Slot::Value);
                }
                _ => {
                    return Err(mismatch(
                        i,
                        "As Any needs a string, non-empty bytes or ByVal Long",
                    ));
                }
            },
            ParamKind::Udt(l) => match v {
                ArgValue::Bytes(b) if b.len() == l.size() => {
                    let addr = self.temp(memory, b)?;
                    self.args.push(NativeArg::Int(addr as i64));
                    self.slots.push(Slot::Buffer { addr, len: b.len() });
                }
                _ => return Err(mismatch(i, "UDT needs encoded bytes of exactly LenB")),
            },
            ParamKind::Array(elem) => {
                let esize = elem
                    .value_size(target)
                    .ok_or(FfiError::Unsupported("array element size"))?;
                match v {
                    ArgValue::Bytes(b) if *elem == ParamKind::Byte && !b.is_empty() => {
                        let addr = self.temp(memory, b)?;
                        self.args.push(NativeArg::Int(addr as i64));
                        self.slots.push(Slot::Buffer { addr, len: b.len() });
                    }
                    ArgValue::Array(items) if !items.is_empty() && elem.is_scalar() => {
                        let mut bytes = Vec::with_capacity(items.len() * esize);
                        for it in items {
                            bytes.extend(
                                encode_scalar(&elem, it, target).map_err(|r| mismatch(i, &r))?,
                            );
                        }
                        let addr = self.temp(memory, &bytes)?;
                        self.args.push(NativeArg::Int(addr as i64));
                        self.slots.push(Slot::Array {
                            elem: *elem,
                            count: items.len(),
                            addr,
                        });
                    }
                    ArgValue::Bytes(b)
                        if matches!(*elem, ParamKind::Udt(_))
                            && !b.is_empty()
                            && b.len() % esize == 0 =>
                    {
                        let addr = self.temp(memory, b)?;
                        self.args.push(NativeArg::Int(addr as i64));
                        self.slots.push(Slot::Buffer { addr, len: b.len() });
                    }
                    _ => return Err(mismatch(i, "array value does not match element type")),
                }
            }
            ParamKind::Variant => return Err(FfiError::Unsupported("Variant arguments")),
            _ => unreachable!("scalars handled above"),
        }
        Ok(())
    }

    /// The native arguments to hand to the callee.
    pub fn args(&self) -> &[NativeArg] {
        &self.args
    }

    /// Describes each native argument for a real call (see [`NativeArgPlan`]).
    pub fn native_plan(&self, decl: &ValidatedDeclare) -> Vec<NativeArgPlan> {
        let target = decl.target();
        self.slots
            .iter()
            .zip(&self.args)
            .map(|(slot, arg)| match slot {
                Slot::Value => NativeArgPlan::Value(*arg),
                Slot::Scalar { kind, addr } => NativeArgPlan::Region {
                    addr: *addr,
                    len: kind.value_size(target).expect("scalar"),
                },
                Slot::Text { addr, len } => match arg.as_int() {
                    Some(a) if a as u64 != *addr => NativeArgPlan::TextCell {
                        cell: a as u64,
                        text: *addr,
                        text_len: len + 1,
                    },
                    _ => NativeArgPlan::Region {
                        addr: *addr,
                        len: len + 1,
                    },
                },
                Slot::Buffer { addr, len } => NativeArgPlan::Region {
                    addr: *addr,
                    len: *len,
                },
                Slot::Array { elem, count, addr } => NativeArgPlan::Region {
                    addr: *addr,
                    len: elem.value_size(target).expect("scalar") * count,
                },
            })
            .collect()
    }

    /// Frees all temporaries without reading anything back (error path).
    pub fn release(&mut self, memory: &mut Memory) {
        for a in self.temps.drain(..) {
            let _ = memory.free(a);
        }
    }

    /// Reads results back, narrows the return value and frees temporaries.
    pub fn finish(
        mut self,
        decl: &ValidatedDeclare,
        memory: &mut Memory,
        ret: NativeReturn,
    ) -> Result<CallOutcome, FfiError> {
        let result = self.collect(decl, memory, ret);
        self.release(memory);
        result
    }

    fn collect(
        &self,
        decl: &ValidatedDeclare,
        memory: &Memory,
        ret: NativeReturn,
    ) -> Result<CallOutcome, FfiError> {
        let target = decl.target();
        let mut args = Vec::with_capacity(self.slots.len());
        for (slot, original) in self.slots.iter().zip(&self.originals) {
            args.push(match slot {
                Slot::Value => original.clone(),
                Slot::Scalar { kind, addr } => {
                    let size = kind.value_size(target).expect("scalar");
                    decode_scalar(kind, memory.read(*addr, size)?)
                }
                Slot::Text { addr, len } => {
                    let bytes = memory.read(*addr, *len)?;
                    ArgValue::Str(VbaString::from_units(self.codepage.decode(bytes, true)?))
                }
                Slot::Buffer { addr, len } => ArgValue::Bytes(memory.read(*addr, *len)?.to_vec()),
                Slot::Array { elem, count, addr } => {
                    let size = elem.value_size(target).expect("scalar");
                    let bytes = memory.read(*addr, size * count)?;
                    ArgValue::Array(bytes.chunks(size).map(|c| decode_scalar(elem, c)).collect())
                }
            });
        }
        let ret = match (&decl.spec().ret, ret) {
            (None, _) => None,
            (Some(ParamKind::Single), NativeReturn::Float(f)) => {
                Some(ArgValue::Float(f64::from(f as f32)))
            }
            (Some(ParamKind::Double | ParamKind::Date), NativeReturn::Float(f)) => {
                Some(ArgValue::Float(f))
            }
            (Some(k), NativeReturn::Int(n)) => Some(narrow_return(k, n, target)?),
            (Some(_), _) => {
                return Err(FfiError::Unsupported(
                    "return class does not match declaration",
                ));
            }
        };
        Ok(CallOutcome { args, ret })
    }
}
