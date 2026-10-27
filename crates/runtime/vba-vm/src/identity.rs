//! Emulated `ObjPtr`, `StrPtr` and `VarPtr`.
//!
//! Real VBA returns machine addresses. This VM never exposes one: each
//! function returns an opaque identity token, a `LongPtr` that is
//!
//! * stable: the same object (or variable, or string content) gives the
//!   same token for the whole session,
//! * distinct: different live objects give different tokens (collisions
//!   between different variables or strings are possible in principle, as
//!   the tokens are hashes, but are not expected),
//! * nonzero, except for `Nothing` and the empty string, which give 0,
//! * not an address: the value is derived from an internal handle number
//!   mixed with `VmConfig::identity_salt`, carries a fixed low-bit tag
//!   (always odd), and does not point at anything. There is no operation in
//!   this VM that reads memory at a number, so a token cannot be
//!   dereferenced; `CopyMemory` and friends only act through emulated shims
//!   that recognise their own handles.
//!
//! No capability is needed because nothing is leaked, and no step is
//! charged beyond the call itself.

use vba_ir::CallArg;
use vba_profile::Bitness;
use vba_value::{ObjectRef, PointerWidth, Variant, VbaError, convert};

use crate::error::VResult;
use crate::state::{Idx, Loc, Slot};
use crate::vm::Vm;

const KIND_OBJECT: u64 = 1;
const KIND_STRING: u64 = 2;
const KIND_VARIABLE: u64 = 3;

/// FNV-1a over 64-bit words: small, dependency-free and identical on every
/// toolchain, so tokens are reproducible.
fn fnv(words: &[u64]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn loc_words(loc: &Loc, out: &mut Vec<u64>) {
    match loc {
        Loc::Local { frame, slot } => out.extend([1, *frame as u64, u64::from(*slot)]),
        Loc::Global(g) => out.extend([2, u64::from(*g)]),
        Loc::Field { obj, index } => out.extend([3, *obj, u64::from(*index)]),
        Loc::Elem { base, idx } => {
            out.push(4);
            loc_words(base, out);
            index_words(idx, out);
        }
        Loc::UdtField { base, index } => {
            out.push(5);
            loc_words(base, out);
            out.push(u64::from(*index));
        }
    }
}

fn index_words(idx: &Idx, out: &mut Vec<u64>) {
    out.push(idx.len() as u64);
    out.extend(idx.iter().map(|i| u64::from(*i as u32)));
}

impl Vm {
    fn pointer_width(&self) -> PointerWidth {
        match self.ir.program.profile.target.bitness {
            Bitness::B64 => PointerWidth::Bits64,
            _ => PointerWidth::Bits32,
        }
    }

    /// Packs `payload` into a token of the session's pointer width: the
    /// payload occupies the high bits, the low four are a kind tag and a
    /// constant 1, so the result is never 0 and never looks aligned.
    fn identity_token(&self, kind: u64, payload: u64) -> VResult<Variant> {
        let width = self.pointer_width();
        let bits = if width == PointerWidth::Bits64 {
            47
        } else {
            31
        };
        let mask = (1u64 << (bits - 4)) - 1;
        let p = (payload ^ self.cfg.identity_salt) & mask;
        let token = (p << 4) | (kind << 1) | 1;
        Ok(Variant::from_long_ptr(token as i64, width)?)
    }

    fn zero_token(&self) -> VResult<Variant> {
        Ok(Variant::from_long_ptr(0, self.pointer_width())?)
    }

    /// `ObjPtr(obj)`: 0 for `Nothing`, else a token of the object's handle.
    /// Handles are not reused within a session, so distinct live objects get
    /// distinct tokens.
    pub(crate) fn exec_objptr(&mut self, arg: &Option<Variant>) -> VResult<Variant> {
        match arg {
            Some(Variant::Object(ObjectRef::Nothing)) => self.zero_token(),
            Some(Variant::Object(ObjectRef::Handle(h))) => {
                // The odd multiplier keeps the mapping one to one; `+ 1`
                // keeps handle 0 away from an all-zero payload.
                let x = h.0.wrapping_add(1).wrapping_mul(0x2545_F491_4F6C_DD1D);
                self.identity_token(KIND_OBJECT, x)
            }
            Some(_) => Err(VbaError::TYPE_MISMATCH.into()),
            None => Err(VbaError::new(449).into()),
        }
    }

    /// `StrPtr(s)`: 0 for the empty string (this VM does not distinguish
    /// `vbNullString` from `""`, where real VBA gives `""` a nonzero
    /// pointer), else a token of the content, so equal text gives equal
    /// tokens.
    pub(crate) fn exec_strptr(&mut self, arg: &Option<Variant>) -> VResult<Variant> {
        let Some(v) = arg else {
            return Err(VbaError::new(449).into());
        };
        let v = self.value_of(v.clone())?;
        let s = convert::to_vba_string(&v)?;
        if s.is_empty() {
            return self.zero_token();
        }
        let words: Vec<u64> = s.as_units().iter().map(|u| u64::from(*u)).collect();
        self.identity_token(KIND_STRING, fnv(&words))
    }

    /// `VarPtr(v)`: a token of the variable's storage location, so the same
    /// variable always gives the same token and a `ByRef` parameter gives
    /// the token of the caller's variable. An argument that is not a
    /// variable (a value) gets a token of the value.
    pub(crate) fn exec_varptr(&mut self, args: &[CallArg]) -> VResult<Variant> {
        let mut words = Vec::new();
        match args.first() {
            Some(CallArg::Ref(r)) => {
                let loc = self.reg_loc(*r)?;
                loc_words(&self.canonical_loc(loc), &mut words);
            }
            Some(CallArg::Val(op)) => {
                let v = self.val(op)?;
                words.push(0);
                words.extend(format!("{v:?}").bytes().map(u64::from));
            }
            _ => return Err(VbaError::new(449).into()),
        }
        self.identity_token(KIND_VARIABLE, fnv(&words))
    }

    /// Follows `ByRef` bindings to the storage that owns the variable.
    fn canonical_loc(&self, loc: Loc) -> Loc {
        let mut loc = loc;
        while let Loc::Local { frame, slot } = &loc {
            match self
                .frames
                .get(*frame)
                .and_then(|f| f.slots.get(*slot as usize))
            {
                Some(Slot::Bound(l)) => loc = l.clone(),
                _ => break,
            }
        }
        loc
    }
}
