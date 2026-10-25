//! The VBA `Currency` type: a signed 64-bit integer scaled by 10 000
//! (MS-VBAL §2.2, documented range -922337203685477.5808 to
//! 922337203685477.5807).
//!
//! Arithmetic is exact. Where a result needs more than four decimal places
//! (multiplication, division) it is rounded half to even; this matches the
//! documented banker's rounding of VBA conversions, but the exact tie rule of
//! the native `Currency` multiply is documented behavior, not spec text.

use crate::error::{VbaError, VbaResult};
use std::fmt;

/// Number of units per whole currency unit.
pub const SCALE: i64 = 10_000;

/// A `Currency` value (scaled integer, four implied decimal places).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Currency(i64);

/// Divides `n` by `d` (`d != 0`) rounding half to even.
pub(crate) fn div_round_half_even(n: i128, d: i128) -> i128 {
    let q = n / d;
    let r = n % d;
    if r == 0 {
        return q;
    }
    let twice = r.abs() * 2;
    let away = if n < 0 { -1 } else { 1 } * if d < 0 { -1 } else { 1 };
    match twice.cmp(&d.abs()) {
        std::cmp::Ordering::Greater => q + away,
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Equal => {
            if q % 2 == 0 {
                q
            } else {
                q + away
            }
        }
    }
}

impl Currency {
    /// Largest representable value.
    pub const MAX: Currency = Currency(i64::MAX);
    /// Smallest representable value.
    pub const MIN: Currency = Currency(i64::MIN);
    /// Zero.
    pub const ZERO: Currency = Currency(0);

    /// Creates a value from the raw scaled representation (units of 0.0001).
    pub const fn from_scaled(scaled: i64) -> Self {
        Self(scaled)
    }

    /// Returns the raw scaled representation (units of 0.0001).
    pub const fn scaled(self) -> i64 {
        self.0
    }

    /// Converts a whole number; overflow is error 6.
    pub fn from_i64(v: i64) -> VbaResult<Self> {
        v.checked_mul(SCALE).map(Self).ok_or(VbaError::OVERFLOW)
    }

    /// Converts a double, rounding half to even at four decimals.
    /// NaN, infinities and out-of-range values are error 6.
    pub fn from_f64(v: f64) -> VbaResult<Self> {
        if !v.is_finite() {
            return Err(VbaError::OVERFLOW);
        }
        let s = (v * SCALE as f64).round_ties_even();
        // 2^63 as f64 is exactly representable; the valid range is [-2^63, 2^63).
        if !(-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&s) {
            return Err(VbaError::OVERFLOW);
        }
        Ok(Self(s as i64))
    }

    /// Converts to a double (may lose precision above 2^53 scaled units).
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / SCALE as f64
    }

    /// Rounds half to even to a whole number.
    pub fn round_to_i64(self) -> i64 {
        div_round_half_even(i128::from(self.0), i128::from(SCALE)) as i64
    }

    /// `self + rhs`; overflow is error 6.
    pub fn checked_add(self, rhs: Self) -> VbaResult<Self> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(VbaError::OVERFLOW)
    }

    /// `self - rhs`; overflow is error 6.
    pub fn checked_sub(self, rhs: Self) -> VbaResult<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(VbaError::OVERFLOW)
    }

    /// `self * rhs`, rounded half to even to four decimals; overflow is error 6.
    pub fn checked_mul(self, rhs: Self) -> VbaResult<Self> {
        let p = i128::from(self.0) * i128::from(rhs.0);
        let r = div_round_half_even(p, i128::from(SCALE));
        i64::try_from(r).map(Self).map_err(|_| VbaError::OVERFLOW)
    }

    /// `self / rhs`, rounded half to even to four decimals.
    /// Division by zero is error 11, overflow is error 6.
    pub fn checked_div(self, rhs: Self) -> VbaResult<Self> {
        if rhs.0 == 0 {
            return Err(VbaError::DIVISION_BY_ZERO);
        }
        let n = i128::from(self.0) * i128::from(SCALE);
        let r = div_round_half_even(n, i128::from(rhs.0));
        i64::try_from(r).map(Self).map_err(|_| VbaError::OVERFLOW)
    }

    /// `-self`; overflow (negating the minimum) is error 6.
    pub fn checked_neg(self) -> VbaResult<Self> {
        self.0.checked_neg().map(Self).ok_or(VbaError::OVERFLOW)
    }
}

