//! The VBA `Decimal` type: sign, 96-bit unsigned mantissa and a decimal scale
//! of 0 to 28 (MS-VBAL §2.2; layout as the OLE `DECIMAL` structure).
//!
//! This is an independent implementation. Documented rules (taken from the
//! behavior of `VarDecAdd`/`VarDecMul`/`VarDecDiv`, not from MS-VBAL text):
//!
//! * `+`/`-`: the result scale is the larger operand scale.
//! * `*`: the result scale is the sum of the operand scales.
//! * `/`: the result carries as many fractional digits as fit in 96 bits and
//!   a scale of at most 28; an exact quotient has trailing zeros removed
//!   (down to scale 0).
//! * Whenever a result exceeds 96 bits or scale 28, low digits are dropped and
//!   the value is rounded half to even. If this would need a negative scale the
//!   result is error 6 (Overflow).
//! * Division by zero is error 11.
//!
//! Equality and ordering compare numeric value, not representation
//! (`1.0` equals `1.00`).

use crate::currency::{Currency, SCALE as CURRENCY_SCALE};
use std::cmp::Ordering;
use std::fmt;
use vba_core::{VbaError, VbaResult};

/// 2^96: the exclusive upper bound of the mantissa.
const LIMIT: u128 = 1u128 << 96;
/// Maximum scale.
pub const MAX_SCALE: u8 = 28;

/// 256-bit unsigned integer used for intermediate results.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct U256([u64; 4]);

impl U256 {
    const ZERO: U256 = U256([0; 4]);

    fn from_u128(v: u128) -> Self {
        U256([v as u64, (v >> 64) as u64, 0, 0])
    }

    fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }

    /// Returns the value if it is below 2^96.
    fn to_mantissa(self) -> Option<u128> {
        if self.0[2] != 0 || self.0[3] != 0 || self.0[1] >> 32 != 0 {
            None
        } else {
            Some(u128::from(self.0[0]) | (u128::from(self.0[1]) << 64))
        }
    }

    /// Multiplies by a small factor; `None` if the result exceeds 256 bits.
    fn mul_small(self, m: u64) -> Option<Self> {
        let mut out = [0u64; 4];
        let mut carry = 0u128;
        for (o, &limb) in out.iter_mut().zip(self.0.iter()) {
            let p = u128::from(limb) * u128::from(m) + carry;
            *o = p as u64;
            carry = p >> 64;
        }
        if carry != 0 { None } else { Some(U256(out)) }
    }

    fn add_small(self, a: u64) -> Self {
        let mut out = self.0;
        let mut carry = u128::from(a);
        for o in out.iter_mut() {
            let s = u128::from(*o) + carry;
            *o = s as u64;
            carry = s >> 64;
        }
        U256(out)
    }

    fn add(self, other: Self) -> Self {
        let mut out = [0u64; 4];
        let mut carry = 0u128;
        for (o, (&a, &b)) in out.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            let s = u128::from(a) + u128::from(b) + carry;
            *o = s as u64;
            carry = s >> 64;
        }
        U256(out)
    }

    /// `self - other`, requires `self >= other`.
    fn sub(self, other: Self) -> Self {
        let mut out = [0u64; 4];
        let mut borrow = 0u64;
        for (o, (&a, &b)) in out.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            let (d1, b1) = a.overflowing_sub(b);
            let (d2, b2) = d1.overflowing_sub(borrow);
            *o = d2;
            borrow = u64::from(b1 || b2);
        }
        U256(out)
    }

    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => {}
                o => return o,
            }
        }
        Ordering::Equal
    }

    fn divrem10(self) -> (Self, u8) {
        let mut out = [0u64; 4];
        let mut rem = 0u128;
        for i in (0..4).rev() {
            let cur = (rem << 64) | u128::from(self.0[i]);
            out[i] = (cur / 10) as u64;
            rem = cur % 10;
        }
        (U256(out), rem as u8)
    }

    fn is_odd(&self) -> bool {
        self.0[0] & 1 == 1
    }

    /// Long division by a value below 2^97.
    fn divrem_u128(self, d: u128) -> (Self, u128) {
        let mut q = [0u64; 4];
        let mut r = 0u128;
        for bit in (0..256).rev() {
            let b = (self.0[bit / 64] >> (bit % 64)) & 1;
            r = (r << 1) | u128::from(b);
            if r >= d {
                r -= d;
                q[bit / 64] |= 1 << (bit % 64);
            }
        }
        (U256(q), r)
    }

    fn mul128(a: u128, b: u128) -> Self {
        // a, b < 2^96 so each fits in two 64-bit limbs.
        let al = [a as u64, (a >> 64) as u64];
        let bl = [b as u64, (b >> 64) as u64];
        let mut out = [0u64; 4];
        for (i, &x) in al.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &y) in bl.iter().enumerate() {
                let cur = u128::from(out[i + j]) + u128::from(x) * u128::from(y) + carry;
                out[i + j] = cur as u64;
                carry = cur >> 64;
            }
            let mut k = i + bl.len();
            while carry != 0 && k < 4 {
                let cur = u128::from(out[k]) + carry;
                out[k] = cur as u64;
                carry = cur >> 64;
                k += 1;
            }
        }
        U256(out)
    }
}

