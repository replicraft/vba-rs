//! Operators on Variants (MS-VBAL §5.6.9 arithmetic, concatenation,
//! relational and logical operators; type promotion as documented for VBA).
//!
//! # Operand rules shared by all operators
//!
//! * A `vbError` operand re-raises its error; objects, user-defined types and
//!   arrays are error 13 (default-property resolution is the VM's job).
//! * `Null` propagates (except in the logical short cases listed below).
//! * `Empty` acts as 0 (numeric) or `""` (string context); `Boolean` acts as
//!   an `Integer` (-1 / 0).
//! * Strings in arithmetic become `Double` (non-numeric text is error 13).
//!
//! # Overflow behavior
//!
//! Statically typed operands (`Dim a As Integer`) raise error 6 when the
//! result does not fit the result type. `Variant` operands are promoted
//! instead: Byte to Integer to Long to Double. The caller chooses with
//! [`Overflow`].
//!
//! # Result type of `+ - *`
//!
//! Decimal > Currency > Double/Date > Single > LongLong > Long > Integer >
//! Byte, where a Single combined with a LongLong gives Double. The rank order
//! of Decimal, Currency and Double is documented VBA behavior, not MS-VBAL
//! text, and has not been verified against a real VBA host.
//!
//! Date rules: `Date + x` and `x + Date` are Dates, `Date - Date` is a Double,
//! `Date - x` is a Date, `x - Date` is a Double, `*` and `/` give Doubles.
//! A Date result outside the Date range is error 6.
//!
//! `/` gives Decimal if either operand is Decimal, Currency if either is
//! Currency, Single if both are Single, otherwise Double. `\` and `Mod` round
//! both operands to integers (half to even) and give a LongLong if either
//! operand is LongLong, a Long if either is Long or a non-integer type, an
//! Integer for Integer/Boolean operands and a Byte only for Byte with Byte.
//! `^` always gives Double.

use crate::currency::Currency;
use crate::decimal::Decimal;
use crate::string::{CompareMode, VbaString};
use crate::text::{self, Numeric};
use crate::variant::Variant;
use crate::{Date, convert};
use std::cmp::Ordering;
use vba_core::{VbaError, VbaResult};

/// How integer overflow is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Statically typed operands: overflow is error 6.
    Error,
    /// Variant operands: promote Byte, Integer, Long towards Double.
    Promote,
}

/// Binary arithmetic operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `\`
    IntDiv,
    /// `Mod`
    Mod,
    /// `^`
    Pow,
}

/// Integer width classes in promotion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Byte,
    Integer,
    Long,
    LongLong,
}

impl Kind {
    fn range(self) -> (i128, i128) {
        match self {
            Kind::Byte => (0, 255),
            Kind::Integer => (i128::from(i16::MIN), i128::from(i16::MAX)),
            Kind::Long => (i128::from(i32::MIN), i128::from(i32::MAX)),
            Kind::LongLong => (i128::from(i64::MIN), i128::from(i64::MAX)),
        }
    }

    fn fits(self, v: i128) -> bool {
        let (lo, hi) = self.range();
        (lo..=hi).contains(&v)
    }

    fn make(self, v: i64) -> Variant {
        match self {
            Kind::Byte => Variant::Byte(v as u8),
            Kind::Integer => Variant::Integer(v as i16),
            Kind::Long => Variant::Long(v as i32),
            Kind::LongLong => Variant::LongLong(v),
        }
    }

    fn next(self) -> Option<Kind> {
        match self {
            Kind::Byte => Some(Kind::Integer),
            Kind::Integer => Some(Kind::Long),
            Kind::Long | Kind::LongLong => None,
        }
    }
}

/// A numeric operand after normalization.
#[derive(Debug, Clone, Copy)]
enum Val {
    Int(Kind, i64),
    Single(f32),
    Double(f64),
    Cur(Currency),
    Dec(Decimal),
    Date(f64),
}

impl Val {
    fn to_f64(self) -> f64 {
        match self {
            Val::Int(_, i) => i as f64,
            Val::Single(s) => f64::from(s),
            Val::Double(d) | Val::Date(d) => d,
            Val::Cur(c) => c.to_f64(),
            Val::Dec(d) => d.to_f64(),
        }
    }

    fn to_decimal(self) -> VbaResult<Decimal> {
        match self {
            Val::Int(_, i) => Ok(Decimal::from_i64(i)),
            Val::Single(s) => Decimal::from_f32(s),
            Val::Double(d) | Val::Date(d) => Decimal::from_f64(d),
            Val::Cur(c) => Ok(Decimal::from_currency(c)),
            Val::Dec(d) => Ok(d),
        }
    }

    fn to_currency(self) -> VbaResult<Currency> {
        match self {
            Val::Int(_, i) => Currency::from_i64(i),
            Val::Single(s) => Currency::from_f64(f64::from(s)),
            Val::Double(d) | Val::Date(d) => Currency::from_f64(d),
            Val::Cur(c) => Ok(c),
            Val::Dec(d) => d.to_currency(),
        }
    }

    /// Rounds half to even to an integer.
    fn to_i64_rounded(self) -> VbaResult<i64> {
        match self {
            Val::Int(_, i) => Ok(i),
            Val::Cur(c) => Ok(c.round_to_i64()),
            Val::Dec(d) => d.to_i64_round(),
            other => {
                let f = other.to_f64();
                if !f.is_finite() {
                    return Err(VbaError::OVERFLOW);
                }
                let r = f.round_ties_even();
                if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&r) {
                    Ok(r as i64)
                } else {
                    Err(VbaError::OVERFLOW)
                }
            }
        }
    }
}

/// Rejects error values, objects, user-defined types and arrays.
fn check_class(v: &Variant) -> VbaResult<()> {
    match v {
        Variant::Error(e) => Err(*e),
        Variant::Object(_) | Variant::UserDefined(_) | Variant::Array(_) => {
            Err(VbaError::TYPE_MISMATCH)
        }
        _ => Ok(()),
    }
}

/// Normalizes a non-Null, checked operand to a numeric value.
fn val_of(v: &Variant) -> VbaResult<Val> {
    Ok(match v {
        Variant::Empty => Val::Int(Kind::Integer, 0),
        Variant::Boolean(b) => Val::Int(Kind::Integer, if *b { -1 } else { 0 }),
        Variant::Byte(x) => Val::Int(Kind::Byte, i64::from(*x)),
        Variant::Integer(x) => Val::Int(Kind::Integer, i64::from(*x)),
        Variant::Long(x) => Val::Int(Kind::Long, i64::from(*x)),
        Variant::LongLong(x) => Val::Int(Kind::LongLong, *x),
        Variant::Single(x) => Val::Single(*x),
        Variant::Double(x) => Val::Double(*x),
        Variant::Currency(x) => Val::Cur(*x),
        Variant::Decimal(x) => Val::Dec(*x),
        Variant::Date(x) => Val::Date(x.serial()),
        Variant::String(s) => Val::Double(match text::parse_numeric(s)? {
            Numeric::Integer(i) => i as f64,
            Numeric::Real { value, .. } => value,
        }),
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
        Variant::Error(e) => return Err(*e),
        _ => return Err(VbaError::TYPE_MISMATCH),
    })
}

