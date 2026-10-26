//! Differential tests against independent libraries (`jiff` for the
//! calendar, `rust_decimal` for Decimal arithmetic). The libraries are
//! dev-dependencies only; shipped code does not use them.
//!
//! Where VBA differs from the oracle the difference is documented next to the
//! test and not asserted: `rust_decimal` returns `None` where VBA raises
//! error 6, and the two libraries may round excess digits differently, so the
//! full-range test compares only results that are exactly representable.

use jiff::Unit;
use jiff::civil::Date as CivilDate;
use proptest::prelude::*;
use std::cmp::Ordering;
use vba_value::date::{days_in_month, is_leap_year};
use vba_value::{Date, Decimal};

fn epoch() -> CivilDate {
    // OLE Automation date 0 is 1899-12-30.
    jiff::civil::date(1899, 12, 30)
}

fn serial_of(d: CivilDate) -> i64 {
    i64::from(d.since((Unit::Day, epoch())).unwrap().get_days())
}

/// Every day from 0100-01-01 to 9999-12-31: calendar fields to serial and
/// back, weekday agree with jiff.
#[test]
fn calendar_matches_jiff_for_every_day() {
    let last = jiff::civil::date(9999, 12, 31);
    let mut d = jiff::civil::date(100, 1, 1);
    let mut count = 0u32;
    loop {
        let want = serial_of(d);
        let (y, m, day) = (
            i64::from(d.year()),
            i64::from(d.month()),
            i64::from(d.day()),
        );
        let ours = Date::date_serial(y, m, day).unwrap();
        assert_eq!(ours.serial(), want as f64, "{d}");
        let p = Date::from_serial(want as f64).unwrap().parts();
        assert_eq!(
            (i64::from(p.year), i64::from(p.month), i64::from(p.day)),
            (y, m, day),
            "{d}"
        );
        assert_eq!((p.hour, p.minute, p.second), (0, 0, 0));
        // vbSunday = 1 .. vbSaturday = 7
        assert_eq!(
            ours.weekday(),
            d.weekday().to_sunday_one_offset() as u32,
            "{d}"
        );
        count += 1;
        if d == last {
            break;
        }
        d = d.tomorrow().unwrap();
    }
    // Serials -657434 ..= 2958465 inclusive.
    assert_eq!(count, 657_434 + 2_958_465 + 1);
}

#[test]
fn leap_years_and_month_lengths_match_jiff() {
    for y in 100..=9999i16 {
        let jan = jiff::civil::date(y, 1, 1);
        assert_eq!(is_leap_year(i64::from(y)), jan.in_leap_year(), "{y}");
        for m in 1..=12i8 {
            let d = jiff::civil::date(y, m, 1);
            assert_eq!(
                days_in_month(i64::from(y), m as u32),
                d.days_in_month() as u32,
                "{y}-{m}"
            );
        }
    }
    assert!(!is_leap_year(1900) && is_leap_year(2000));
}

proptest! {
    /// Day-of-month roll-over in `DateSerial` agrees with jiff's span
    /// arithmetic from the first of the month.
    #[test]
    fn date_serial_day_rollover(y in 100i16..=9998, m in 1i8..=12, extra in 0i64..400) {
        let first = jiff::civil::date(y, m, 1);
        let want = first.checked_add(jiff::Span::new().days(extra)).unwrap();
        let ours = Date::date_serial(i64::from(y), i64::from(m), 1 + extra).unwrap();
        prop_assert_eq!(ours.serial(), serial_of(want) as f64);
    }

    /// Month roll-over (month beyond 12) lands on the same day as jiff.
    #[test]
    fn date_serial_month_rollover(y in 100i16..=9990, extra in 0i64..60, d in 1i8..=28) {
        let want = jiff::civil::date(y, 1, d)
            .checked_add(jiff::Span::new().months(extra))
            .unwrap();
        let ours = Date::date_serial(i64::from(y), 1 + extra, i64::from(d)).unwrap();
        prop_assert_eq!(ours.serial(), serial_of(want) as f64);
    }

    /// Time of day for non-negative serials. (Negative serials use the OLE
    /// quirk that the fraction is `abs(frac)`; jiff has no such concept, so
    /// they are not compared here.)
    #[test]
    fn time_of_day_matches_jiff(days in 0i64..2_958_465, secs in 0u32..86_400) {
        let serial = days as f64 + f64::from(secs) / 86_400.0;
        let p = Date::from_serial(serial).unwrap().parts();
        prop_assert_eq!((p.hour, p.minute, p.second), (secs / 3600, secs / 60 % 60, secs % 60));
        let want = epoch().checked_add(jiff::Span::new().days(days)).unwrap();
        prop_assert_eq!(
            (p.year, p.month as i8, p.day as i8),
            (i32::from(want.year()), want.month(), want.day())
        );
    }
}