/// A VBA `Decimal` value.
#[derive(Clone, Copy)]
pub struct Decimal {
    mag: u128,
    scale: u8,
    neg: bool,
}

impl Decimal {
    /// Zero (scale 0).
    pub const ZERO: Decimal = Decimal {
        mag: 0,
        scale: 0,
        neg: false,
    };
    /// 79228162514264337593543950335.
    pub const MAX: Decimal = Decimal {
        mag: LIMIT - 1,
        scale: 0,
        neg: false,
    };
    /// -79228162514264337593543950335.
    pub const MIN: Decimal = Decimal {
        mag: LIMIT - 1,
        scale: 0,
        neg: true,
    };

    /// Builds a value from its parts. The mantissa must be below 2^96 and the
    /// scale at most 28, otherwise error 6.
    pub fn from_parts(mantissa: u128, scale: u8, negative: bool) -> VbaResult<Self> {
        if mantissa >= LIMIT || scale > MAX_SCALE {
            return Err(VbaError::OVERFLOW);
        }
        Ok(Self {
            mag: mantissa,
            scale,
            neg: negative,
        })
    }

    /// The unsigned 96-bit mantissa.
    pub fn mantissa(&self) -> u128 {
        self.mag
    }

    /// The decimal scale (0 to 28).
    pub fn scale(&self) -> u8 {
        self.scale
    }

    /// `true` if the sign bit is set (a negative zero is possible only when
    /// constructed explicitly).
    pub fn is_negative(&self) -> bool {
        self.neg
    }

    /// `true` if the numeric value is zero.
    pub fn is_zero(&self) -> bool {
        self.mag == 0
    }

    /// Exact conversion from an integer.
    pub fn from_i64(v: i64) -> Self {
        Self {
            mag: u128::from(v.unsigned_abs()),
            scale: 0,
            neg: v < 0,
        }
    }

    /// Exact conversion from a currency value (scale 4, trailing zeros kept
    /// only if needed to preserve four places).
    pub fn from_currency(c: Currency) -> Self {
        Self {
            mag: u128::from(c.scaled().unsigned_abs()),
            scale: 4,
            neg: c.scaled() < 0,
        }
    }

    /// Normalizes an arbitrary-size magnitude and scale into a valid decimal.
    fn normalize(mut mag: U256, mut scale: i32, neg: bool) -> VbaResult<Self> {
        while scale < 0 {
            mag = mag.mul_small(10).ok_or(VbaError::OVERFLOW)?;
            scale += 1;
        }
        loop {
            let mut last = 0u8;
            let mut sticky = false;
            while scale > i32::from(MAX_SCALE) || mag.to_mantissa().is_none() {
                if scale == 0 {
                    return Err(VbaError::OVERFLOW);
                }
                let (q, d) = mag.divrem10();
                sticky |= last != 0;
                last = d;
                mag = q;
                scale -= 1;
            }
            if last > 5 || (last == 5 && (sticky || mag.is_odd())) {
                mag = mag.add_small(1);
            }
            if mag.to_mantissa().is_some() {
                break;
            }
        }
        let m = mag.to_mantissa().ok_or(VbaError::OVERFLOW)?;
        Ok(Self {
            mag: m,
            scale: scale as u8,
            neg: neg && m != 0,
        })
    }

