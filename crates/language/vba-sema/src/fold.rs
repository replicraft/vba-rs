//! Literal typing and constant folding (MS-VBAL §3.3, §5.6).

use vba_ast::{BinaryOp, CompareMode, Expr, ExprKind, Ident, Literal, Radix, UnaryOp};
use vba_lexer::TypeSuffix;
use vba_profile::{Bitness, Feature, Profile};
use vba_source::Span;
use vba_value::ops::{
    self, ArithOp, CompareOp, CompareOptions, LogicOp, Overflow, StringNumberRule,
};
use vba_value::{Currency, Variant, VbaString, text};

use crate::bound::Constant;
use crate::types::Type;

/// Decodes literal bytes: UTF-8 when valid, otherwise Latin-1 (the code page
/// is not known at this level; see `docs/sema.md`).
pub fn decode_bytes(b: &[u8]) -> VbaString {
    match std::str::from_utf8(b) {
        Ok(s) => VbaString::from(s),
        Err(_) => VbaString::from_units(b.iter().map(|&c| u16::from(c)).collect()),
    }
}

/// The static type of a variant subtype (for folded values).
pub fn type_of_variant(v: &Variant) -> Type {
    match v {
        Variant::Boolean(_) => Type::Boolean,
        Variant::Byte(_) => Type::Byte,
        Variant::Integer(_) => Type::Integer,
        Variant::Long(_) => Type::Long,
        Variant::LongLong(_) => Type::LongLong,
        Variant::Single(_) => Type::Single,
        Variant::Double(_) => Type::Double,
        Variant::Currency(_) => Type::Currency,
        Variant::Date(_) => Type::Date,
        Variant::Decimal(_) => Type::Decimal,
        Variant::String(_) => Type::String,
        Variant::Object(_) => Type::Nothing,
        _ => Type::Variant,
    }
}

/// Types a literal (§3.3.2 for integer literals).
///
/// Decimal integer literals without a suffix are `Integer`, then `Long`,
/// then `Double` (not `LongLong`: that needs the `^` suffix). `&H`/`&O`
/// literals are 16-bit `Integer` when they fit 16 bits (so `&HFFFF` is -1),
/// else 32-bit `Long`.
pub fn literal(lit: &Literal, profile: &Profile) -> Result<(Constant, Type), String> {
    use Constant::Value;
    Ok(match lit {
        Literal::Integer {
            value,
            radix,
            suffix,
        } => {
            let Some(mag) = *value else {
                return Err("integer literal is too large".to_owned());
            };
            integer_literal(mag, *radix, *suffix, profile)?
        }
        Literal::Float { value, suffix } => match suffix {
            Some(TypeSuffix::Single) => (Value(Variant::Single(*value as f32)), Type::Single),
            Some(TypeSuffix::Currency) => {
                let c = Currency::from_f64(*value).map_err(|_| "Currency literal overflows")?;
                (Value(Variant::Currency(c)), Type::Currency)
            }
            _ => (Value(Variant::Double(*value)), Type::Double),
        },
        Literal::String(b) => (Value(Variant::String(decode_bytes(b))), Type::String),
        Literal::Date(t) => {
            let s = decode_bytes(t);
            match text::parse_date(&s) {
                Ok(d) => (Value(Variant::Date(d)), Type::Date),
                Err(_) => (Constant::DateText(t.clone()), Type::Date),
            }
        }
        Literal::True => (Value(Variant::Boolean(true)), Type::Boolean),
        Literal::False => (Value(Variant::Boolean(false)), Type::Boolean),
        Literal::Nothing => (
            Value(Variant::Object(vba_value::ObjectRef::Nothing)),
            Type::Nothing,
        ),
        Literal::Empty => (Value(Variant::Empty), Type::Variant),
        Literal::Null => (Value(Variant::Null), Type::Variant),
    })
}

