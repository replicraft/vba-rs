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
    ///
    /// Values from 1e-5 up to 1e15 take an exact integer path; everything
    /// else goes through [`Dec::from_f64_text`].
    pub fn from_f64(x: f64) -> Dec {
        match fast_digits(x.abs()) {
            Some(d) => d,
            None => Dec::from_f64_text(x),
        }
    }

    /// The reference conversion: the exactly rounded 15-digit scientific
    /// text of the value, split into digits and exponent.
    pub(crate) fn from_f64_text(x: f64) -> Dec {
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
        if let Some(v) = self.fast_f64() {
            return v;
        }
        self.to_f64_text()
    }

    /// Exact when the digits fit a double's mantissa and the power of ten is
    /// itself exact: one correctly rounded multiplication or division.
    fn fast_f64(&self) -> Option<f64> {
        const POW10: [f64; 23] = [
            1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15,
            1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
        ];
        if self.digits.len() > 15 {
            return None;
        }
        let m = self.digits.iter().fold(0u64, |a, &d| a * 10 + u64::from(d));
        let p = i64::from(self.point) - self.digits.len() as i64;
        let m = m as f64;
        if (0..=22).contains(&p) {
            Some(m * POW10[p as usize])
        } else if (-22..0).contains(&p) {
            Some(m / POW10[(-p) as usize])
        } else {
            None
        }
    }

    pub(crate) fn to_f64_text(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let ds: String = self.digits.iter().map(|d| char::from(b'0' + d)).collect();
        format!("0.{ds}e{}", self.point).parse().unwrap_or(0.0)
    }
}

/// The 15-significant-digit decimal of a finite non-negative `x` in
/// `[1e-5, 1e15)`, by exact integer arithmetic: `x = m * 2^-s` and the
/// result is `m * 10^k / 2^s` rounded to nearest, ties to even, which is the
/// exact rounding the text conversion performs.
fn fast_digits(x: f64) -> Option<Dec> {
    if x == 0.0 {
        return Some(Dec {
            digits: Vec::new(),
            point: 0,
        });
    }
    if !(1e-5..1e15).contains(&x) {
        return None;
    }
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let m = u128::from((bits & ((1u64 << 52) - 1)) | (1u64 << 52));
    let s = 1075 - biased; // x = m * 2^-s, 17 <= s <= 69 in range
    if !(1..128).contains(&s) {
        return None;
    }
    const LO: u128 = 100_000_000_000_000;
    const HI: u128 = 1_000_000_000_000_000;
    let mut e10 = x.log10().floor() as i32;
    // `m * 10^k / 2^s` rounded to nearest, ties to even.
    let scaled = |k: i32| -> Option<u128> {
        if !(0..=19).contains(&k) {
            return None;
        }
        let num = m * 10u128.pow(k as u32);
        let mut q = num >> s;
        let rem = num & ((1u128 << s) - 1);
        let half = 1u128 << (s - 1);
        if rem > half || (rem == half && q & 1 == 1) {
            q += 1;
        }
        Some(q)
    };
    for _ in 0..4 {
        let mut q = scaled(14 - e10)?;
        if q >= HI {
            e10 += 1;
            continue;
        }
        if q < LO {
            e10 -= 1;
            continue;
        }
        // A guess one too high can round up to exactly 10^14; one more digit
        // then fits unless the value really was that large.
        if q == LO
            && let Some(q2) = scaled(15 - e10)
            && q2 < HI
        {
            q = q2;
            e10 -= 1;
        }
        let mut digits = [0u8; 15];
        let mut v = q as u64;
        for d in digits.iter_mut().rev() {
            *d = (v % 10) as u8;
            v /= 10;
        }
        let n = digits.iter().rposition(|&d| d != 0).map_or(0, |p| p + 1);
        return Some(Dec {
            digits: digits[..n].to_vec(),
            point: e10 + 1,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the fast conversions against the text-based reference.
    fn same(x: f64) {
        if !x.is_finite() {
            return;
        }
        let fast = Dec::from_f64(x);
        let slow = Dec::from_f64_text(x);
        assert_eq!(fast, slow, "digits of {x:e}");
        assert_eq!(
            fast.to_f64().to_bits(),
            fast.to_f64_text().to_bits(),
            "to_f64 of {x:e}"
        );
        for places in [-3, -1, 0, 1, 2, 3, 5, 9] {
            for rule in [Rounding::HalfUp, Rounding::HalfEven] {
                let (a, b) = (fast.round(places, rule), slow.round(places, rule));
                assert_eq!(a, b);
                assert_eq!(a.to_f64().to_bits(), b.to_f64_text().to_bits());
            }
        }
    }

    #[test]
    fn fast_path_matches_text_reference() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        // Special and boundary values, with their neighbours.
        let mut edge = vec![0.0, 1.0, 0.5, 0.125, 0.1, 0.3, 1.0 / 3.0, 2.0 / 3.0];
        for e in -8..=17 {
            edge.push(10f64.powi(e));
        }
        edge.extend([
            1234567890123.125, // a tie at the 15th digit
            123456789012.3125,
            99999999999999.95,
            999999999999999.4,
            999999999999999.5,
            9.999999999999995,
            0.999999999999999_5,
            0.000009999999999999_9,
            f64::MIN_POSITIVE,
            f64::MAX,
            1e-5,
            1e15,
            4503599627370496.5,
        ]);
        for &x in &edge {
            let mut v = x;
            for _ in 0..4 {
                same(v);
                same(-v);
                v = f64::from_bits(v.to_bits().wrapping_add(1));
            }
            let mut v = x;
            for _ in 0..4 {
                v = f64::from_bits(v.to_bits().wrapping_sub(1));
                same(v);
            }
        }
        // Random bit patterns restricted to the fast range and beyond.
        for _ in 0..30_000 {
            let r = next();
            let x = f64::from_bits(r);
            if x.is_finite() {
                same(x);
            }
            // Same exponent range as the fast path.
            let e = 1000 + (next() % 70) as u64; // about 2^-23 .. 2^46
            same(f64::from_bits((e << 52) | (r & ((1 << 52) - 1))));
        }
        // Short decimals such as the programs write: n / 10^d.
        for _ in 0..30_000 {
            let n = next() % 100_000_000_000;
            let d = (next() % 12) as i32;
            same(n as f64 / 10f64.powi(d));
            same(n as f64 * 10f64.powi(d % 4));
            same(n as f64 / 7.0);
            same(n as f64 * 1.37);
        }
        // Ties: odd multiples of 2^-k that have exactly 16 significant digits.
        for k in 1..=20 {
            let step = 2f64.powi(-k);
            for j in 0..200u64 {
                same((2 * j + 1) as f64 * step);
                same(((2 * j + 1) * 1_000_003) as f64 * step);
                same(((2 * j + 1) * 100_000_007_u64) as f64 * step);
            }
        }
        for i in 0..20_000u64 {
            same(i as f64);
            same(i as f64 + 0.5);
            same(i as f64 * 0.01);
        }
    }

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