    /// Builds a value from decimal digits: `digits * 10^exp10`, rounding half
    /// to even when more than 28 places are needed. `digits` must be ASCII
    /// digits only (it may be empty, meaning zero).
    pub fn from_digits(negative: bool, digits: &str, exp10: i32) -> VbaResult<Self> {
        let mut mag = U256::ZERO;
        let mut scale = -i64::from(exp10);
        let mut sticky = false;
        for (i, ch) in digits.bytes().enumerate() {
            debug_assert!(ch.is_ascii_digit());
            let d = u64::from(ch - b'0');
            // Beyond 60 digits further digits only affect rounding stickiness.
            if i >= 60 {
                sticky |= d != 0;
                scale -= 1;
                continue;
            }
            mag = mag.mul_small(10).ok_or(VbaError::OVERFLOW)?.add_small(d);
        }
        if sticky {
            // Make the (discarded) tail break ties correctly.
            mag = mag.mul_small(10).ok_or(VbaError::OVERFLOW)?.add_small(1);
            scale += 1;
        }
        if mag.is_zero() {
            return Ok(Self {
                mag: 0,
                scale: scale.clamp(0, i64::from(MAX_SCALE)) as u8,
                neg: false,
            });
        }
        let scale = i32::try_from(scale).map_err(|_| VbaError::OVERFLOW)?;
        Self::normalize(mag, scale, negative)
    }

