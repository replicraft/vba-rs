//! `VBA.Math` (MS-VBAL §6.1.2.8 and the Microsoft language reference).
//!
//! `Rnd`/`Randomize` implement the generator documented in Microsoft KB
//! 231847 ("How Visual Basic generates pseudo-random numbers"): a 24-bit
//! linear congruential generator
//! `seed = (seed * 1140671485 + 12820163) mod 2^24`, `Rnd = seed / 2^24`,
//! initial seed `0x50000`. The seeding rules of `Rnd(negative)` and
//! `Randomize n` follow the Visual Basic runtime and are not verified
//! against Office (see `docs/language/vba-stdlib.md`).

use crate::call::{Args, Ctx};
use crate::digits::{Dec, Rounding};
use crate::error::{StdError, StdResult};
use vba_value::{Currency, Decimal, Variant, VbaError, VbaResult, convert};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

/// State of the `Rnd` generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RndState {
    seed: u32,
}

impl Default for RndState {
    fn default() -> Self {
        RndState { seed: 0x50000 }
    }
}

impl RndState {
    /// The raw 24-bit seed.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    fn step(&mut self) {
        let next = (u64::from(self.seed) * 1_140_671_485 + 12_820_163) & 0xFF_FFFF;
        self.seed = next as u32;
    }

    fn value(&self) -> f32 {
        self.seed as f32 / 16_777_216.0
    }

    /// `Rnd(x)`: positive advances the sequence, zero repeats the last
    /// number, negative reseeds from the bits of `x` (the same `x` always
    /// yields the same number) and then advances.
    pub fn rnd(&mut self, x: f32) -> f32 {
        if x != 0.0 {
            if x < 0.0 {
                let bits = u64::from(x.to_bits());
                self.seed = ((bits + (bits >> 24)) & 0xFF_FFFF) as u32;
            }
            self.step();
        }
        self.value()
    }

    /// `Randomize n`: mixes the high 32 bits of the double into the middle
    /// 16 bits of the seed.
    pub fn randomize(&mut self, n: f64) {
        let hi = (n.to_bits() >> 32) as u32;
        let mixed = ((hi & 0xFFFF) ^ (hi >> 16)) << 8;
        self.seed = (self.seed & 0xFF00_00FF) | (mixed & 0x00FF_FF00);
    }
}

/// Normalizes an argument to a numeric subtype: strings become `Double`,
/// `Empty` is `Integer` 0, `Boolean` is `Integer`, `Date` is `Double`.
fn numeric(v: &Variant) -> VbaResult<Variant> {
    Ok(match v {
        Variant::Byte(_)
        | Variant::Integer(_)
        | Variant::Long(_)
        | Variant::LongLong(_)
        | Variant::Single(_)
        | Variant::Double(_)
        | Variant::Currency(_)
        | Variant::Decimal(_) => v.clone(),
        Variant::Empty => Variant::Integer(0),
        Variant::Boolean(b) => Variant::Integer(if *b { -1 } else { 0 }),
        Variant::Date(d) => Variant::Double(d.serial()),
        Variant::String(_) | Variant::Error(_) => Variant::Double(convert::to_double(v)?),
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
        _ => return Err(VbaError::TYPE_MISMATCH),
    })
}

/// `Abs`: keeps the subtype; `Integer`/`Long` minimum values overflow (6).
pub fn abs(v: &Variant) -> VbaResult<Variant> {
    Ok(match numeric(v)? {
        Variant::Integer(x) => Variant::Integer(x.checked_abs().ok_or(VbaError::OVERFLOW)?),
        Variant::Long(x) => Variant::Long(x.checked_abs().ok_or(VbaError::OVERFLOW)?),
        Variant::LongLong(x) => Variant::LongLong(x.checked_abs().ok_or(VbaError::OVERFLOW)?),
        Variant::Single(x) => Variant::Single(x.abs()),
        Variant::Double(x) => Variant::Double(x.abs()),
        Variant::Currency(c) if c.scaled() < 0 => Variant::Currency(c.checked_neg()?),
        Variant::Decimal(d) => Variant::Decimal(d.abs()),
        other => other,
    })
}

/// `Sgn`: -1, 0 or 1 as `Integer`.
pub fn sgn(v: &Variant) -> VbaResult<Variant> {
    let s = match numeric(v)? {
        Variant::Byte(x) => i16::from(x != 0),
        Variant::Integer(x) => x.signum(),
        Variant::Long(x) => x.signum() as i16,
        Variant::LongLong(x) => x.signum() as i16,
        Variant::Single(x) => sign_f64(f64::from(x)),
        Variant::Double(x) => sign_f64(x),
        Variant::Currency(c) => c.scaled().signum() as i16,
        Variant::Decimal(d) => {
            if d.is_zero() {
                0
            } else if d.is_negative() {
                -1
            } else {
                1
            }
        }
        _ => unreachable!("numeric() normalizes"),
    };
    Ok(Variant::Integer(s))
}

fn sign_f64(x: f64) -> i16 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}

/// `Int` (floor) when `floor` is true, `Fix` (truncate toward zero) otherwise.
pub fn int_fix(v: &Variant, floor: bool) -> VbaResult<Variant> {
    Ok(match numeric(v)? {
        Variant::Single(x) => Variant::Single(if floor { x.floor() } else { x.trunc() }),
        Variant::Double(x) => Variant::Double(if floor { x.floor() } else { x.trunc() }),
        Variant::Currency(c) => {
            let s = c.scaled();
            let unit = vba_value::currency::SCALE;
            let q = if floor { s.div_euclid(unit) } else { s / unit };
            Variant::Currency(Currency::from_scaled(q * unit))
        }
        Variant::Decimal(d) => Variant::Decimal(if floor { d.floor() } else { d.truncate() }),
        other => other,
    })
}