fn finite(f: f64) -> VbaResult<f64> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(VbaError::OVERFLOW)
    }
}

fn single_result(f: f64) -> VbaResult<Variant> {
    let s = f as f32;
    if !f.is_finite() || s.is_infinite() {
        Err(VbaError::OVERFLOW)
    } else {
        Ok(Variant::Single(s))
    }
}

/// Binary arithmetic: `l op r`.
///
/// `overflow` selects typed (error 6) or Variant (promotion) integer overflow.
/// For `+` with two String (or String and Empty) operands this is
/// concatenation.
pub fn arith(op: ArithOp, l: &Variant, r: &Variant, overflow: Overflow) -> VbaResult<Variant> {
    check_class(l)?;
    check_class(r)?;
    if op == ArithOp::Add {
        match (l, r) {
            (Variant::String(a), Variant::String(b)) => return Ok(Variant::String(a.concat(b))),
            (Variant::String(a), Variant::Empty) | (Variant::Empty, Variant::String(a)) => {
                return Ok(Variant::String(a.clone()));
            }
            _ => {}
        }
    }
    if l.is_null() || r.is_null() {
        return Ok(Variant::Null);
    }
    let a = val_of(l)?;
    let b = val_of(r)?;
    match op {
        ArithOp::Add | ArithOp::Sub | ArithOp::Mul => add_sub_mul(op, a, b, overflow),
        ArithOp::Div => divide(a, b),
        ArithOp::IntDiv | ArithOp::Mod => int_div_mod(op, a, b, overflow),
        ArithOp::Pow => power(a, b),
    }
}

fn float_op(op: ArithOp, x: f64, y: f64) -> f64 {
    match op {
        ArithOp::Add => x + y,
        ArithOp::Sub => x - y,
        _ => x * y,
    }
}

fn add_sub_mul(op: ArithOp, a: Val, b: Val, overflow: Overflow) -> VbaResult<Variant> {
    use Val::{Cur, Date as VDate, Dec, Double, Int, Single};
    if matches!(a, Dec(_)) || matches!(b, Dec(_)) {
        let (x, y) = (a.to_decimal()?, b.to_decimal()?);
        let r = match op {
            ArithOp::Add => x.checked_add(&y),
            ArithOp::Sub => x.checked_sub(&y),
            _ => x.checked_mul(&y),
        }?;
        return Ok(Variant::Decimal(r));
    }
    if matches!(a, Cur(_)) || matches!(b, Cur(_)) {
        let (x, y) = (a.to_currency()?, b.to_currency()?);
        let r = match op {
            ArithOp::Add => x.checked_add(y),
            ArithOp::Sub => x.checked_sub(y),
            _ => x.checked_mul(y),
        }?;
        return Ok(Variant::Currency(r));
    }
    if matches!(a, VDate(_)) || matches!(b, VDate(_)) {
        let r = finite(float_op(op, a.to_f64(), b.to_f64()))?;
        let both = matches!(a, VDate(_)) && matches!(b, VDate(_));
        return Ok(match op {
            ArithOp::Add => Variant::Date(Date::from_serial(r)?),
            ArithOp::Sub if both => Variant::Double(r),
            ArithOp::Sub if matches!(a, VDate(_)) => Variant::Date(Date::from_serial(r)?),
            _ => Variant::Double(r),
        });
    }
    if matches!(a, Double(_)) || matches!(b, Double(_)) {
        let r = finite(float_op(op, a.to_f64(), b.to_f64()))?;
        return Ok(Variant::Double(r));
    }
    if matches!(a, Single(_)) || matches!(b, Single(_)) {
        let wide = matches!(a, Int(Kind::LongLong, _)) || matches!(b, Int(Kind::LongLong, _));
        let r = float_op(op, a.to_f64(), b.to_f64());
        return if wide {
            Ok(Variant::Double(finite(r)?))
        } else {
            single_result(r)
        };
    }
    let (Int(ka, x), Int(kb, y)) = (a, b) else {
        return Err(VbaError::TYPE_MISMATCH);
    };
    let (x, y) = (i128::from(x), i128::from(y));
    let res = match op {
        ArithOp::Add => x + y,
        ArithOp::Sub => x - y,
        _ => x * y,
    };
    let mut kind = ka.max(kb);
    loop {
        if kind.fits(res) {
            return Ok(kind.make(res as i64));
        }
        match (overflow, kind.next()) {
            (Overflow::Promote, Some(n)) => kind = n,
            (Overflow::Promote, None) if kind == Kind::Long => {
                return Ok(Variant::Double(res as f64));
            }
            _ => return Err(VbaError::OVERFLOW),
        }
    }
}

fn divide(a: Val, b: Val) -> VbaResult<Variant> {
    use Val::{Cur, Dec, Single};
    if matches!(a, Dec(_)) || matches!(b, Dec(_)) {
        return Ok(Variant::Decimal(
            a.to_decimal()?.checked_div(&b.to_decimal()?)?,
        ));
    }
    if matches!(a, Cur(_)) || matches!(b, Cur(_)) {
        return Ok(Variant::Currency(
            a.to_currency()?.checked_div(b.to_currency()?)?,
        ));
    }
    let d = b.to_f64();
    if d == 0.0 {
        return Err(VbaError::DIVISION_BY_ZERO);
    }
    let r = a.to_f64() / d;
    if matches!(a, Single(_)) && matches!(b, Single(_)) {
        single_result(r)
    } else {
        Ok(Variant::Double(finite(r)?))
    }
}

fn int_div_mod(op: ArithOp, a: Val, b: Val, overflow: Overflow) -> VbaResult<Variant> {
    let kind_of = |v: Val| match v {
        Val::Int(k, _) => k,
        _ => Kind::Long,
    };
    let mut kind = kind_of(a).max(kind_of(b));
    let (x, y) = (a.to_i64_rounded()?, b.to_i64_rounded()?);
    // Operands must fit the working kind (non-integer operands are Long).
    let (x, y) = (i128::from(x), i128::from(y));
    if !kind.fits(x) || !kind.fits(y) {
        return Err(VbaError::OVERFLOW);
    }
    if y == 0 {
        return Err(VbaError::DIVISION_BY_ZERO);
    }
    let res = if op == ArithOp::IntDiv { x / y } else { x % y };
    loop {
        if kind.fits(res) {
            return Ok(kind.make(res as i64));
        }
        match (overflow, kind.next()) {
            (Overflow::Promote, Some(n)) => kind = n,
            _ => return Err(VbaError::OVERFLOW),
        }
    }
}