    /// Parses plain decimal text: optional sign, digits, optional fraction and
    /// optional `E`/`e` exponent. Returns `None` for malformed text and error 6
    /// for out-of-range values.
    pub fn parse(s: &str) -> Option<VbaResult<Self>> {
        let b = s.as_bytes();
        let mut i = 0;
        let mut neg = false;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            neg = b[i] == b'-';
            i += 1;
        }
        let mut digits = String::new();
        let mut frac = 0i32;
        let mut seen_digit = false;
        while i < b.len() && b[i].is_ascii_digit() {
            digits.push(b[i] as char);
            seen_digit = true;
            i += 1;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                digits.push(b[i] as char);
                frac += 1;
                seen_digit = true;
                i += 1;
            }
        }
        if !seen_digit {
            return None;
        }
        let mut exp = 0i32;
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            i += 1;
            let mut eneg = false;
            if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
                eneg = b[i] == b'-';
                i += 1;
            }
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                exp = exp
                    .saturating_mul(10)
                    .saturating_add(i32::from(b[i] - b'0'));
                i += 1;
            }
            if start == i {
                return None;
            }
            if eneg {
                exp = -exp;
            }
        }
        if i != b.len() {
            return None;
        }
        Some(Self::from_digits(neg, &digits, exp.saturating_sub(frac)))
    }

    /// Converts a double as `VarDecFromR8` does: rounded to 15 significant
    /// digits, values with magnitude below 1e-28 become zero.
    pub fn from_f64(v: f64) -> VbaResult<Self> {
        Self::from_float_digits(v, 14)
    }

    /// Converts a single as `VarDecFromR4` does (7 significant digits).
    pub fn from_f32(v: f32) -> VbaResult<Self> {
        Self::from_float_digits(f64::from(v), 6)
    }

    fn from_float_digits(v: f64, frac_digits: usize) -> VbaResult<Self> {
        if !v.is_finite() || v.abs() >= 7.922_816_251_426_434e28 {
            return Err(VbaError::OVERFLOW);
        }
        if v == 0.0 {
            return Ok(Self::ZERO);
        }
        let text = format!("{:.*e}", frac_digits, v.abs());
        let (mant, exp) = text.split_once('e').ok_or(VbaError::OVERFLOW)?;
        let exp: i32 = exp.parse().map_err(|_| VbaError::OVERFLOW)?;
        let all: String = mant.chars().filter(char::is_ascii_digit).collect();
        let digits = all.trim_end_matches('0');
        let stripped = (all.len() - digits.len()) as i32;
        let r = Self::from_digits(v < 0.0, digits, exp - frac_digits as i32 + stripped)?;
        if r.mag == 0 {
            return Ok(Self::ZERO);
        }
        Ok(r)
    }

    /// Converts to a double.
    pub fn to_f64(&self) -> f64 {
        let m = self.mag as f64 / 10f64.powi(i32::from(self.scale));
        if self.neg { -m } else { m }
    }

    fn pow10(n: u8) -> u128 {
        10u128.pow(u32::from(n))
    }

    /// Rounds half to even to `places` decimal places (scale is reduced, never
    /// increased).
    pub fn round(&self, places: u8) -> Self {
        if self.scale <= places {
            return *self;
        }
        let div = Self::pow10(self.scale - places);
        let q = crate::currency::div_round_half_even(self.mag as i128, div as i128) as u128;
        Self {
            mag: q,
            scale: places,
            neg: self.neg && q != 0,
        }
    }

    /// Removes the fractional part (rounds toward zero), as `Fix`.
    pub fn truncate(&self) -> Self {
        if self.scale == 0 {
            return *self;
        }
        let q = self.mag / Self::pow10(self.scale);
        Self {
            mag: q,
            scale: 0,
            neg: self.neg && q != 0,
        }
    }

    /// Rounds toward negative infinity to a whole number, as `Int`.
    pub fn floor(&self) -> Self {
        let t = self.truncate();
        if self.neg && t.numeric_cmp(self) != Ordering::Equal {
            // t > self for negatives with a fractional part.
            Self {
                mag: t.mag + 1,
                scale: 0,
                neg: true,
            }
        } else {
            t
        }
    }

    /// Rounds half to even to a whole number and returns it as `i64`;
    /// error 6 if it does not fit.
    pub fn to_i64_round(&self) -> VbaResult<i64> {
        let r = self.round(0);
        let m = i128::try_from(r.mag).map_err(|_| VbaError::OVERFLOW)?;
        let v = if r.neg { -m } else { m };
        i64::try_from(v).map_err(|_| VbaError::OVERFLOW)
    }

    /// Rounds half to even to four places and returns the scaled currency;
    /// error 6 if it does not fit.
    pub fn to_currency(&self) -> VbaResult<Currency> {
        let r = self.round(4);
        let mut m = r.mag;
        for _ in r.scale..4 {
            m = m.checked_mul(10).ok_or(VbaError::OVERFLOW)?;
        }
        let m = i128::try_from(m).map_err(|_| VbaError::OVERFLOW)?;
        let v = if r.neg { -m } else { m };
        i64::try_from(v)
            .map(Currency::from_scaled)
            .map_err(|_| VbaError::OVERFLOW)
    }

    /// Negation (the sign of zero is always positive).
    pub fn negate(&self) -> Self {
        Self {
            neg: !self.neg && self.mag != 0,
            ..*self
        }
    }

    /// Absolute value.
    pub fn abs(&self) -> Self {
        Self {
            neg: false,
            ..*self
        }
    }

    fn aligned(a: &Self, b: &Self) -> (U256, U256, i32) {
        let s = a.scale.max(b.scale);
        let mut x = U256::from_u128(a.mag);
        let mut y = U256::from_u128(b.mag);
        for _ in a.scale..s {
            x = x.mul_small(10).unwrap_or(x);
        }
        for _ in b.scale..s {
            y = y.mul_small(10).unwrap_or(y);
        }
        (x, y, i32::from(s))
    }

    /// Numeric comparison.
    pub fn numeric_cmp(&self, other: &Self) -> Ordering {
        let an = self.neg && self.mag != 0;
        let bn = other.neg && other.mag != 0;
        match (an, bn) {
            (false, true) => return Ordering::Greater,
            (true, false) => return Ordering::Less,
            _ => {}
        }
        let (x, y, _) = Self::aligned(self, other);
        let o = x.cmp(&y);
        if an { o.reverse() } else { o }
    }

    /// `self + rhs`.
    pub fn checked_add(&self, rhs: &Self) -> VbaResult<Self> {
        let (x, y, s) = Self::aligned(self, rhs);
        if self.neg == rhs.neg {
            Self::normalize(x.add(y), s, self.neg)
        } else {
            match x.cmp(&y) {
                Ordering::Equal => Self::normalize(U256::ZERO, s, false),
                Ordering::Greater => Self::normalize(x.sub(y), s, self.neg),
                Ordering::Less => Self::normalize(y.sub(x), s, rhs.neg),
            }
        }
    }

    /// `self - rhs`.
    pub fn checked_sub(&self, rhs: &Self) -> VbaResult<Self> {
        self.checked_add(&rhs.negate())
    }

    /// `self * rhs`.
    pub fn checked_mul(&self, rhs: &Self) -> VbaResult<Self> {
        let p = U256::mul128(self.mag, rhs.mag);
        Self::normalize(
            p,
            i32::from(self.scale) + i32::from(rhs.scale),
            self.neg != rhs.neg,
        )
    }

    /// `self / rhs`; division by zero is error 11.
    pub fn checked_div(&self, rhs: &Self) -> VbaResult<Self> {
        if rhs.mag == 0 {
            return Err(VbaError::DIVISION_BY_ZERO);
        }
        let neg = self.neg != rhs.neg;
        let d = rhs.mag;
        let s = i32::from(self.scale) - i32::from(rhs.scale);
        let mut n = U256::from_u128(self.mag);
        for _ in s..0 {
            n = n.mul_small(10).ok_or(VbaError::OVERFLOW)?;
        }
        let (q256, mut r) = n.divrem_u128(d);
        let mut scale = s.max(0);
        let Some(mut q) = q256.to_mantissa() else {
            return Self::normalize(q256, scale, neg);
        };
        while r != 0 && scale < i32::from(MAX_SCALE) {
            let next = r * 10;
            let digit = next / d;
            let q2 = q * 10 + digit;
            if q2 >= LIMIT {
                break;
            }
            q = q2;
            r = next % d;
            scale += 1;
        }
        if r != 0 {
            let twice = r * 2;
            if twice > d || (twice == d && q % 2 == 1) {
                q += 1;
            }
        } else {
            while scale > 0 && q % 10 == 0 && q != 0 {
                q /= 10;
                scale -= 1;
            }
            if q == 0 {
                scale = 0;
            }
        }
        Self::normalize(U256::from_u128(q), scale, neg)
    }

    /// Remainder with the sign of the dividend (`Mod` on whole numbers is
    /// handled by the operator layer; this is the exact decimal remainder).
    pub fn checked_rem(&self, rhs: &Self) -> VbaResult<Self> {
        if rhs.mag == 0 {
            return Err(VbaError::DIVISION_BY_ZERO);
        }
        let (x, y, s) = Self::aligned(self, rhs);
        // |x| mod |y| via the long-division helper (y < 2^97 by construction
        // only when aligned values fit; otherwise reduce through repeated
        // subtraction is impractical, so report overflow).
        let ym = y.to_u128_wide().ok_or(VbaError::OVERFLOW)?;
        let (_, r) = x.divrem_u128(ym);
        Self::normalize(U256::from_u128(r), s, self.neg)
    }
}