fn integer_literal(
    mag: u64,
    radix: Radix,
    suffix: Option<TypeSuffix>,
    profile: &Profile,
) -> Result<(Constant, Type), String> {
    use Constant::Value;
    let decimal = radix == Radix::Decimal;
    match suffix {
        Some(TypeSuffix::Integer) => {
            let v = narrow(mag, decimal, 16)?;
            Ok((Value(Variant::Integer(v as i16)), Type::Integer))
        }
        Some(TypeSuffix::Long) => {
            let v = narrow(mag, decimal, 32)?;
            Ok((Value(Variant::Long(v as i32)), Type::Long))
        }
        Some(TypeSuffix::LongLong) => {
            if !profile.supports(Feature::LongLong) {
                return Err("LongLong literals need a 64-bit VBA7 profile".to_owned());
            }
            let v = narrow(mag, decimal, 64)?;
            Ok((Value(Variant::LongLong(v)), Type::LongLong))
        }
        Some(TypeSuffix::Single) => Ok((Value(Variant::Single(mag as f32)), Type::Single)),
        Some(TypeSuffix::Double) => Ok((Value(Variant::Double(mag as f64)), Type::Double)),
        Some(TypeSuffix::Currency) => {
            let c =
                Currency::from_i64(i64::try_from(mag).map_err(|_| "Currency literal overflows")?)
                    .map_err(|_| "Currency literal overflows")?;
            Ok((Value(Variant::Currency(c)), Type::Currency))
        }
        Some(TypeSuffix::String) => Err("`$` is not a numeric type suffix".to_owned()),
        None => {
            if decimal {
                if mag <= i16::MAX as u64 {
                    Ok((Value(Variant::Integer(mag as i16)), Type::Integer))
                } else if mag <= i32::MAX as u64 {
                    Ok((Value(Variant::Long(mag as i32)), Type::Long))
                } else {
                    Ok((Value(Variant::Double(mag as f64)), Type::Double))
                }
            } else if mag <= 0xFFFF {
                Ok((Value(Variant::Integer(mag as u16 as i16)), Type::Integer))
            } else if mag <= 0xFFFF_FFFF {
                Ok((Value(Variant::Long(mag as u32 as i32)), Type::Long))
            } else if profile.supports(Feature::LongLong) {
                Ok((Value(Variant::LongLong(mag as i64)), Type::LongLong))
            } else {
                Err("hexadecimal or octal literal exceeds 32 bits".to_owned())
            }
        }
    }
}

/// Narrows a magnitude to a signed `bits`-wide value; non-decimal literals
/// wrap into the sign bit (`&HFFFF&` is not -1 for `Long` because 16-bit
/// patterns are zero extended by the `&` suffix).
fn narrow(mag: u64, decimal: bool, bits: u32) -> Result<i64, String> {
    let max_pos = (1u64 << (bits - 1)) - 1;
    if mag <= max_pos {
        return Ok(mag as i64);
    }
    if !decimal {
        let full = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        if mag <= full {
            return Ok(match bits {
                16 => i64::from(mag as u16 as i16),
                32 => i64::from(mag as u32 as i32),
                _ => mag as i64,
            });
        }
    }
    Err("integer literal overflows its type".to_owned())
}

/// The `LongPtr` type for a profile.
pub fn long_ptr(profile: &Profile) -> Type {
    if profile.target.bitness == Bitness::B64 {
        Type::LongLong
    } else {
        Type::Long
    }
}

/// An error found while folding. An empty message marks an error that has
/// already been reported (a use of a constant whose definition failed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldError {
    /// Where.
    pub span: Span,
    /// What.
    pub message: String,
}

/// The result of looking a name up for constant evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstLookup {
    /// A constant.
    Found(Constant, Type),
    /// The name exists but is not constant (message describes it).
    NotConstant(String),
    /// The name does not exist.
    Missing,
    /// The name is ambiguous.
    Ambiguous,
    /// The constant exists but its evaluation failed (already reported).
    Failed,
}

/// What folding needs from its surroundings.
pub trait ConstScope {
    /// Looks up a (possibly qualified) name; `path` has one element for a
    /// simple name.
    fn lookup(&self, path: &[&Ident]) -> ConstLookup;
    /// The profile.
    fn profile(&self) -> &Profile;
    /// `Option Compare` mode in force.
    fn compare(&self) -> CompareMode;
}

fn path_of<'e>(e: &'e Expr, out: &mut Vec<&'e Ident>) -> bool {
    match &e.kind {
        ExprKind::Name(i) => {
            out.push(i);
            true
        }
        ExprKind::Member {
            target: Some(t),
            name,
        } => {
            if !path_of(t, out) {
                return false;
            }
            out.push(name);
            true
        }
        _ => false,
    }
}

fn err(span: Span, msg: impl Into<String>) -> FoldError {
    FoldError {
        span,
        message: msg.into(),
    }
}