fn power(a: Val, b: Val) -> VbaResult<Variant> {
    let (x, y) = (a.to_f64(), b.to_f64());
    if x == 0.0 && y < 0.0 {
        return Err(VbaError::DIVISION_BY_ZERO);
    }
    let r = x.powf(y);
    if r.is_nan() {
        return Err(VbaError::INVALID_PROCEDURE_CALL);
    }
    Ok(Variant::Double(finite(r)?))
}

/// Unary minus.
pub fn negate(v: &Variant, overflow: Overflow) -> VbaResult<Variant> {
    check_class(v)?;
    if v.is_null() {
        return Ok(Variant::Null);
    }
    Ok(match val_of(v)? {
        Val::Int(Kind::Byte, x) => Variant::Integer(-(x as i16)),
        Val::Int(k, x) => {
            let r = -i128::from(x);
            let mut kind = k;
            loop {
                if kind.fits(r) {
                    break kind.make(r as i64);
                }
                match (overflow, kind.next()) {
                    (Overflow::Promote, Some(n)) => kind = n,
                    (Overflow::Promote, None) if kind == Kind::Long => {
                        break Variant::Double(r as f64);
                    }
                    _ => return Err(VbaError::OVERFLOW),
                }
            }
        }
        Val::Single(s) => Variant::Single(-s),
        Val::Double(d) => Variant::Double(-d),
        Val::Cur(c) => Variant::Currency(c.checked_neg()?),
        Val::Dec(d) => Variant::Decimal(d.negate()),
        Val::Date(d) => Variant::Date(Date::from_serial(-d)?),
    })
}

/// String concatenation `&`: operands are converted with `CStr` rules;
/// `Null & Null` is `Null`, a single `Null` counts as `""`.
pub fn concat(l: &Variant, r: &Variant) -> VbaResult<Variant> {
    check_class(l)?;
    check_class(r)?;
    if l.is_null() && r.is_null() {
        return Ok(Variant::Null);
    }
    let s = |v: &Variant| -> VbaResult<VbaString> {
        if v.is_null() {
            Ok(VbaString::new())
        } else {
            convert::to_vba_string(v)
        }
    };
    Ok(Variant::String(s(l)?.concat(&s(r)?)))
}

/// Relational operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `=`
    Eq,
    /// `<>`
    Ne,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
}

/// How a String is compared with a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StringNumberRule {
    /// Both operands are Variants: a number is less than a string.
    #[default]
    VariantOrdering,
    /// One operand is statically typed numeric: the string is converted to a
    /// number (error 13 if impossible).
    Coerce,
}

/// Options of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompareOptions {
    /// `Option Compare` mode for string comparison.
    pub mode: CompareMode,
    /// String versus number rule.
    pub string_number: StringNumberRule,
}

/// The ordering of two values; `Unordered` arises only from NaN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ordered {
    /// Left is less.
    Less,
    /// Equal.
    Equal,
    /// Left is greater.
    Greater,
    /// A NaN took part.
    Unordered,
}

impl From<Ordering> for Ordered {
    fn from(o: Ordering) -> Self {
        match o {
            Ordering::Less => Ordered::Less,
            Ordering::Equal => Ordered::Equal,
            Ordering::Greater => Ordered::Greater,
        }
    }
}

fn cmp_f64(a: f64, b: f64) -> Ordered {
    a.partial_cmp(&b).map_or(Ordered::Unordered, Ordered::from)
}

fn cmp_val(a: Val, b: Val) -> VbaResult<Ordered> {
    use Val::{Cur, Dec, Int};
    Ok(match (a, b) {
        (Int(_, x), Int(_, y)) => x.cmp(&y).into(),
        (Cur(x), Cur(y)) => x.cmp(&y).into(),
        (Cur(x), Int(_, y)) => i128::from(x.scaled()).cmp(&(i128::from(y) * 10_000)).into(),
        (Int(_, x), Cur(y)) => (i128::from(x) * 10_000).cmp(&i128::from(y.scaled())).into(),
        (Dec(_), _) | (_, Dec(_)) => {
            let exact = |v: Val| matches!(v, Int(..) | Cur(_) | Dec(_));
            if exact(a) && exact(b) {
                a.to_decimal()?.cmp(&b.to_decimal()?).into()
            } else {
                cmp_f64(a.to_f64(), b.to_f64())
            }
        }
        _ => cmp_f64(a.to_f64(), b.to_f64()),
    })
}

/// Compares two values; `Ok(None)` means the result is `Null` (a Null
/// operand).
pub fn compare_values(
    l: &Variant,
    r: &Variant,
    opts: CompareOptions,
) -> VbaResult<Option<Ordered>> {
    check_class(l)?;
    check_class(r)?;
    if l.is_null() || r.is_null() {
        return Ok(None);
    }
    let empty = VbaString::new();
    let text = |v: &Variant| match v {
        Variant::String(s) => Some(s.clone()),
        Variant::Empty => Some(empty.clone()),
        _ => None,
    };
    match (l, r) {
        (Variant::Empty, Variant::Empty) => return Ok(Some(Ordered::Equal)),
        (Variant::String(_), Variant::String(_))
        | (Variant::Empty, Variant::String(_))
        | (Variant::String(_), Variant::Empty) => {
            let (a, b) = (text(l), text(r));
            if let (Some(a), Some(b)) = (a, b) {
                return Ok(Some(a.compare(&b, opts.mode).into()));
            }
        }
        _ => {}
    }
    match (l, r) {
        (Variant::String(s), other) | (other, Variant::String(s)) => {
            let string_left = matches!(l, Variant::String(_));
            let flip = |o: Ordered| match (string_left, o) {
                (true, Ordered::Less) => Ordered::Greater,
                (true, Ordered::Greater) => Ordered::Less,
                (_, o) => o,
            };
            // Date versus parsable date string compares as dates.
            if let Variant::Date(d) = other
                && let Ok(sd) = text::parse_date(s)
            {
                return Ok(Some(flip(cmp_f64(d.serial(), sd.serial()))));
            }
            let num = val_of(other)?;
            match opts.string_number {
                StringNumberRule::VariantOrdering => {
                    // other (number) < string
                    Ok(Some(flip(Ordered::Less)))
                }
                StringNumberRule::Coerce => {
                    // MS-VBAL 5.6.9.5: Boolean with String compares as Boolean.
                    if matches!(other, Variant::Boolean(_)) {
                        let sb = crate::convert::to_boolean(&Variant::String(s.clone()))?;
                        return Ok(Some(flip(cmp_val(num, val_of(&Variant::Boolean(sb))?)?)));
                    }
                    let sv = val_of(&Variant::String(s.clone()))?;
                    Ok(Some(flip(cmp_val(num, sv)?)))
                }
            }
        }
        _ => Ok(Some(cmp_val(val_of(l)?, val_of(r)?)?)),
    }
}