/// `Round(x, places)` with banker's rounding (ties to even) performed on the
/// 15-significant-digit decimal value of the number, so `Round(0.285, 2)` is
/// `0.28` and `Round(2.5)` is `2`. Negative `places` is error 5.
pub fn round(v: &Variant, places: i32) -> VbaResult<Variant> {
    if places < 0 {
        return Err(E5);
    }
    let rd = |x: f64| -> f64 {
        if !x.is_finite() || places > 20 {
            return x;
        }
        let r = Dec::from_f64(x).round(places, Rounding::HalfEven).to_f64();
        if x.is_sign_negative() { -r } else { r }
    };
    Ok(match numeric(v)? {
        Variant::Single(x) => Variant::Single(rd(f64::from(x)) as f32),
        Variant::Double(x) => Variant::Double(rd(x)),
        Variant::Currency(c) => {
            let d = Decimal::from_currency(c).round(places.min(4) as u8);
            Variant::Currency(d.to_currency()?)
        }
        Variant::Decimal(d) => Variant::Decimal(d.round(places.min(28) as u8)),
        other => other,
    })
}

fn finite(x: f64) -> VbaResult<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(VbaError::OVERFLOW)
    }
}

/// `Sqr`: negative argument is error 5.
pub fn sqr(x: f64) -> VbaResult<f64> {
    if x < 0.0 { Err(E5) } else { Ok(x.sqrt()) }
}

/// `Log` (natural): argument <= 0 is error 5.
pub fn log(x: f64) -> VbaResult<f64> {
    if x <= 0.0 { Err(E5) } else { Ok(x.ln()) }
}

/// `Exp`: results beyond the double range are error 6.
pub fn exp(x: f64) -> VbaResult<f64> {
    finite(x.exp())
}

// ---------------------------------------------------------------------------
// Registry adapters.

fn num_arg(a: &Args<'_>) -> StdResult<Variant> {
    Ok(numeric(a.req(0)?)?)
}

pub(crate) fn f_abs(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Ok(Variant::Null);
    }
    Ok(abs(a.req(0)?)?)
}
pub(crate) fn f_sgn(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Ok(Variant::Null);
    }
    Ok(sgn(a.req(0)?)?)
}
pub(crate) fn f_int(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Ok(Variant::Null);
    }
    Ok(int_fix(a.req(0)?, true)?)
}
pub(crate) fn f_fix(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Ok(Variant::Null);
    }
    Ok(int_fix(a.req(0)?, false)?)
}
pub(crate) fn f_round(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    num_arg(a)?;
    Ok(round(a.req(0)?, a.long_or(1, 0)?)?)
}
pub(crate) fn f_sqr(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(sqr(a.double(0)?)?))
}
pub(crate) fn f_log(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(log(a.double(0)?)?))
}
pub(crate) fn f_exp(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(exp(a.double(0)?)?))
}
pub(crate) fn f_sin(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(a.double(0)?.sin()))
}
pub(crate) fn f_cos(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(a.double(0)?.cos()))
}
pub(crate) fn f_tan(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(finite(a.double(0)?.tan())?))
}
pub(crate) fn f_atn(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(a.double(0)?.atan()))
}
pub(crate) fn f_rnd(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let x = a.double_or(0, 1.0)? as f32;
    if !x.is_finite() {
        return Err(StdError::Vba(VbaError::OVERFLOW));
    }
    Ok(Variant::Single(ctx.state.rnd.rnd(x)))
}
pub(crate) fn f_randomize(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let n = match a.get(0) {
        Some(v) => convert::to_double(v)?,
        None => f64::from(ctx.host.timer()),
    };
    ctx.state.rnd.randomize(n);
    Ok(Variant::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Microsoft KB 231847: with the initial seed the first five `Rnd` values
    /// printed to seven digits.
    #[test]
    fn documented_default_sequence() {
        let mut r = RndState::default();
        let got: Vec<String> = (0..5).map(|_| format!("{:.7}", r.rnd(1.0))).collect();
        assert_eq!(
            got,
            [
                "0.7055475",
                "0.5334240",
                "0.5795186",
                "0.2895625",
                "0.3019480"
            ]
        );
    }

    #[test]
    fn rnd_zero_repeats_and_negative_is_repeatable() {
        let mut r = RndState::default();
        let a = r.rnd(1.0);
        assert_eq!(r.rnd(0.0), a);
        let x = r.rnd(-5.0);
        let y = r.rnd(-5.0);
        assert_eq!(x, y);
        let z = r.rnd(1.0);
        r.rnd(-5.0);
        assert_eq!(r.rnd(1.0), z);
    }

    #[test]
    fn rounding() {
        let d = |x: f64, p| round(&Variant::Double(x), p).unwrap();
        assert_eq!(d(2.5, 0), Variant::Double(2.0));
        assert_eq!(d(3.5, 0), Variant::Double(4.0));
        assert_eq!(d(-2.5, 0), Variant::Double(-2.0));
        assert_eq!(d(0.125, 2), Variant::Double(0.12));
        assert_eq!(d(1234.5678, 2), Variant::Double(1234.57));
        assert_eq!(round(&Variant::Double(1.0), -1), Err(E5));
    }
}