impl fmt::Display for Currency {
    /// Plain decimal text with trailing fractional zeros removed
    /// (`CStr(CCur(1.5))` is `1.5`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let whole = abs / SCALE as u64;
        let frac = abs % SCALE as u64;
        let sign = if self.0 < 0 { "-" } else { "" };
        if frac == 0 {
            write!(f, "{sign}{whole}")
        } else {
            let digits = format!("{frac:04}");
            write!(f, "{sign}{whole}.{}", digits.trim_end_matches('0'))
        }
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Currency({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: i64) -> Currency {
        Currency::from_scaled(s)
    }

    #[test]
    fn display() {
        assert_eq!(c(15_000).to_string(), "1.5");
        assert_eq!(c(0).to_string(), "0");
        assert_eq!(c(-5).to_string(), "-0.0005");
        assert_eq!(c(i64::MAX).to_string(), "922337203685477.5807");
        assert_eq!(c(i64::MIN).to_string(), "-922337203685477.5808");
    }

    #[test]
    fn from_f64_rounds_half_even() {
        assert_eq!(Currency::from_f64(0.00005).unwrap().scaled(), 0);
        assert_eq!(Currency::from_f64(1.23456).unwrap().scaled(), 12_346);
        assert_eq!(Currency::from_f64(-1.5).unwrap().scaled(), -15_000);
    }

    #[test]
    fn from_f64_errors() {
        for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e15, -1e15] {
            assert_eq!(Currency::from_f64(v), Err(VbaError::OVERFLOW), "{v}");
        }
        assert!(Currency::from_f64(922_337_203_685_477.0).is_ok());
    }

    #[test]
    fn add_sub_overflow() {
        assert_eq!(c(i64::MAX).checked_add(c(1)), Err(VbaError::OVERFLOW));
        assert_eq!(c(i64::MIN).checked_sub(c(1)), Err(VbaError::OVERFLOW));
        assert_eq!(c(10_000).checked_add(c(5_000)).unwrap(), c(15_000));
        assert_eq!(c(i64::MIN).checked_neg(), Err(VbaError::OVERFLOW));
    }

    #[test]
    fn mul_div() {
        // 1.5 * 2.5 = 3.75
        assert_eq!(c(15_000).checked_mul(c(25_000)).unwrap(), c(37_500));
        // 0.0001 * 0.5 = 0.00005 -> tie rounds to even (0)
        assert_eq!(c(1).checked_mul(c(5_000)).unwrap(), c(0));
        // 0.0003 * 0.5 = 0.00015 -> tie rounds to even (2)
        assert_eq!(c(3).checked_mul(c(5_000)).unwrap(), c(2));
        // 1 / 3 = 0.3333
        assert_eq!(c(10_000).checked_div(c(30_000)).unwrap(), c(3_333));
        // -2 / 3 = -0.6667
        assert_eq!(c(-20_000).checked_div(c(30_000)).unwrap(), c(-6_667));
        assert_eq!(c(1).checked_div(c(0)), Err(VbaError::DIVISION_BY_ZERO));
        assert_eq!(c(i64::MAX).checked_mul(c(20_000)), Err(VbaError::OVERFLOW));
        assert_eq!(c(i64::MAX).checked_div(c(5_000)), Err(VbaError::OVERFLOW));
    }

    #[test]
    fn round_to_whole() {
        assert_eq!(c(25_000).round_to_i64(), 2);
        assert_eq!(c(35_000).round_to_i64(), 4);
        assert_eq!(c(-25_000).round_to_i64(), -2);
        assert_eq!(c(-25_001).round_to_i64(), -3);
    }

    #[test]
    fn from_i64_bounds() {
        assert_eq!(
            Currency::from_i64(922_337_203_685_477).unwrap().scaled(),
            9_223_372_036_854_770_000
        );
        assert_eq!(
            Currency::from_i64(922_337_203_685_478),
            Err(VbaError::OVERFLOW)
        );
    }
}