/// Evaluates a relational operator; the result is `Boolean` or `Null`.
pub fn compare(
    op: CompareOp,
    l: &Variant,
    r: &Variant,
    opts: CompareOptions,
) -> VbaResult<Variant> {
    let Some(o) = compare_values(l, r, opts)? else {
        return Ok(Variant::Null);
    };
    let b = match (op, o) {
        (CompareOp::Ne, Ordered::Unordered) => true,
        (_, Ordered::Unordered) => false,
        (CompareOp::Eq, o) => o == Ordered::Equal,
        (CompareOp::Ne, o) => o != Ordered::Equal,
        (CompareOp::Lt, o) => o == Ordered::Less,
        (CompareOp::Gt, o) => o == Ordered::Greater,
        (CompareOp::Le, o) => matches!(o, Ordered::Less | Ordered::Equal),
        (CompareOp::Ge, o) => matches!(o, Ordered::Greater | Ordered::Equal),
    };
    Ok(Variant::Boolean(b))
}

/// Logical / bitwise binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOp {
    /// `And`
    And,
    /// `Or`
    Or,
    /// `Xor`
    Xor,
    /// `Eqv`
    Eqv,
    /// `Imp`
    Imp,
}

/// A logical operand: a Boolean or an integer of some width.
#[derive(Debug, Clone, Copy)]
enum LVal {
    Bool(bool),
    Int(Kind, i64),
}

fn lval_of(v: &Variant) -> VbaResult<LVal> {
    Ok(match v {
        Variant::Boolean(b) => LVal::Bool(*b),
        Variant::Empty => LVal::Int(Kind::Integer, 0),
        Variant::Byte(x) => LVal::Int(Kind::Byte, i64::from(*x)),
        Variant::Integer(x) => LVal::Int(Kind::Integer, i64::from(*x)),
        Variant::Long(x) => LVal::Int(Kind::Long, i64::from(*x)),
        Variant::LongLong(x) => LVal::Int(Kind::LongLong, *x),
        other => LVal::Int(
            Kind::Long,
            i64::from(
                i32::try_from(val_of(other)?.to_i64_rounded()?).map_err(|_| VbaError::OVERFLOW)?,
            ),
        ),
    })
}

fn all_ones(k: Kind) -> i64 {
    match k {
        Kind::Byte => 255,
        _ => -1,
    }
}

fn truncate_to(k: Kind, v: i64) -> i64 {
    match k {
        Kind::Byte => i64::from(v as u8),
        Kind::Integer => i64::from(v as i16),
        Kind::Long => i64::from(v as i32),
        Kind::LongLong => v,
    }
}

fn bit_op(op: LogicOp, x: i64, y: i64) -> i64 {
    match op {
        LogicOp::And => x & y,
        LogicOp::Or => x | y,
        LogicOp::Xor => x ^ y,
        LogicOp::Eqv => !(x ^ y),
        LogicOp::Imp => !x | y,
    }
}

/// Binary `And`, `Or`, `Xor`, `Eqv`, `Imp`.
///
/// Two Booleans give a Boolean; otherwise operands are integers and the
/// operation is bitwise, with the result in the wider integer type (Boolean
/// counts as Integer, non-integer types convert to Long with rounding).
/// With a `Null` operand the result is `Null` except where the other operand
/// alone decides it (`Null And False`, `Null Or True`, `False Imp Null`,
/// `Null Imp True`, and the all-zero / all-ones integer equivalents).
pub fn logic(op: LogicOp, l: &Variant, r: &Variant) -> VbaResult<Variant> {
    check_class(l)?;
    check_class(r)?;
    match (l.is_null(), r.is_null()) {
        (true, true) => return Ok(Variant::Null),
        (true, false) => return null_logic(op, true, lval_of(r)?),
        (false, true) => return null_logic(op, false, lval_of(l)?),
        _ => {}
    }
    Ok(match (lval_of(l)?, lval_of(r)?) {
        (LVal::Bool(a), LVal::Bool(b)) => Variant::Boolean(match op {
            LogicOp::And => a & b,
            LogicOp::Or => a | b,
            LogicOp::Xor => a ^ b,
            LogicOp::Eqv => a == b,
            LogicOp::Imp => !a | b,
        }),
        (a, b) => {
            let (ka, x) = widen(a);
            let (kb, y) = widen(b);
            let k = ka.max(kb);
            // Byte values are stored zero-extended and wider kinds
            // sign-extended, so an i64 operation followed by truncation to the
            // result kind is exact.
            k.make(truncate_to(k, bit_op(op, x, y)))
        }
    })
}

fn widen(v: LVal) -> (Kind, i64) {
    match v {
        LVal::Bool(b) => (Kind::Integer, if b { -1 } else { 0 }),
        LVal::Int(k, x) => (k, x),
    }
}

/// Logic with exactly one Null operand; `null_is_left` tells which side.
fn null_logic(op: LogicOp, null_is_left: bool, other: LVal) -> VbaResult<Variant> {
    let zero = |o: LVal| match o {
        LVal::Bool(b) => !b,
        LVal::Int(_, x) => x == 0,
    };
    let ones = |o: LVal| match o {
        LVal::Bool(b) => b,
        LVal::Int(k, x) => x == all_ones(k),
    };
    let make = |o: LVal, v: bool| match o {
        LVal::Bool(_) => Variant::Boolean(v),
        LVal::Int(k, _) => k.make(if v { all_ones(k) } else { 0 }),
    };
    Ok(match op {
        LogicOp::And if zero(other) => make(other, false),
        LogicOp::Or if ones(other) => make(other, true),
        // Null Imp x = (Not Null) Or x ; x Imp Null = (Not x) Or Null.
        LogicOp::Imp if null_is_left && ones(other) => make(other, true),
        LogicOp::Imp if !null_is_left && zero(other) => make(other, true),
        _ => Variant::Null,
    })
}