// ---------------------------------------------------------------------------
// Decimal

fn to_rd(d: &Decimal) -> rust_decimal::Decimal {
    let m = d.mantissa();
    rust_decimal::Decimal::from_parts(
        m as u32,
        (m >> 32) as u32,
        (m >> 64) as u32,
        d.is_negative(),
        u32::from(d.scale()),
    )
}

fn from_rd(d: rust_decimal::Decimal) -> Decimal {
    Decimal::from_parts(
        d.mantissa().unsigned_abs(),
        d.scale() as u8,
        d.is_sign_negative(),
    )
    .unwrap()
}

fn wide() -> impl Strategy<Value = Decimal> {
    (0u128..(1u128 << 96), 0u8..=28, any::<bool>())
        .prop_map(|(m, s, n)| Decimal::from_parts(m, s, n).unwrap())
}

/// Operands whose exact sum/product fits in 96 bits and scale 28.
fn narrow() -> impl Strategy<Value = Decimal> {
    (0u128..(1u128 << 40), 0u8..=12, any::<bool>())
        .prop_map(|(m, s, n)| Decimal::from_parts(m, s, n).unwrap())
}

proptest! {
    /// In the exact range both libraries produce the same value and the same
    /// scale (add/sub: larger scale; mul: sum of scales).
    #[test]
    fn exact_range_add_sub_mul(a in narrow(), b in narrow()) {
        let (ra, rb) = (to_rd(&a), to_rd(&b));
        let sum = a.checked_add(&b).unwrap();
        let rsum = ra.checked_add(rb).unwrap();
        prop_assert_eq!(sum.numeric_cmp(&from_rd(rsum)), Ordering::Equal);
        prop_assert_eq!(u32::from(sum.scale()), rsum.scale());
        let diff = a.checked_sub(&b).unwrap();
        let rdiff = ra.checked_sub(rb).unwrap();
        prop_assert_eq!(diff.numeric_cmp(&from_rd(rdiff)), Ordering::Equal);
        prop_assert_eq!(u32::from(diff.scale()), rdiff.scale());
        let prod = a.checked_mul(&b).unwrap();
        let rprod = ra.checked_mul(rb).unwrap();
        prop_assert_eq!(prod.numeric_cmp(&from_rd(rprod)), Ordering::Equal);
        prop_assert_eq!(u32::from(prod.scale()), rprod.scale());
    }

    /// Over the full range, results are compared only when exactly
    /// representable on our side (no digits dropped).
    #[test]
    fn full_range_agrees_when_exact(a in wide(), b in wide()) {
        let (ra, rb) = (to_rd(&a), to_rd(&b));
        if let (Ok(x), Some(rx)) = (a.checked_add(&b), ra.checked_add(rb))
            && x.scale() == a.scale().max(b.scale())
            && u32::from(x.scale()) == rx.scale()
        {
            prop_assert_eq!(x.numeric_cmp(&from_rd(rx)), Ordering::Equal);
        }
        if let (Ok(x), Some(rx)) = (a.checked_mul(&b), ra.checked_mul(rb))
            && u32::from(x.scale()) == u32::from(a.scale()) + u32::from(b.scale())
        {
            prop_assert_eq!(x.numeric_cmp(&from_rd(rx)), Ordering::Equal);
        }
    }

    /// Ordering by value agrees.
    #[test]
    fn comparison_agrees(a in wide(), b in wide()) {
        prop_assert_eq!(a.numeric_cmp(&b), to_rd(&a).cmp(&to_rd(&b)));
    }
}

#[test]
fn range_limits_agree() {
    assert_eq!(to_rd(&Decimal::MAX), rust_decimal::Decimal::MAX);
    assert_eq!(to_rd(&Decimal::MIN), rust_decimal::Decimal::MIN);
    // Overflow: rust_decimal gives None, VBA error 6.
    assert!(
        rust_decimal::Decimal::MAX
            .checked_add(rust_decimal::Decimal::ONE)
            .is_none()
    );
    assert!(Decimal::MAX.checked_add(&Decimal::from_i64(1)).is_err());
}
