//! Property tests for invariants of the value system.

use proptest::prelude::*;
use std::cmp::Ordering;
use vba_value::convert::*;
use vba_value::ops::{self, ArithOp, LogicOp, Overflow};
use vba_value::{CompareMode, Currency, Date, Decimal, Variant, VbaError, VbaString};

fn decimal_strategy() -> impl Strategy<Value = Decimal> {
    (0u128..(1u128 << 96), 0u8..=28, any::<bool>())
        .prop_map(|(m, s, n)| Decimal::from_parts(m, s, n).unwrap())
}

proptest! {
    #[test]
    fn utf16_units_survive_concat(a in proptest::collection::vec(any::<u16>(), 0..40),
                                  b in proptest::collection::vec(any::<u16>(), 0..40)) {
        let (sa, sb) = (VbaString::from_units(a.clone()), VbaString::from_units(b.clone()));
        let c = sa.concat(&sb);
        let mut want = a.clone();
        want.extend_from_slice(&b);
        prop_assert_eq!(c.as_units(), want.as_slice());
        prop_assert_eq!(c.len(), a.len() + b.len());
        // Binary comparison is code-unit order.
        prop_assert_eq!(sa.compare(&sb, CompareMode::Binary), a.cmp(&b));
        // Text comparison is reflexive and symmetric-consistent.
        prop_assert_eq!(sa.compare(&sa, CompareMode::Text), Ordering::Equal);
        prop_assert_eq!(
            sa.compare(&sb, CompareMode::Text),
            sb.compare(&sa, CompareMode::Text).reverse()
        );
        // Converting through CStr keeps the exact units.
        let v = Variant::String(sa.clone());
        prop_assert_eq!(to_vba_string(&v).unwrap(), sa);
    }

    #[test]
    fn valid_text_roundtrips(s in "\\PC{0,30}") {
        let v = VbaString::from(s.as_str());
        prop_assert_eq!(v.to_string_strict().unwrap(), s.clone());
        prop_assert_eq!(v.to_string_lossy(), s);
    }

    #[test]
    fn currency_text_roundtrip(c in any::<i64>()) {
        let cur = Currency::from_scaled(c);
        let dec = Decimal::parse(&cur.to_string()).unwrap().unwrap();
        prop_assert_eq!(dec.to_currency().unwrap(), cur);
        prop_assert_eq!(to_currency(&Variant::from(cur.to_string().as_str())).unwrap(), cur);
        prop_assert_eq!(Decimal::from_currency(cur).to_currency().unwrap(), cur);
    }

    #[test]
    fn currency_add_sub_inverse(a in any::<i64>(), b in any::<i64>()) {
        let (a, b) = (Currency::from_scaled(a), Currency::from_scaled(b));
        if let Ok(s) = a.checked_add(b) {
            prop_assert_eq!(s.checked_sub(b).unwrap(), a);
        }
    }

    #[test]
    fn currency_f64_roundtrip_in_safe_range(c in -(1i64 << 50)..(1i64 << 50)) {
        let cur = Currency::from_scaled(c);
        prop_assert_eq!(Currency::from_f64(cur.to_f64()).unwrap(), cur);
    }

    #[test]
    fn decimal_text_roundtrip(d in decimal_strategy()) {
        let back = Decimal::parse(&d.to_string()).unwrap().unwrap();
        prop_assert_eq!(back, d);
        prop_assert_eq!(back.scale(), d.scale());
    }

    #[test]
    fn decimal_add_sub_inverse_small(a in any::<i64>(), b in any::<i64>(), sa in 0u8..=10, sb in 0u8..=10) {
        let x = Decimal::from_parts(u128::from(a.unsigned_abs()), sa, a < 0).unwrap();
        let y = Decimal::from_parts(u128::from(b.unsigned_abs()), sb, b < 0).unwrap();
        let sum = x.checked_add(&y).unwrap();
        prop_assert_eq!(sum.checked_sub(&y).unwrap(), x);
        prop_assert_eq!(x.checked_add(&y).unwrap(), y.checked_add(&x).unwrap());
    }

    #[test]
    fn decimal_mul_commutes_and_matches_integers(a in any::<i32>(), b in any::<i32>()) {
        let (x, y) = (Decimal::from_i64(i64::from(a)), Decimal::from_i64(i64::from(b)));
        let p = x.checked_mul(&y).unwrap();
        prop_assert_eq!(p, y.checked_mul(&x).unwrap());
        prop_assert_eq!(p.to_i64_round().unwrap(), i64::from(a) * i64::from(b));
    }

    #[test]
    fn decimal_order_matches_integers(a in any::<i64>(), b in any::<i64>()) {
        prop_assert_eq!(Decimal::from_i64(a).cmp(&Decimal::from_i64(b)), a.cmp(&b));
    }

    #[test]
    fn decimal_div_then_mul_close(a in 1i64..1_000_000, b in 1i64..1_000_000) {
        let q = Decimal::from_i64(a).checked_div(&Decimal::from_i64(b)).unwrap();
        let back = q.checked_mul(&Decimal::from_i64(b)).unwrap();
        let err = back.checked_sub(&Decimal::from_i64(a)).unwrap().abs();
        let tol = Decimal::parse("0.0000000001").unwrap().unwrap();
        prop_assert!(err < tol);
    }

    #[test]
    fn integer_string_roundtrips(i in any::<i16>(), l in any::<i32>(), ll in any::<i64>()) {
        let s = |v: &Variant| Variant::String(to_vba_string(v).unwrap());
        prop_assert_eq!(to_integer(&s(&Variant::Integer(i))).unwrap(), i);
        prop_assert_eq!(to_long(&s(&Variant::Long(l))).unwrap(), l);
        prop_assert_eq!(to_longlong(&s(&Variant::LongLong(ll))).unwrap(), ll);
    }

    #[test]
    fn cint_is_bankers_rounding(x in -40000.0f64..40000.0) {
        let r = x.round_ties_even();
        match to_integer(&Variant::Double(x)) {
            Ok(v) => prop_assert_eq!(f64::from(v), r),
            Err(e) => {
                prop_assert_eq!(e, VbaError::OVERFLOW);
                prop_assert!(!(-32768.0..=32767.0).contains(&r));
            }
        }
    }

    #[test]
    fn nonfinite_always_overflows(neg in any::<bool>()) {
        let v = Variant::Double(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
        prop_assert_eq!(to_long(&v), Err(VbaError::OVERFLOW));
        prop_assert_eq!(to_single(&v), Err(VbaError::OVERFLOW));
        prop_assert_eq!(to_currency(&v), Err(VbaError::OVERFLOW));
        prop_assert_eq!(to_decimal(&v), Err(VbaError::OVERFLOW));
        prop_assert_eq!(to_date(&v), Err(VbaError::OVERFLOW));
    }

    #[test]
    fn double_text_roundtrip_keeps_15_digits(x in -1e14f64..1e14) {
        let text = to_vba_string(&Variant::Double(x)).unwrap();
        let back = to_double(&Variant::String(text)).unwrap();
        prop_assert!((back - x).abs() <= x.abs() * 1e-14 + 1e-300);
    }

    #[test]
    fn variant_long_addition_is_exact(a in any::<i32>(), b in any::<i32>()) {
        let r = ops::arith(ArithOp::Add, &Variant::Long(a), &Variant::Long(b), Overflow::Promote).unwrap();
        let exact = i64::from(a) + i64::from(b);
        match r {
            Variant::Long(v) => prop_assert_eq!(i64::from(v), exact),
            Variant::Double(v) => {
                prop_assert_eq!(v, exact as f64);
                prop_assert!(i32::try_from(exact).is_err());
            }
            other => prop_assert!(false, "unexpected {other:?}"),
        }
        let typed = ops::arith(ArithOp::Add, &Variant::Long(a), &Variant::Long(b), Overflow::Error);
        prop_assert_eq!(typed.is_ok(), i32::try_from(exact).is_ok());
    }

    #[test]
    fn not_is_involutive_and_de_morgan(a in any::<i32>(), b in any::<i32>()) {
        let (x, y) = (Variant::Long(a), Variant::Long(b));
        prop_assert_eq!(ops::not(&ops::not(&x).unwrap()).unwrap(), x.clone());
        let lhs = ops::not(&ops::logic(LogicOp::And, &x, &y).unwrap()).unwrap();
        let rhs = ops::logic(LogicOp::Or, &ops::not(&x).unwrap(), &ops::not(&y).unwrap()).unwrap();
        prop_assert_eq!(lhs, rhs);
        prop_assert_eq!(
            ops::logic(LogicOp::Xor, &x, &y).unwrap(),
            Variant::Long(a ^ b)
        );
    }

    #[test]
    fn mod_and_intdiv_identity(a in any::<i32>(), b in any::<i32>().prop_filter("nonzero", |b| *b != 0)) {
        let (x, y) = (Variant::Long(a), Variant::Long(b));
        let q = ops::arith(ArithOp::IntDiv, &x, &y, Overflow::Error);
        let r = ops::arith(ArithOp::Mod, &x, &y, Overflow::Error).unwrap();
        prop_assert_eq!(&r, &Variant::Long(a.wrapping_rem(b)));
        if let Ok(Variant::Long(q)) = q {
            prop_assert_eq!(i64::from(q) * i64::from(b) + i64::from(a.wrapping_rem(b)), i64::from(a));
        }
    }

    #[test]
    fn date_calendar_roundtrip(days in -657_434i64..2_958_466, secs in 0u32..86_400) {
        let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
        let p = Date::from_serial(days as f64).unwrap().parts();
        let d = Date::from_ymd_hms(p.year, p.month, p.day, h, m, s).unwrap();
        let q = d.parts();
        prop_assert_eq!((q.year, q.month, q.day), (p.year, p.month, p.day));
        prop_assert_eq!((q.hour, q.minute, q.second), (h, m, s));
        // The integer part of the serial is the original day number.
        prop_assert_eq!(d.serial().trunc(), days as f64);
    }

    #[test]
    fn comparison_is_antisymmetric(a in any::<i64>(), b in any::<f64>()) {
        use ops::{CompareOp::*, CompareOptions};
        let (x, y) = (Variant::LongLong(a), Variant::Double(b));
        let o = CompareOptions::default();
        let lt = ops::compare(Lt, &x, &y, o).unwrap();
        let gt = ops::compare(Gt, &y, &x, o).unwrap();
        prop_assert_eq!(lt, gt);
    }
}
