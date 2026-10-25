//! Validated `Declare` signature descriptors (MS-VBAL §5.2.3.5 external
//! procedure declarations).
//!
//! A [`DeclareSpec`] is plain data; only [`DeclareSpec::validate`] produces a
//! [`ValidatedDeclare`], and every later stage accepts only validated
//! descriptors. Target semantics (pointer width, `PtrSafe` requirement,
//! calling convention) come from the `vba-profile` [`Target`] in the spec.

use crate::{FfiError, UdtLayout};
use vba_profile::{Bitness, OperatingSystem, Target};

/// Calling convention requested by the declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallConv {
    /// `__stdcall`, the VBA default on Windows.
    Stdcall,
    /// `__cdecl` (Office for Mac and explicitly cdecl libraries).
    Cdecl,
}

/// Character set used to choose between ANSI and Unicode entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Charset {
    /// ANSI (the VBA default): try `Name` then `NameA`.
    Ansi,
    /// Unicode: try `NameW` then `Name`.
    Unicode,
}

/// How a parameter is passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Passing {
    /// `ByVal`.
    ByVal,
    /// `ByRef` (the VBA default).
    ByRef,
}

/// The declared type of a parameter or return value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamKind {
    /// `Byte`.
    Byte,
    /// `Boolean`.
    Boolean,
    /// `Integer`.
    Integer,
    /// `Long`.
    Long,
    /// `LongLong` (64-bit targets only).
    LongLong,
    /// `LongPtr`.
    LongPtr,
    /// `Single`.
    Single,
    /// `Double`.
    Double,
    /// `Currency`.
    Currency,
    /// `Date`.
    Date,
    /// `String`.
    String,
    /// `Variant` (validated, marshaling not implemented).
    Variant,
    /// `As Any`.
    Any,
    /// A user-defined type.
    Udt(UdtLayout),
    /// An array of scalars or UDTs, passed as a pointer to its first element.
    Array(Box<ParamKind>),
}

impl ParamKind {
    /// Whether this is a plain scalar (usable as a return type).
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            ParamKind::Byte
                | ParamKind::Boolean
                | ParamKind::Integer
                | ParamKind::Long
                | ParamKind::LongLong
                | ParamKind::LongPtr
                | ParamKind::Single
                | ParamKind::Double
                | ParamKind::Currency
                | ParamKind::Date
        )
    }

    /// Size in bytes of a scalar or UDT value on `target`.
    pub fn value_size(&self, target: Target) -> Option<usize> {
        Some(match self {
            ParamKind::Byte => 1,
            ParamKind::Boolean | ParamKind::Integer => 2,
            ParamKind::Long | ParamKind::Single => 4,
            ParamKind::LongLong | ParamKind::Double | ParamKind::Currency | ParamKind::Date => 8,
            ParamKind::LongPtr => usize::from(target.pointer_size()),
            ParamKind::Udt(l) => l.size(),
            _ => return None,
        })
    }
}

/// One declared parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSpec {
    /// Parameter name (documentation only).
    pub name: String,
    /// Declared type.
    pub kind: ParamKind,
    /// Passing mode.
    pub passing: Passing,
}

/// The name an `Alias` clause (or the procedure name) designates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryName {
    /// A named export.
    Symbol(String),
    /// An ordinal export written `#n`.
    Ordinal(u16),
}

impl EntryName {
    /// Parses an `Alias` string: `#n` (1..=65535) is an ordinal.
    pub fn parse(alias: &str) -> Result<Self, FfiError> {
        if alias.is_empty() || alias.contains('\0') {
            return Err(FfiError::InvalidSignature("empty or NUL alias".into()));
        }
        if let Some(digits) = alias.strip_prefix('#') {
            let n: u16 = digits.parse().ok().filter(|n| *n != 0).ok_or_else(|| {
                FfiError::InvalidSignature(format!("bad ordinal alias `{alias}`"))
            })?;
            return Ok(EntryName::Ordinal(n));
        }
        Ok(EntryName::Symbol(alias.to_string()))
    }
}

/// One entry point to try, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryCandidate {
    /// Look up by name.
    Symbol(String),
    /// Look up by ordinal.
    Ordinal(u16),
}

/// An unvalidated `Declare` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclareSpec {
    /// Target whose semantics apply.
    pub target: Target,
    /// Whether `PtrSafe` was written.
    pub ptr_safe: bool,
    /// The VBA-side procedure name.
    pub name: String,
    /// The `Lib` string.
    pub library: String,
    /// The `Alias` string, if any.
    pub alias: Option<String>,
    /// ANSI/Unicode selection.
    pub charset: Charset,
    /// Calling convention.
    pub conv: CallConv,
    /// Parameters.
    pub params: Vec<ParamSpec>,
    /// Return type; `None` for a `Sub`.
    pub ret: Option<ParamKind>,
}

/// A declaration that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDeclare {
    spec: DeclareSpec,
    entry: EntryName,
    explicit_alias: bool,
}

fn invalid<T>(msg: impl Into<String>) -> Result<T, FfiError> {
    Err(FfiError::InvalidSignature(msg.into()))
}