/// Folds a constant expression.
pub fn fold(scope: &dyn ConstScope, e: &Expr) -> Result<(Constant, Type), FoldError> {
    match &e.kind {
        ExprKind::Literal(l) => literal(l, scope.profile()).map_err(|m| err(e.span, m)),
        ExprKind::Paren(inner) => fold(scope, inner),
        ExprKind::Name(_) | ExprKind::Member { .. } => {
            let mut path = Vec::new();
            if !path_of(e, &mut path) {
                return Err(err(e.span, "constant expression required"));
            }
            match scope.lookup(&path) {
                ConstLookup::Found(c, t) => Ok((c, t)),
                ConstLookup::NotConstant(what) => Err(err(
                    e.span,
                    format!("constant expression required ({what} is not a constant)"),
                )),
                ConstLookup::Missing => Err(err(
                    e.span,
                    "constant expression required: name is not defined",
                )),
                ConstLookup::Ambiguous => Err(err(e.span, "ambiguous name in constant expression")),
                // Already reported where the constant is defined.
                ConstLookup::Failed => Err(err(e.span, "")),
            }
        }
        ExprKind::Unary { op, operand } => {
            let (c, _) = fold(scope, operand)?;
            let Constant::Value(v) = c else {
                return Err(err(e.span, "date constant cannot be folded here"));
            };
            let r = match op {
                UnaryOp::Neg => ops::negate(&v, Overflow::Error),
                UnaryOp::Plus => Ok(v),
                UnaryOp::Not => ops::not(&v),
            }
            .map_err(|x| err(e.span, format!("constant expression: {}", x.description())))?;
            let t = type_of_variant(&r);
            Ok((Constant::Value(r), t))
        }
        ExprKind::Binary { op, lhs, rhs } => {
            let (l, _) = fold(scope, lhs)?;
            let (r, _) = fold(scope, rhs)?;
            let (Constant::Value(l), Constant::Value(r)) = (l, r) else {
                return Err(err(e.span, "date constant cannot be folded here"));
            };
            let res = fold_binary(*op, &l, &r, scope.compare()).map_err(|m| err(e.span, m))?;
            let t = type_of_variant(&res);
            Ok((Constant::Value(res), t))
        }
        _ => Err(err(e.span, "constant expression required")),
    }
}

fn fold_binary(
    op: BinaryOp,
    l: &Variant,
    r: &Variant,
    compare: CompareMode,
) -> Result<Variant, String> {
    let res = match op {
        BinaryOp::Add => ops::arith(ArithOp::Add, l, r, Overflow::Error),
        BinaryOp::Sub => ops::arith(ArithOp::Sub, l, r, Overflow::Error),
        BinaryOp::Mul => ops::arith(ArithOp::Mul, l, r, Overflow::Error),
        BinaryOp::Div => ops::arith(ArithOp::Div, l, r, Overflow::Error),
        BinaryOp::IntDiv => ops::arith(ArithOp::IntDiv, l, r, Overflow::Error),
        BinaryOp::Mod => ops::arith(ArithOp::Mod, l, r, Overflow::Error),
        BinaryOp::Pow => ops::arith(ArithOp::Pow, l, r, Overflow::Error),
        BinaryOp::Concat => ops::concat(l, r),
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
            let cop = match op {
                BinaryOp::Eq => CompareOp::Eq,
                BinaryOp::Ne => CompareOp::Ne,
                BinaryOp::Lt => CompareOp::Lt,
                BinaryOp::Gt => CompareOp::Gt,
                BinaryOp::Le => CompareOp::Le,
                _ => CompareOp::Ge,
            };
            ops::compare(
                cop,
                l,
                r,
                CompareOptions {
                    mode: compare_mode(compare),
                    string_number: StringNumberRule::Coerce,
                },
            )
        }
        BinaryOp::And => ops::logic(LogicOp::And, l, r),
        BinaryOp::Or => ops::logic(LogicOp::Or, l, r),
        BinaryOp::Xor => ops::logic(LogicOp::Xor, l, r),
        BinaryOp::Eqv => ops::logic(LogicOp::Eqv, l, r),
        BinaryOp::Imp => ops::logic(LogicOp::Imp, l, r),
        BinaryOp::Like | BinaryOp::Is => {
            return Err("operator is not allowed in a constant expression".to_owned());
        }
    };
    res.map_err(|x| format!("constant expression: {}", x.description()))
}

/// Maps `Option Compare` to the value library's mode (`Database` compares
/// like `Text`).
pub fn compare_mode(m: CompareMode) -> vba_value::CompareMode {
    match m {
        CompareMode::Binary => vba_value::CompareMode::Binary,
        CompareMode::Text | CompareMode::Database => vba_value::CompareMode::Text,
    }
}

/// Converts a constant to a declared scalar type (as `Const x As T = v`).
pub fn convert_constant(c: &Constant, to: &Type) -> Result<Constant, String> {
    let Some(vt) = to.var_type() else {
        return Err(format!("a constant cannot have type {to}"));
    };
    match c {
        Constant::Value(v) => ops_convert(v, vt).map(Constant::Value),
        Constant::DateText(_) => {
            if matches!(to, Type::Date | Type::Variant) {
                Ok(c.clone())
            } else {
                Err(
                    "a date literal that is not statically evaluable cannot be converted"
                        .to_owned(),
                )
            }
        }
    }
}

fn ops_convert(v: &Variant, vt: vba_value::VarType) -> Result<Variant, String> {
    vba_value::convert::convert_to(v, vt).map_err(|e| e.description().to_owned())
}

/// Converts a folded constant to an `i32` (bounds, enum values).
pub fn constant_to_i32(c: &Constant) -> Result<i32, String> {
    match c {
        Constant::Value(v) => {
            vba_value::convert::to_long(v).map_err(|e| e.description().to_owned())
        }
        Constant::DateText(_) => Err("not an integer".to_owned()),
    }
}
