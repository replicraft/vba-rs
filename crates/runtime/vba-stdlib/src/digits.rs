//! Decimal digit strings of doubles, shared by `Round`, `Format` and `Str`.
//!
//! A double is first reduced to 15 significant decimal digits (the precision
//! VBA displays and `Format` works with), then rounded in decimal, so
//! `0.125` rounds half-away to `0.13` in `Format` and half-even to `0.12` in
//! `Round`, independent of binary representation noise.

/// Rounding rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rounding {
    /// Ties away from zero (`Format`).
    HalfUp,
    /// Ties to the even digit (`Round`).
    HalfEven,
}

/// A non-negative decimal: `0.d1 d2 ... dn * 10^point`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dec {
    /// Digits without trailing zeros (empty for zero).
    pub digits: Vec<u8>,
    /// Number of digits left of the decimal point (may be zero or negative).
    pub point: i32,
}

impl Dec {
    /// Digits of `|x|` at 15 significant digits. `x` must be finite.
    pub fn from_f64(x: f64) -> Dec {
        let s = format!("{:.14e}", x.abs());
        let (mant, exp) = s.split_once('e').expect("exponent");
        let exp: i32 = exp.parse().expect("exponent value");
        let mut digits: Vec<u8> = mant
            .bytes()
            .filter(u8::is_ascii_digit)
            .map(|b| b - b'0')
            .collect();
        while digits.last() == Some(&0) {
            digits.pop();
        }
        if digits.is_empty() {
            return Dec { digits, point: 0 };
        }
        Dec {
            digits,
            point: exp + 1,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }

    /// Rounds to `places` digits after the decimal point.
    pub fn round(&self, places: i32, rule: Rounding) -> Dec {
        let keep = i64::from(self.point) + i64::from(places);
        if keep >= self.digits.len() as i64 {
            return self.clone();
        }
        if keep < 0 {
            return Dec {
                digits: Vec::new(),
                point: 0,
            };
        }
        let keep = keep as usize;
        let next = self.digits[keep];
        let rest_nonzero = self.digits[keep + 1..].iter().any(|&d| d != 0);
        let prev_odd = keep > 0 && self.digits[keep - 1] % 2 == 1;
        let up = match rule {
            Rounding::HalfUp => next >= 5,
            Rounding::HalfEven => next > 5 || (next == 5 && (rest_nonzero || prev_odd)),
        };
        let mut digits = self.digits[..keep].to_vec();
        let mut point = self.point;
        if up {
            let mut i = digits.len();
            loop {
                if i == 0 {
                    digits.insert(0, 1);
                    point += 1;
                    break;
                }
                i -= 1;
                if digits[i] == 9 {
                    digits[i] = 0;
                } else {
                    digits[i] += 1;
                    break;
                }
            }
        }
        while digits.last() == Some(&0) {
            digits.pop();
        }
        if digits.is_empty() {
            point = 0;
        }
        Dec { digits, point }
    }

    /// The digit at decimal place `pos`, where `pos = 0` is the units digit,
    /// positive positions go left and negative positions right.
    pub fn digit_at(&self, pos: i32) -> u8 {
        let idx = i64::from(self.point) - 1 - i64::from(pos);
        if idx < 0 || idx >= self.digits.len() as i64 {
            0
        } else {
            self.digits[idx as usize]
        }
    }

    /// Decimal exponent of the first significant digit (`floor(log10)`);
    /// zero for zero.
    pub fn exp10(&self) -> i32 {
        if self.is_zero() { 0 } else { self.point - 1 }
    }

    pub fn to_f64(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let ds: String = self.digits.iter().map(|d| char::from(b'0' + d)).collect();
        format!("0.{ds}e{}", self.point).parse().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_rules() {
        let r = |x: f64, p, rule| Dec::from_f64(x).round(p, rule).to_f64();
        assert_eq!(r(0.125, 2, Rounding::HalfUp), 0.13);
        assert_eq!(r(0.125, 2, Rounding::HalfEven), 0.12);
        assert_eq!(r(2.5, 0, Rounding::HalfEven), 2.0);
        assert_eq!(r(3.5, 0, Rounding::HalfEven), 4.0);
        assert_eq!(r(0.5, 0, Rounding::HalfEven), 0.0);
        assert_eq!(r(0.5, 0, Rounding::HalfUp), 1.0);
        assert_eq!(r(99.995, 2, Rounding::HalfUp), 100.0);
        assert_eq!(r(1234.0, -2, Rounding::HalfUp), 1200.0);
    }
}