fn validate_kind(
    kind: &ParamKind,
    passing: Passing,
    target: Target,
    what: &str,
) -> Result<(), FfiError> {
    match kind {
        ParamKind::LongLong if target.bitness != Bitness::B64 => {
            invalid(format!("{what}: LongLong requires a 64-bit target"))
        }
        ParamKind::Udt(l) if l.target() != target => {
            invalid(format!("{what}: UDT laid out for another target"))
        }
        ParamKind::Udt(_) if passing == Passing::ByVal => invalid(format!(
            "{what}: a user-defined type cannot be passed ByVal"
        )),
        ParamKind::Array(_) if passing == Passing::ByVal => {
            invalid(format!("{what}: an array cannot be passed ByVal"))
        }
        ParamKind::Variant if passing == Passing::ByVal => {
            invalid(format!("{what}: ByVal Variant is not supported"))
        }
        ParamKind::Array(elem) => match **elem {
            ParamKind::Udt(_) => validate_kind(elem, Passing::ByRef, target, what),
            _ if elem.is_scalar() => validate_kind(elem, Passing::ByRef, target, what),
            _ => invalid(format!("{what}: array elements must be scalars or UDTs")),
        },
        _ => Ok(()),
    }
}

impl DeclareSpec {
    /// Validates the declaration.
    ///
    /// Rules: library/name non-empty; Win16 is unsupported; 64-bit targets
    /// require `PtrSafe` (VBA7); `LongLong` only on 64-bit; UDTs, arrays and
    /// Variants only `ByRef`; return types are scalars; a `#n` alias is a
    /// valid ordinal; at most 60 parameters (VBA's procedure limit).
    pub fn validate(self) -> Result<ValidatedDeclare, FfiError> {
        if self.name.is_empty() || self.library.is_empty() {
            return invalid("procedure name and Lib must not be empty");
        }
        if self.name.contains('\0') || self.library.contains('\0') {
            return invalid("NUL in name or Lib");
        }
        if self.target.bitness == Bitness::B16 {
            return invalid("16-bit declarations are not supported");
        }
        if self.target.bitness == Bitness::B64 && !self.ptr_safe {
            return invalid("64-bit VBA requires PtrSafe on every Declare");
        }
        if self.params.len() > 60 {
            return invalid("more than 60 parameters");
        }
        for (i, p) in self.params.iter().enumerate() {
            validate_kind(
                &p.kind,
                p.passing,
                self.target,
                &format!("parameter {i} `{}`", p.name),
            )?;
        }
        if let Some(r) = &self.ret {
            if !r.is_scalar() {
                return invalid("return type must be a scalar");
            }
            validate_kind(r, Passing::ByVal, self.target, "return type")?;
        }
        let (entry, explicit_alias) = match &self.alias {
            Some(a) => (EntryName::parse(a)?, true),
            None => (EntryName::Symbol(self.name.clone()), false),
        };
        Ok(ValidatedDeclare {
            spec: self,
            entry,
            explicit_alias,
        })
    }
}

impl ValidatedDeclare {
    /// The underlying (validated) spec.
    pub fn spec(&self) -> &DeclareSpec {
        &self.spec
    }

    /// The target semantics in force.
    pub fn target(&self) -> Target {
        self.spec.target
    }

    /// The designated entry (alias or procedure name).
    pub fn entry(&self) -> &EntryName {
        &self.entry
    }

    /// The effective convention: 64-bit Windows has a single calling
    /// convention, so both declared conventions collapse to the platform one;
    /// this reports what was declared only on 32-bit targets.
    pub fn effective_conv(&self) -> Option<CallConv> {
        match self.spec.target.bitness {
            Bitness::B32 => Some(self.spec.conv),
            _ => None,
        }
    }

    /// Bytes of stack the arguments occupy on 32-bit stdcall (each slot is
    /// rounded up to 4 bytes; ByRef/String/Any/Array/UDT are pointers).
    pub fn stdcall_arg_bytes(&self) -> usize {
        self.spec
            .params
            .iter()
            .map(|p| match (&p.kind, p.passing) {
                (k, Passing::ByVal) if k.is_scalar() => k
                    .value_size(self.spec.target)
                    .unwrap_or(4)
                    .next_multiple_of(4),
                _ => 4,
            })
            .sum()
    }

    /// The entry points to try, in order.
    ///
    /// * `#n` aliases give exactly that ordinal.
    /// * An explicit `Alias` is used verbatim, with no suffix probing.
    /// * Otherwise (Windows only) ANSI declarations try `Name` then `NameA`
    ///   and Unicode ones `NameW` then `Name` (observed VBA behavior; not
    ///   specified by MS-VBAL).
    /// * On 32-bit stdcall Windows the decorated `_Name@N` is appended last.
    ///
    /// On macOS the name is used verbatim.
    pub fn entry_candidates(&self) -> Vec<EntryCandidate> {
        let name = match &self.entry {
            EntryName::Ordinal(n) => return vec![EntryCandidate::Ordinal(*n)],
            EntryName::Symbol(s) => s.clone(),
        };
        let t = self.spec.target;
        if self.explicit_alias || t.os != OperatingSystem::Windows {
            return vec![EntryCandidate::Symbol(name)];
        }
        let mut out = match self.spec.charset {
            Charset::Ansi => vec![name.clone(), format!("{name}A")],
            Charset::Unicode => vec![format!("{name}W"), name.clone()],
        };
        if t.bitness == Bitness::B32 && self.spec.conv == CallConv::Stdcall {
            out.push(format!("_{name}@{}", self.stdcall_arg_bytes()));
        }
        out.into_iter().map(EntryCandidate::Symbol).collect()
    }
}