/// Unary `Not` (logical for Boolean, bitwise for integers).
pub fn not(v: &Variant) -> VbaResult<Variant> {
    check_class(v)?;
    if v.is_null() {
        return Ok(Variant::Null);
    }
    Ok(match lval_of(v)? {
        LVal::Bool(b) => Variant::Boolean(!b),
        LVal::Int(k, x) => k.make(truncate_to(k, !x)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::{ObjectHandle, ObjectRef};

    fn d(s: &str) -> Variant {
        Variant::Decimal(Decimal::parse(s).unwrap().unwrap())
    }
    fn cur(s: i64) -> Variant {
        Variant::Currency(Currency::from_scaled(s))
    }
    fn date(v: f64) -> Variant {
        Variant::Date(Date::from_serial(v).unwrap())
    }
    fn s(x: &str) -> Variant {
        Variant::from(x)
    }

    /// Renders a result as `Type:value` or `Err:n` for table comparison.
    fn show(r: VbaResult<Variant>) -> String {
        match r {
            Err(e) => format!("Err:{}", e.number()),
            Ok(v) => {
                let t = match &v {
                    Variant::Empty => "Empty",
                    Variant::Null => "Null",
                    Variant::Boolean(_) => "Boolean",
                    Variant::Byte(_) => "Byte",
                    Variant::Integer(_) => "Integer",
                    Variant::Long(_) => "Long",
                    Variant::LongLong(_) => "LongLong",
                    Variant::Single(_) => "Single",
                    Variant::Double(_) => "Double",
                    Variant::Currency(_) => "Currency",
                    Variant::Date(_) => "Date",
                    Variant::Decimal(_) => "Decimal",
                    Variant::String(_) => "String",
                    _ => "Other",
                };
                let text = match &v {
                    Variant::Null => String::new(),
                    Variant::Date(x) => x.serial().to_string(),
                    other => convert::to_vba_string(other)
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_default(),
                };
                format!("{t}:{text}")
            }
        }
    }

    fn run(op: ArithOp, l: Variant, r: Variant, m: Overflow) -> String {
        show(arith(op, &l, &r, m))
    }

    const V: Overflow = Overflow::Promote;
    const T: Overflow = Overflow::Error;

    #[test]
    fn add_integer_promotion_tables() {
        use ArithOp::Add;
        let i = Variant::Integer;
        let cases: Vec<(Variant, Variant, Overflow, &str)> = vec![
            (i(1), i(2), T, "Integer:3"),
            (i(32767), i(1), T, "Err:6"),
            (i(32767), i(1), V, "Long:32768"),
            (i(-32768), i(-1), V, "Long:-32769"),
            (Variant::Long(i32::MAX), Variant::Long(1), T, "Err:6"),
            (
                Variant::Long(i32::MAX),
                Variant::Long(1),
                V,
                "Double:2147483648",
            ),
            (Variant::Byte(200), Variant::Byte(100), T, "Err:6"),
            (Variant::Byte(200), Variant::Byte(100), V, "Integer:300"),
            (Variant::Byte(1), Variant::Byte(2), T, "Byte:3"),
            (Variant::Byte(1), i(2), T, "Integer:3"),
            (i(1), Variant::Long(2), T, "Long:3"),
            (Variant::Long(1), Variant::LongLong(2), T, "LongLong:3"),
            (
                Variant::LongLong(i64::MAX),
                Variant::LongLong(1),
                V,
                "Err:6",
            ),
            (
                Variant::Boolean(true),
                Variant::Boolean(true),
                T,
                "Integer:-2",
            ),
            (Variant::Boolean(true), Variant::Byte(1), T, "Integer:0"),
            (Variant::Empty, Variant::Empty, T, "Integer:0"),
            (Variant::Empty, i(5), T, "Integer:5"),
            (Variant::Empty, Variant::Long(5), T, "Long:5"),
        ];
        for (l, r, m, want) in cases {
            assert_eq!(
                run(Add, l.clone(), r.clone(), m),
                want,
                "{l:?} + {r:?} {m:?}"
            );
        }
    }

    #[test]
    fn add_sub_mul_mixed_types() {
        use ArithOp::*;
        let cases: Vec<(ArithOp, Variant, Variant, &str)> = vec![
            (Add, Variant::Long(1), Variant::Single(0.5), "Single:1.5"),
            (
                Add,
                Variant::Single(0.5),
                Variant::Double(0.25),
                "Double:0.75",
            ),
            (
                Add,
                Variant::LongLong(1),
                Variant::Single(0.5),
                "Double:1.5",
            ),
            (Add, Variant::Integer(1), Variant::Double(0.5), "Double:1.5"),
            (Add, cur(15_000), Variant::Integer(2), "Currency:3.5"),
            (Add, cur(15_000), Variant::Double(0.25), "Currency:1.75"),
            (Add, cur(15_000), Variant::Double(0.00004), "Currency:1.5"),
            (Mul, cur(15_000), cur(15_000), "Currency:2.25"),
            (Mul, cur(i64::MAX), Variant::Integer(2), "Err:6"),
            (Add, d("1.5"), Variant::Integer(2), "Decimal:3.5"),
            (Add, d("1.5"), cur(5_000), "Decimal:2.0000"),
            (Add, d("0.1"), Variant::Double(0.2), "Decimal:0.3"),
            (
                Mul,
                d("79228162514264337593543950335"),
                Variant::Integer(2),
                "Err:6",
            ),
            (Sub, Variant::Integer(1), Variant::Integer(3), "Integer:-2"),
            (Sub, Variant::Byte(1), Variant::Byte(3), "Integer:-2"),
            (
                Mul,
                Variant::Integer(300),
                Variant::Integer(200),
                "Long:60000",
            ),
            (Mul, Variant::Double(1e200), Variant::Double(1e200), "Err:6"),
            (Mul, Variant::Single(3e38), Variant::Single(10.0), "Err:6"),
            (
                Add,
                Variant::Double(f64::MAX),
                Variant::Double(f64::MAX),
                "Err:6",
            ),
            (Mul, Variant::Double(-0.0), Variant::Integer(1), "Double:0"),
        ];
        for (op, l, r, want) in cases {
            assert_eq!(run(op, l.clone(), r.clone(), V), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn string_and_empty_rules_for_add() {
        use ArithOp::*;
        for (op, l, r, want) in [
            (Add, s("a"), s("b"), "String:ab"),
            (Add, s("1"), s("2"), "String:12"),
            (Add, s("1"), Variant::Integer(2), "Double:3"),
            (Add, Variant::Integer(2), s("1.5"), "Double:3.5"),
            (Add, s("abc"), Variant::Integer(1), "Err:13"),
            (Add, s(""), Variant::Integer(1), "Err:13"),
            (Add, Variant::Empty, s("x"), "String:x"),
            (Add, s("x"), Variant::Empty, "String:x"),
            (Add, Variant::Empty, s("5"), "String:5"),
            (Sub, s("5"), s("2"), "Double:3"),
            (Mul, s("&H10"), Variant::Integer(2), "Double:32"),
            (Sub, s("x"), s("2"), "Err:13"),
            (Add, Variant::Null, Variant::Integer(1), "Null:"),
            (Add, s("x"), Variant::Null, "Null:"),
            (Add, Variant::Null, Variant::Null, "Null:"),
            (Mul, Variant::Null, Variant::Integer(0), "Null:"),
            (
                Add,
                Variant::Error(VbaError::new(11)),
                Variant::Integer(1),
                "Err:11",
            ),
            (
                Add,
                Variant::Null,
                Variant::Error(VbaError::new(11)),
                "Err:11",
            ),
            (
                Add,
                Variant::Object(ObjectRef::Nothing),
                Variant::Integer(1),
                "Err:13",
            ),
            (
                Add,
                Variant::Object(ObjectRef::Handle(ObjectHandle(1))),
                Variant::Null,
                "Err:13",
            ),
        ] {
            assert_eq!(run(op, l.clone(), r.clone(), V), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn date_arithmetic() {
        use ArithOp::*;
        for (op, l, r, want) in [
            (Add, date(43831.0), Variant::Integer(1), "Date:43832"),
            (Add, Variant::Integer(1), date(43831.0), "Date:43832"),
            (Add, date(43831.0), Variant::Double(0.5), "Date:43831.5"),
            (Add, date(1.0), date(2.0), "Date:3"),
            (Sub, date(43832.0), date(43831.0), "Double:1"),
            (Sub, date(43832.0), Variant::Integer(1), "Date:43831"),
            (Sub, Variant::Integer(5), date(3.0), "Double:2"),
            (Mul, date(2.0), Variant::Integer(3), "Double:6"),
            (Add, date(2_958_465.0), Variant::Integer(1), "Err:6"),
            (Sub, date(-657_434.0), Variant::Integer(1), "Err:6"),
            (Add, date(1.0), s("1"), "Date:2"),
        ] {
            assert_eq!(run(op, l.clone(), r.clone(), V), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn division() {
        use ArithOp::Div;
        for (l, r, want) in [
            (Variant::Integer(1), Variant::Integer(2), "Double:0.5"),
            (Variant::Integer(6), Variant::Integer(3), "Double:2"),
            (
                Variant::Long(1),
                Variant::Long(3),
                "Double:0.333333333333333",
            ),
            (Variant::Single(1.0), Variant::Single(4.0), "Single:0.25"),
            (Variant::Single(1.0), Variant::Integer(4), "Double:0.25"),
            (Variant::Double(1.0), Variant::Integer(0), "Err:11"),
            (Variant::Integer(0), Variant::Integer(0), "Err:11"),
            (Variant::Double(1.0), Variant::Double(-0.0), "Err:11"),
            (Variant::Double(1e300), Variant::Double(1e-300), "Err:6"),
            (cur(10_000), cur(30_000), "Currency:0.3333"),
            (cur(10_000), Variant::Integer(4), "Currency:0.25"),
            (cur(10_000), Variant::Integer(0), "Err:11"),
            (
                d("1"),
                Variant::Integer(3),
                "Decimal:0.3333333333333333333333333333",
            ),
            (d("1"), Variant::Integer(0), "Err:11"),
            (Variant::Empty, Variant::Integer(2), "Double:0"),
            (Variant::Null, Variant::Integer(0), "Null:"),
            (s("9"), s("3"), "Double:3"),
            (Variant::Boolean(true), Variant::Boolean(true), "Double:1"),
        ] {
            assert_eq!(run(Div, l.clone(), r.clone(), V), want, "{l:?} / {r:?}");
        }
    }

    #[test]
    fn integer_division_and_mod() {
        use ArithOp::{IntDiv, Mod};
        for (op, l, r, m, want) in [
            (
                IntDiv,
                Variant::Integer(7),
                Variant::Integer(2),
                V,
                "Integer:3",
            ),
            (
                IntDiv,
                Variant::Integer(-7),
                Variant::Integer(2),
                V,
                "Integer:-3",
            ),
            (
                IntDiv,
                Variant::Double(7.5),
                Variant::Integer(2),
                V,
                "Long:4",
            ),
            (
                IntDiv,
                Variant::Double(6.5),
                Variant::Double(2.5),
                V,
                "Long:3",
            ),
            (IntDiv, Variant::Byte(7), Variant::Byte(2), V, "Byte:3"),
            (
                IntDiv,
                Variant::Byte(7),
                Variant::Integer(2),
                V,
                "Integer:3",
            ),
            (IntDiv, Variant::Long(7), Variant::Integer(2), V, "Long:3"),
            (
                IntDiv,
                Variant::LongLong(1 << 40),
                Variant::Integer(2),
                V,
                "LongLong:549755813888",
            ),
            (
                IntDiv,
                Variant::Integer(1),
                Variant::Integer(0),
                V,
                "Err:11",
            ),
            (
                IntDiv,
                Variant::Double(0.4),
                Variant::Double(0.4),
                V,
                "Err:11",
            ),
            (
                IntDiv,
                Variant::Integer(i16::MIN),
                Variant::Integer(-1),
                T,
                "Err:6",
            ),
            (
                IntDiv,
                Variant::Integer(i16::MIN),
                Variant::Integer(-1),
                V,
                "Long:32768",
            ),
            (
                IntDiv,
                Variant::Double(3e9),
                Variant::Integer(2),
                V,
                "Err:6",
            ),
            (
                IntDiv,
                Variant::Double(f64::NAN),
                Variant::Integer(2),
                V,
                "Err:6",
            ),
            (IntDiv, s("9"), Variant::Integer(2), V, "Long:4"),
            (
                IntDiv,
                Variant::Boolean(true),
                Variant::Integer(1),
                V,
                "Integer:-1",
            ),
            (IntDiv, Variant::Null, Variant::Integer(0), V, "Null:"),
            (IntDiv, Variant::Empty, Variant::Integer(2), V, "Integer:0"),
            (
                Mod,
                Variant::Integer(7),
                Variant::Integer(3),
                V,
                "Integer:1",
            ),
            (
                Mod,
                Variant::Integer(-7),
                Variant::Integer(3),
                V,
                "Integer:-1",
            ),
            (
                Mod,
                Variant::Integer(7),
                Variant::Integer(-3),
                V,
                "Integer:1",
            ),
            (Mod, Variant::Double(7.5), Variant::Double(3.2), V, "Long:2"),
            (Mod, Variant::Double(8.5), Variant::Integer(3), V, "Long:2"),
            (
                Mod,
                Variant::Integer(i16::MIN),
                Variant::Integer(-1),
                V,
                "Integer:0",
            ),
            (Mod, Variant::Integer(1), Variant::Integer(0), V, "Err:11"),
            (Mod, Variant::Double(2.5), Variant::Double(0.5), V, "Err:11"),
            (Mod, Variant::Byte(10), Variant::Byte(4), V, "Byte:2"),
            (Mod, cur(75_000), Variant::Integer(2), V, "Long:0"),
            (Mod, d("7.5"), Variant::Integer(3), V, "Long:2"),
            (Mod, Variant::Null, Variant::Integer(3), V, "Null:"),
        ] {
            assert_eq!(run(op, l.clone(), r.clone(), m), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn power() {
        use ArithOp::Pow;
        for (l, r, want) in [
            (Variant::Integer(2), Variant::Integer(10), "Double:1024"),
            (Variant::Integer(2), Variant::Integer(-1), "Double:0.5"),
            (Variant::Integer(0), Variant::Integer(0), "Double:1"),
            (Variant::Integer(0), Variant::Integer(-1), "Err:11"),
            (Variant::Integer(-8), Variant::Double(0.5), "Err:5"),
            (Variant::Integer(-8), Variant::Double(3.0), "Double:-512"),
            (Variant::Double(10.0), Variant::Integer(400), "Err:6"),
            (Variant::Double(10.0), Variant::Integer(-400), "Double:0"),
            (Variant::Null, Variant::Integer(2), "Null:"),
            (s("3"), Variant::Integer(2), "Double:9"),
            (Variant::Empty, Variant::Integer(2), "Double:0"),
        ] {
            assert_eq!(run(Pow, l.clone(), r.clone(), V), want, "{l:?} ^ {r:?}");
        }
    }

    #[test]
    fn unary_minus() {
        for (v, m, want) in [
            (Variant::Integer(5), T, "Integer:-5"),
            (Variant::Integer(i16::MIN), T, "Err:6"),
            (Variant::Integer(i16::MIN), V, "Long:32768"),
            (Variant::Long(i32::MIN), V, "Double:2147483648"),
            (Variant::Long(i32::MIN), T, "Err:6"),
            (Variant::LongLong(i64::MIN), V, "Err:6"),
            (Variant::Byte(5), T, "Integer:-5"),
            (Variant::Byte(255), T, "Integer:-255"),
            (Variant::Boolean(true), T, "Integer:1"),
            (Variant::Boolean(false), T, "Integer:0"),
            (Variant::Empty, T, "Integer:0"),
            (Variant::Double(1.5), T, "Double:-1.5"),
            (Variant::Single(1.5), T, "Single:-1.5"),
            (s("2"), T, "Double:-2"),
            (s("x"), T, "Err:13"),
            (Variant::Null, T, "Null:"),
            (cur(15_000), T, "Currency:-1.5"),
            (cur(i64::MIN), T, "Err:6"),
            (d("1.5"), T, "Decimal:-1.5"),
            (date(2.0), T, "Date:-2"),
        ] {
            assert_eq!(show(negate(&v, m)), want, "-{v:?}");
        }
    }

    #[test]
    fn concatenation() {
        for (l, r, want) in [
            (s("a"), s("b"), "String:ab"),
            (s("a"), Variant::Integer(1), "String:a1"),
            (Variant::Integer(1), Variant::Integer(2), "String:12"),
            (Variant::Null, Variant::Null, "Null:"),
            (Variant::Null, s("x"), "String:x"),
            (s("x"), Variant::Null, "String:x"),
            (Variant::Empty, Variant::Empty, "String:"),
            (Variant::Boolean(true), s("!"), "String:True!"),
            (Variant::Double(0.5), s("x"), "String:0.5x"),
            (date(43831.0), s("x"), "String:1/1/2020x"),
            (Variant::Error(VbaError::new(5)), s("x"), "Err:5"),
            (Variant::Object(ObjectRef::Nothing), s("x"), "Err:13"),
        ] {
            assert_eq!(show(concat(&l, &r)), want, "{l:?} & {r:?}");
        }
    }

    fn cmp(op: CompareOp, l: Variant, r: Variant) -> String {
        show(compare(op, &l, &r, CompareOptions::default()))
    }

    #[test]
    fn comparisons() {
        use CompareOp::*;
        for (op, l, r, want) in [
            (Eq, Variant::Integer(1), Variant::Long(1), "Boolean:True"),
            (
                Eq,
                Variant::Integer(1),
                Variant::Double(1.0),
                "Boolean:True",
            ),
            (
                Lt,
                Variant::Integer(1),
                Variant::Double(1.5),
                "Boolean:True",
            ),
            (
                Ge,
                Variant::Integer(2),
                Variant::Double(1.5),
                "Boolean:True",
            ),
            (
                Ne,
                Variant::Integer(2),
                Variant::Integer(2),
                "Boolean:False",
            ),
            (
                Eq,
                Variant::Boolean(true),
                Variant::Integer(-1),
                "Boolean:True",
            ),
            (Eq, Variant::Empty, Variant::Empty, "Boolean:True"),
            (Eq, Variant::Empty, Variant::Integer(0), "Boolean:True"),
            (Eq, Variant::Empty, s(""), "Boolean:True"),
            (Lt, Variant::Empty, s("a"), "Boolean:True"),
            (Eq, Variant::Empty, Variant::Boolean(false), "Boolean:True"),
            (Eq, s("a"), s("a"), "Boolean:True"),
            (Lt, s("a"), s("b"), "Boolean:True"),
            (Lt, s("B"), s("a"), "Boolean:True"),
            (Eq, s("a"), s("A"), "Boolean:False"),
            (Lt, Variant::Integer(100), s("2"), "Boolean:True"),
            (Gt, s("1"), Variant::Integer(100), "Boolean:True"),
            (Eq, s("1"), Variant::Integer(1), "Boolean:False"),
            (Eq, cur(10_000), Variant::Integer(1), "Boolean:True"),
            (Lt, cur(9_999), Variant::Integer(1), "Boolean:True"),
            (Eq, d("1.0"), Variant::Integer(1), "Boolean:True"),
            (Eq, d("0.1"), Variant::Double(0.1), "Boolean:True"),
            (Lt, d("0.1"), d("0.11"), "Boolean:True"),
            (
                Eq,
                Variant::LongLong(i64::MAX),
                Variant::LongLong(i64::MAX - 1),
                "Boolean:False",
            ),
            (Eq, date(1.0), Variant::Double(1.0), "Boolean:True"),
            (Eq, date(43831.0), s("1/1/2020"), "Boolean:True"),
            (Lt, date(43831.0), s("1/2/2020"), "Boolean:True"),
            (
                Eq,
                Variant::Double(f64::NAN),
                Variant::Double(f64::NAN),
                "Boolean:False",
            ),
            (
                Ne,
                Variant::Double(f64::NAN),
                Variant::Double(f64::NAN),
                "Boolean:True",
            ),
            (
                Lt,
                Variant::Double(f64::NAN),
                Variant::Double(1.0),
                "Boolean:False",
            ),
            (
                Eq,
                Variant::Double(0.0),
                Variant::Double(-0.0),
                "Boolean:True",
            ),
            (Eq, Variant::Null, Variant::Integer(1), "Null:"),
            (Ne, Variant::Null, Variant::Null, "Null:"),
            (Lt, s("a"), Variant::Null, "Null:"),
            (
                Eq,
                Variant::Error(VbaError::new(5)),
                Variant::Integer(1),
                "Err:5",
            ),
            (
                Eq,
                Variant::Object(ObjectRef::Nothing),
                Variant::Integer(1),
                "Err:13",
            ),
        ] {
            assert_eq!(cmp(op, l.clone(), r.clone()), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn text_compare_mode_and_coerce_rule() {
        let text = CompareOptions {
            mode: CompareMode::Text,
            ..Default::default()
        };
        assert_eq!(
            show(compare(CompareOp::Eq, &s("abc"), &s("ABC"), text)),
            "Boolean:True"
        );
        let coerce = CompareOptions {
            string_number: StringNumberRule::Coerce,
            ..Default::default()
        };
        assert_eq!(
            show(compare(
                CompareOp::Eq,
                &s("1"),
                &Variant::Integer(1),
                coerce
            )),
            "Boolean:True"
        );
        assert_eq!(
            show(compare(
                CompareOp::Lt,
                &Variant::Integer(100),
                &s("2"),
                coerce
            )),
            "Boolean:False"
        );
        assert_eq!(
            show(compare(
                CompareOp::Eq,
                &s("x"),
                &Variant::Integer(1),
                coerce
            )),
            "Err:13"
        );
    }

    fn lg(op: LogicOp, l: Variant, r: Variant) -> String {
        show(logic(op, &l, &r))
    }

    #[test]
    fn logical_operators() {
        use LogicOp::*;
        let t = Variant::Boolean(true);
        let f = Variant::Boolean(false);
        let n = Variant::Null;
        for (op, l, r, want) in [
            (And, t.clone(), t.clone(), "Boolean:True"),
            (And, t.clone(), f.clone(), "Boolean:False"),
            (Or, f.clone(), f.clone(), "Boolean:False"),
            (Or, t.clone(), f.clone(), "Boolean:True"),
            (Xor, t.clone(), t.clone(), "Boolean:False"),
            (Xor, t.clone(), f.clone(), "Boolean:True"),
            (Eqv, f.clone(), f.clone(), "Boolean:True"),
            (Eqv, t.clone(), f.clone(), "Boolean:False"),
            (Imp, t.clone(), f.clone(), "Boolean:False"),
            (Imp, f.clone(), f.clone(), "Boolean:True"),
            (Imp, f.clone(), t.clone(), "Boolean:True"),
            (And, Variant::Integer(12), Variant::Integer(10), "Integer:8"),
            (Or, Variant::Integer(12), Variant::Integer(10), "Integer:14"),
            (Xor, Variant::Integer(12), Variant::Integer(10), "Integer:6"),
            (
                Eqv,
                Variant::Integer(12),
                Variant::Integer(10),
                "Integer:-7",
            ),
            (
                Imp,
                Variant::Integer(12),
                Variant::Integer(10),
                "Integer:-5",
            ),
            (And, Variant::Byte(0xF0), Variant::Byte(0x3C), "Byte:48"),
            (Or, Variant::Byte(0xF0), Variant::Byte(0x0F), "Byte:255"),
            (
                And,
                Variant::Byte(0xFF),
                Variant::Integer(-1),
                "Integer:255",
            ),
            (Or, Variant::Byte(1), Variant::Integer(-256), "Integer:-255"),
            (
                And,
                Variant::Integer(-1),
                Variant::Long(70000),
                "Long:70000",
            ),
            (And, Variant::Integer(5), Variant::Long(3), "Long:1"),
            (
                And,
                Variant::Long(-1),
                Variant::LongLong(1 << 40),
                "LongLong:1099511627776",
            ),
            (And, t.clone(), Variant::Integer(5), "Integer:5"),
            (And, f.clone(), Variant::Long(5), "Long:0"),
            (Or, f.clone(), Variant::Long(5), "Long:5"),
            (And, Variant::Double(6.5), Variant::Integer(3), "Long:2"),
            (And, s("7"), Variant::Integer(3), "Long:3"),
            (And, Variant::Empty, Variant::Empty, "Integer:0"),
            (Or, Variant::Empty, t.clone(), "Integer:-1"),
            (And, Variant::Double(3e10), Variant::Integer(1), "Err:6"),
            (And, s("x"), Variant::Integer(1), "Err:13"),
            (And, n.clone(), t.clone(), "Null:"),
            (And, n.clone(), f.clone(), "Boolean:False"),
            (And, f.clone(), n.clone(), "Boolean:False"),
            (And, n.clone(), Variant::Integer(0), "Integer:0"),
            (And, n.clone(), Variant::Integer(5), "Null:"),
            (Or, n.clone(), t.clone(), "Boolean:True"),
            (Or, n.clone(), f.clone(), "Null:"),
            (Or, Variant::Integer(-1), n.clone(), "Integer:-1"),
            (Or, Variant::Integer(5), n.clone(), "Null:"),
            (Xor, n.clone(), t.clone(), "Null:"),
            (Eqv, n.clone(), n.clone(), "Null:"),
            (Imp, n.clone(), t.clone(), "Boolean:True"),
            (Imp, n.clone(), f.clone(), "Null:"),
            (Imp, f.clone(), n.clone(), "Boolean:True"),
            (Imp, t.clone(), n.clone(), "Null:"),
            (Imp, n.clone(), n.clone(), "Null:"),
            (And, Variant::Error(VbaError::new(5)), t.clone(), "Err:5"),
            (
                And,
                Variant::Object(ObjectRef::Nothing),
                t.clone(),
                "Err:13",
            ),
        ] {
            assert_eq!(lg(op, l.clone(), r.clone()), want, "{l:?} {op:?} {r:?}");
        }
    }

    #[test]
    fn not_operator() {
        for (v, want) in [
            (Variant::Boolean(true), "Boolean:False"),
            (Variant::Boolean(false), "Boolean:True"),
            (Variant::Integer(0), "Integer:-1"),
            (Variant::Integer(5), "Integer:-6"),
            (Variant::Byte(0), "Byte:255"),
            (Variant::Byte(0xF0), "Byte:15"),
            (Variant::Long(0), "Long:-1"),
            (Variant::LongLong(0), "LongLong:-1"),
            (Variant::Empty, "Integer:-1"),
            (Variant::Double(2.5), "Long:-3"),
            (s("3"), "Long:-4"),
            (Variant::Null, "Null:"),
            (s("x"), "Err:13"),
            (Variant::Error(VbaError::new(7)), "Err:7"),
        ] {
            assert_eq!(show(not(&v)), want, "Not {v:?}");
        }
    }
}