impl U256 {
    /// Returns the value if it fits in 97 bits (the range `divrem_u128`
    /// supports).
    fn to_u128_wide(self) -> Option<u128> {
        if self.0[2] != 0 || self.0[3] != 0 || self.0[1] >> 33 != 0 {
            None
        } else {
            Some(u128::from(self.0[0]) | (u128::from(self.0[1]) << 64))
        }
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Self) -> bool {
        self.numeric_cmp(other) == Ordering::Equal
    }
}

impl Eq for Decimal {}

impl PartialOrd for Decimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Decimal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numeric_cmp(other)
    }
}

impl Default for Decimal {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for Decimal {
    /// Plain decimal text with exactly `scale` fractional digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut digits = self.mag.to_string();
        let scale = usize::from(self.scale);
        if scale > 0 {
            if digits.len() <= scale {
                digits = format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits);
            }
            digits.insert(digits.len() - scale, '.');
        }
        if self.neg && self.mag != 0 {
            write!(f, "-{digits}")
        } else {
            f.write_str(&digits)
        }
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decimal({self})")
    }
}

impl From<Currency> for Decimal {
    fn from(c: Currency) -> Self {
        Self::from_currency(c)
    }
}

/// Number of currency units per whole unit, re-exported for convenience.
pub const CURRENCY_UNITS: i64 = CURRENCY_SCALE;

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::parse(s).unwrap().unwrap()
    }

    fn show(r: VbaResult<Decimal>) -> String {
        match r {
            Ok(v) => v.to_string(),
            Err(e) => format!("E{}", e.number()),
        }
    }

    #[test]
    fn parse_and_display() {
        for (i, o) in [
            ("0", "0"),
            ("1.50", "1.50"),
            ("-0.001", "-0.001"),
            (
                "79228162514264337593543950335",
                "79228162514264337593543950335",
            ),
            ("1e3", "1000"),
            ("1.5e-3", "0.0015"),
            (".5", "0.5"),
            ("5.", "5"),
        ] {
            assert_eq!(d(i).to_string(), o, "{i}");
        }
        assert!(Decimal::parse("abc").is_none());
        assert!(Decimal::parse("").is_none());
        assert!(Decimal::parse("1e").is_none());
        assert!(Decimal::parse("1.2.3").is_none());
    }

    #[test]
    fn parse_rounds_beyond_28_places() {
        assert_eq!(
            d("0.12345678901234567890123456785").to_string(),
            "0.1234567890123456789012345678"
        );
        assert_eq!(
            d("0.12345678901234567890123456795").to_string(),
            "0.1234567890123456789012345680"
        );
        // 1e-29 rounds to zero at scale 28.
        let tiny = format!("0.{}1", "0".repeat(28));
        assert_eq!(d(&tiny).to_string(), format!("0.{}", "0".repeat(28)));
    }

    #[test]
    fn parse_overflow() {
        assert_eq!(
            Decimal::parse("79228162514264337593543950336").unwrap(),
            Err(VbaError::OVERFLOW)
        );
        assert_eq!(Decimal::parse("1e29").unwrap(), Err(VbaError::OVERFLOW));
        // 29 integer digits with a fraction rounds into range.
        assert_eq!(
            show(Decimal::parse("7922816251426433759354395033.55").unwrap()),
            "7922816251426433759354395034"
        );
    }

    #[test]
    fn add_sub_scales() {
        assert_eq!(show(d("1.1").checked_add(&d("2.205"))), "3.305");
        assert_eq!(show(d("1.5").checked_sub(&d("1.5"))), "0.0");
        assert_eq!(show(d("1").checked_sub(&d("2.5"))), "-1.5");
        assert_eq!(show(d("-1").checked_add(&d("-2"))), "-3");
        assert_eq!(show(Decimal::MAX.checked_add(&d("1"))), "E6");
        assert_eq!(show(Decimal::MIN.checked_sub(&d("1"))), "E6");
        // Overflowing sum with a fractional operand rounds instead of failing.
        assert_eq!(
            show(Decimal::MAX.checked_add(&d("0.4"))),
            "79228162514264337593543950335"
        );
        assert_eq!(show(Decimal::MAX.checked_add(&d("0.6"))), "E6");
    }

    #[test]
    fn mul() {
        assert_eq!(show(d("1.5").checked_mul(&d("2"))), "3.0");
        assert_eq!(show(d("0.1").checked_mul(&d("0.1"))), "0.01");
        assert_eq!(show(d("-2").checked_mul(&d("3"))), "-6");
        assert_eq!(show(Decimal::MAX.checked_mul(&d("2"))), "E6");
        assert_eq!(
            show(d("79228162514264337593543950335").checked_mul(&d("0.1"))),
            "7922816251426433759354395033.5"
        );
        assert_eq!(
            show(d("0.0000000000000000000000000001").checked_mul(&d("0.5"))),
            "0.0000000000000000000000000000"
        );
        assert_eq!(
            show(d("0.0000000000000000000000000003").checked_mul(&d("0.5"))),
            "0.0000000000000000000000000002"
        );
    }

    #[test]
    fn div() {
        assert_eq!(
            show(d("1").checked_div(&d("3"))),
            "0.3333333333333333333333333333"
        );
        assert_eq!(
            show(d("2").checked_div(&d("3"))),
            "0.6666666666666666666666666667"
        );
        assert_eq!(show(d("1").checked_div(&d("4"))), "0.25");
        assert_eq!(show(d("6").checked_div(&d("2"))), "3");
        assert_eq!(show(d("1.00").checked_div(&d("1"))), "1");
        assert_eq!(show(d("-10").checked_div(&d("4"))), "-2.5");
        assert_eq!(show(d("10").checked_div(&d("0"))), "E11");
        assert_eq!(show(d("0").checked_div(&d("0"))), "E11");
        assert_eq!(show(d("0").checked_div(&d("5"))), "0");
        assert_eq!(
            show(d("100").checked_div(&d("3"))),
            "33.333333333333333333333333333"
        );
        assert_eq!(show(Decimal::MAX.checked_div(&d("0.1"))), "E6");
        assert_eq!(
            show(Decimal::MAX.checked_div(&d("1.0000000000000000000000000000"))),
            "79228162514264337593543950335"
        );
    }

    #[test]
    fn rem() {
        assert_eq!(show(d("7.5").checked_rem(&d("2"))), "1.5");
        assert_eq!(show(d("-7.5").checked_rem(&d("2"))), "-1.5");
        assert_eq!(show(d("1").checked_rem(&d("0"))), "E11");
    }

    #[test]
    fn rounding() {
        assert_eq!(d("2.5").round(0).to_string(), "2");
        assert_eq!(d("3.5").round(0).to_string(), "4");
        assert_eq!(d("-2.5").round(0).to_string(), "-2");
        assert_eq!(d("1.005").round(2).to_string(), "1.00");
        assert_eq!(d("1.015").round(2).to_string(), "1.02");
        assert_eq!(d("-1.9").truncate().to_string(), "-1");
        assert_eq!(d("-1.1").floor().to_string(), "-2");
        assert_eq!(d("1.9").floor().to_string(), "1");
        assert_eq!(d("-3").floor().to_string(), "-3");
    }

    #[test]
    fn integer_and_currency_conversion() {
        assert_eq!(d("2.5").to_i64_round().unwrap(), 2);
        assert_eq!(d("-3.5").to_i64_round().unwrap(), -4);
        assert_eq!(
            d("9223372036854775808").to_i64_round(),
            Err(VbaError::OVERFLOW)
        );
        assert_eq!(d("-9223372036854775808").to_i64_round().unwrap(), i64::MIN);
        assert_eq!(d("1.23456").to_currency().unwrap().scaled(), 12_346);
        assert_eq!(d("922337203685478").to_currency(), Err(VbaError::OVERFLOW));
        assert_eq!(
            Decimal::from_currency(Currency::from_scaled(-15_000)).to_string(),
            "-1.5000"
        );
    }

    #[test]
    fn from_float() {
        assert_eq!(show(Decimal::from_f64(0.1)), "0.1");
        assert_eq!(show(Decimal::from_f64(0.1 + 0.2)), "0.3");
        assert_eq!(show(Decimal::from_f64(1.0 / 3.0)), "0.333333333333333");
        assert_eq!(show(Decimal::from_f64(-123456.789)), "-123456.789");
        assert_eq!(show(Decimal::from_f64(1e-30)), "0");
        assert_eq!(show(Decimal::from_f64(f64::NAN)), "E6");
        assert_eq!(show(Decimal::from_f64(1e29)), "E6");
        assert_eq!(show(Decimal::from_f64(7.922_816_251_426_434e28)), "E6");
        assert_eq!(show(Decimal::from_f32(0.1f32)), "0.1");
        assert_eq!(show(Decimal::from_f32(16777216.0)), "16777220");
    }

    #[test]
    fn value_equality_ignores_scale() {
        assert_eq!(d("1.0"), d("1.00"));
        assert!(d("-0.5") < d("0.25"));
        assert!(d("-2") < d("-1.5"));
        assert_eq!(d("0"), Decimal::from_parts(0, 0, true).unwrap());
    }

    #[test]
    fn to_f64() {
        assert_eq!(d("1.5").to_f64(), 1.5);
        assert_eq!(d("-0.25").to_f64(), -0.25);
    }

    #[test]
    fn parts_validation() {
        assert!(Decimal::from_parts(1u128 << 96, 0, false).is_err());
        assert!(Decimal::from_parts(1, 29, false).is_err());
        assert_eq!(
            Decimal::from_parts(15, 1, true).unwrap().to_string(),
            "-1.5"
        );
    }
}
