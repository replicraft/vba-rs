//! Algebraic invariants of the numeric tower: conversion round trips,
//! overflow boundaries of every integer type, Currency and Decimal scaling,
//! Date range edges, and the Variant result-type tables of every operator over
//! every pair of operand types, written down independently of the
//! implementation (MS-VBAL 5.6.9 with the documented VBA ranks).

use proptest::prelude::*;
use vba_value::convert::{
    convert_to, to_byte, to_currency, to_date, to_decimal, to_double, to_integer, to_long,
    to_longlong, to_vba_string,
};
use vba_value::ops::{self, ArithOp, CompareOp, CompareOptions, LogicOp, Overflow};
use vba_value::{Currency, Date, Decimal, VarType, Variant, VbaError};

// ------------------------------------------------------------ operand types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Empty,
    Boolean,
    Byte,
    Integer,
    Long,
    LongLong,
    Single,
    Double,
    Currency,
    Date,
    Decimal,
    Str,
}

const TYPES: [Ty; 12] = [
    Ty::Empty,
    Ty::Boolean,
    Ty::Byte,
    Ty::Integer,
    Ty::Long,
    Ty::LongLong,
    Ty::Single,
    Ty::Double,
    Ty::Currency,
    Ty::Date,
    Ty::Decimal,
    Ty::Str,
];

impl Ty {
    /// A sample of this type holding `n` (Boolean is always True, Empty has
    /// no value).
    fn sample(self, n: i32) -> Variant {
        match self {
            Ty::Empty => Variant::Empty,
            Ty::Boolean => Variant::Boolean(true),
            Ty::Byte => Variant::Byte(n as u8),
            Ty::Integer => Variant::Integer(n as i16),
            Ty::Long => Variant::Long(n),
            Ty::LongLong => Variant::LongLong(i64::from(n)),
            Ty::Single => Variant::Single(n as f32),
            Ty::Double => Variant::Double(f64::from(n)),
            Ty::Currency => Variant::Currency(Currency::from_i64(i64::from(n)).unwrap()),
            Ty::Date => Variant::Date(Date::from_serial(f64::from(n)).unwrap()),
            Ty::Decimal => Variant::Decimal(Decimal::from_i64(i64::from(n))),
            Ty::Str => Variant::from(n.to_string().as_str()),
        }
    }

    /// The width class of an integer-like type: Byte 0, Integer 1 (also
    /// Boolean and Empty), Long 2, LongLong 3.
    fn int_rank(self) -> Option<u8> {
        match self {
            Ty::Byte => Some(0),
            Ty::Empty | Ty::Boolean | Ty::Integer => Some(1),
            Ty::Long => Some(2),
            Ty::LongLong => Some(3),
            _ => None,
        }
    }
}

fn int_name(rank: u8) -> &'static str {
    ["Byte", "Integer", "Long", "LongLong"][usize::from(rank)]
}

fn type_name(v: &Variant) -> &'static str {
    match v {
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
    }
}

// ------------------------------------------------------- the reference tables

/// Result type of `+ - *` on two non-Null Variants (or the error number).
fn expected_add_sub_mul(op: ArithOp, a: Ty, b: Ty) -> Result<&'static str, i32> {
    let either = |t: Ty| a == t || b == t;
    if op == ArithOp::Add
        && (matches!((a, b), (Ty::Str, Ty::Str))
            || matches!((a, b), (Ty::Str, Ty::Empty) | (Ty::Empty, Ty::Str)))
    {
        return Ok("String");
    }
    Ok(if either(Ty::Decimal) {
        "Decimal"
    } else if either(Ty::Currency) {
        "Currency"
    } else if either(Ty::Date) {
        match op {
            ArithOp::Add => "Date",
            ArithOp::Sub if a == Ty::Date && b != Ty::Date => "Date",
            ArithOp::Sub => "Double",
            _ => "Double",
        }
    } else if either(Ty::Double) || either(Ty::Str) {
        "Double"
    } else if either(Ty::Single) {
        if either(Ty::LongLong) {
            "Double"
        } else {
            "Single"
        }
    } else {
        int_name(a.int_rank().unwrap().max(b.int_rank().unwrap()))
    })
}

fn expected_div(a: Ty, b: Ty) -> Result<&'static str, i32> {
    let either = |t: Ty| a == t || b == t;
    if b == Ty::Empty {
        // 0 / 0 is Overflow; anything else over Empty is Division by zero.
        return Err(if a == Ty::Empty { 6 } else { 11 });
    }
    Ok(if either(Ty::Decimal) {
        "Decimal"
    } else if either(Ty::Currency) {
        "Currency"
    } else if a == Ty::Single && b == Ty::Single {
        "Single"
    } else {
        "Double"
    })
}

fn expected_int_div_mod(a: Ty, b: Ty) -> Result<&'static str, i32> {
    if b == Ty::Empty {
        return Err(11);
    }
    // Non-integer operands round to Long.
    let rank = |t: Ty| t.int_rank().unwrap_or(2);
    Ok(int_name(rank(a).max(rank(b))))
}

fn expected_arith(op: ArithOp, a: Ty, b: Ty) -> Result<&'static str, i32> {
    match op {
        ArithOp::Add | ArithOp::Sub | ArithOp::Mul => expected_add_sub_mul(op, a, b),
        ArithOp::Div => expected_div(a, b),
        ArithOp::IntDiv | ArithOp::Mod => expected_int_div_mod(a, b),
        // 0 ^ negative is Division by zero (VBA behaviour kept over MS-VBAL);
        // only Empty ^ True has a zero base and a negative exponent here.
        ArithOp::Pow if a == Ty::Empty && b == Ty::Boolean => Err(11),
        ArithOp::Pow => Ok("Double"),
    }
}

fn expected_logic(a: Ty, b: Ty) -> &'static str {
    if a == Ty::Boolean && b == Ty::Boolean {
        return "Boolean";
    }
    // Non-integer operands convert to Long.
    let rank = |t: Ty| t.int_rank().unwrap_or(2);
    int_name(rank(a).max(rank(b)))
}

const ARITH: [ArithOp; 7] = [
    ArithOp::Add,
    ArithOp::Sub,
    ArithOp::Mul,
    ArithOp::Div,
    ArithOp::IntDiv,
    ArithOp::Mod,
    ArithOp::Pow,
];
const LOGIC: [LogicOp; 5] = [
    LogicOp::And,
    LogicOp::Or,
    LogicOp::Xor,
    LogicOp::Eqv,
    LogicOp::Imp,
];
const COMPARE: [CompareOp; 6] = [
    CompareOp::Eq,
    CompareOp::Ne,
    CompareOp::Lt,
    CompareOp::Gt,
    CompareOp::Le,
    CompareOp::Ge,
];

#[test]
fn arithmetic_result_types_match_the_tables_for_every_pair() {
    for op in ARITH {
        for a in TYPES {
            for b in TYPES {
                let (l, r) = (a.sample(7), b.sample(2));
                let got = ops::arith(op, &l, &r, Overflow::Promote);
                match expected_arith(op, a, b) {
                    Ok(t) => {
                        let v = got.unwrap_or_else(|e| {
                            panic!("{op:?} {a:?} {b:?}: expected {t}, got error {}", e.number())
                        });
                        assert_eq!(type_name(&v), t, "{op:?} {a:?} {b:?}");
                    }
                    Err(n) => {
                        let e = got.expect_err(&format!("{op:?} {a:?} {b:?} must fail"));
                        assert_eq!(e.number(), n, "{op:?} {a:?} {b:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn null_propagates_through_every_arithmetic_operator() {
    for op in ARITH {
        for t in TYPES {
            for (l, r) in [
                (Variant::Null, t.sample(2)),
                (t.sample(7), Variant::Null),
                (Variant::Null, Variant::Null),
            ] {
                // A String operand of `+` against Null is still Null.
                assert_eq!(
                    ops::arith(op, &l, &r, Overflow::Promote),
                    Ok(Variant::Null),
                    "{op:?} {t:?}"
                );
            }
        }
    }
}

#[test]
fn error_and_object_operands_are_rejected_by_every_operator() {
    let err = Variant::Error(VbaError::new(5));
    let obj = Variant::Object(vba_value::ObjectRef::Nothing);
    for t in TYPES {
        for other in [t.sample(2)] {
            for op in ARITH {
                // Re-raising the operand's own error is a deliberate VBA
                // behaviour kept where MS-VBAL says error 13.
                assert_eq!(
                    ops::arith(op, &err, &other, Overflow::Promote)
                        .unwrap_err()
                        .number(),
                    5
                );
                assert_eq!(
                    ops::arith(op, &other, &err, Overflow::Promote)
                        .unwrap_err()
                        .number(),
                    5
                );
                assert_eq!(
                    ops::arith(op, &obj, &other, Overflow::Promote).unwrap_err(),
                    VbaError::TYPE_MISMATCH
                );
                assert_eq!(
                    ops::arith(op, &other, &obj, Overflow::Promote).unwrap_err(),
                    VbaError::TYPE_MISMATCH
                );
            }
            for op in LOGIC {
                assert!(ops::logic(op, &obj, &other).is_err());
                assert!(ops::logic(op, &other, &err).is_err());
            }
            for op in COMPARE {
                assert!(ops::compare(op, &obj, &other, CompareOptions::default()).is_err());
            }
        }
    }
}

#[test]
fn logical_result_types_match_the_table_for_every_pair() {
    for op in LOGIC {
        for a in TYPES {
            for b in TYPES {
                // Samples of 1 and 2 keep the Long conversion in range.
                let got = ops::logic(op, &a.sample(1), &b.sample(2))
                    .unwrap_or_else(|e| panic!("{op:?} {a:?} {b:?}: error {}", e.number()));
                assert_eq!(type_name(&got), expected_logic(a, b), "{op:?} {a:?} {b:?}");
            }
        }
    }
}

#[test]
fn comparison_yields_boolean_or_null_and_mirrors_for_every_pair() {
    let mirror = |op: CompareOp| match op {
        CompareOp::Lt => CompareOp::Gt,
        CompareOp::Gt => CompareOp::Lt,
        CompareOp::Le => CompareOp::Ge,
        CompareOp::Ge => CompareOp::Le,
        o => o,
    };
    let mut all: Vec<Variant> = TYPES
        .iter()
        .flat_map(|t| [t.sample(2), t.sample(7)])
        .collect();
    all.push(Variant::Null);
    let opts = CompareOptions::default();
    for op in COMPARE {
        for a in &all {
            for b in &all {
                let got = ops::compare(op, a, b, opts).unwrap();
                if a.is_null() || b.is_null() {
                    assert_eq!(got, Variant::Null);
                } else {
                    assert!(matches!(got, Variant::Boolean(_)), "{op:?} {a:?} {b:?}");
                    assert_eq!(got, ops::compare(mirror(op), b, a, opts).unwrap());
                }
            }
        }
    }
    // Exactly one of <, =, > holds for every pair of non-Null values.
    for a in all.iter().filter(|v| !v.is_null()) {
        for b in all.iter().filter(|v| !v.is_null()) {
            let t = |op| ops::compare(op, a, b, opts).unwrap() == Variant::Boolean(true);
            let n = [CompareOp::Lt, CompareOp::Eq, CompareOp::Gt]
                .into_iter()
                .filter(|o| t(*o))
                .count();
            assert_eq!(n, 1, "{a:?} {b:?}");
        }
    }
}

// ------------------------------------------------------ integer boundaries

/// (VarType, lowest, highest) of the integral types.
const INTEGRAL: [(VarType, i64, i64); 4] = [
    (VarType::Byte, 0, 255),
    (VarType::Integer, -32_768, 32_767),
    (VarType::Long, -2_147_483_648, 2_147_483_647),
    (VarType::LongLong, i64::MIN, i64::MAX),
];

#[test]
fn every_integer_type_accepts_its_bounds_and_rejects_one_beyond() {
    for (vt, lo, hi) in INTEGRAL {
        for edge in [lo, hi] {
            let v = convert_to(&Variant::LongLong(edge), vt).unwrap();
            assert_eq!(to_longlong(&v).unwrap(), edge, "{vt:?}");
        }
        if lo > i64::MIN {
            assert_eq!(
                convert_to(&Variant::LongLong(lo - 1), vt),
                Err(VbaError::OVERFLOW),
                "{vt:?}"
            );
        }
        if hi < i64::MAX {
            assert_eq!(
                convert_to(&Variant::LongLong(hi + 1), vt),
                Err(VbaError::OVERFLOW),
                "{vt:?}"
            );
        }
    }
}

#[test]
fn typed_arithmetic_overflows_at_each_boundary_and_variants_promote() {
    let add = |l: Variant, r: Variant, m| ops::arith(ArithOp::Add, &l, &r, m);
    use Overflow::{Error, Promote};
    assert_eq!(
        add(Variant::Byte(255), Variant::Byte(1), Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        add(Variant::Byte(255), Variant::Byte(1), Promote),
        Ok(Variant::Integer(256))
    );
    assert_eq!(
        add(Variant::Integer(32_767), Variant::Integer(1), Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        add(Variant::Integer(32_767), Variant::Integer(1), Promote),
        Ok(Variant::Long(32_768))
    );
    assert_eq!(
        add(Variant::Long(i32::MAX), Variant::Long(1), Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        add(Variant::Long(i32::MAX), Variant::Long(1), Promote),
        Ok(Variant::Double(2_147_483_648.0))
    );
    assert_eq!(
        add(Variant::LongLong(i64::MAX), Variant::LongLong(1), Promote),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        ops::negate(&Variant::Integer(i16::MIN), Overflow::Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        ops::negate(&Variant::Integer(i16::MIN), Overflow::Promote),
        Ok(Variant::Long(32_768))
    );
}

#[test]
fn single_and_date_overflow_widen_to_double_only_for_variants() {
    use Overflow::{Error, Promote};
    let big = Variant::Single(3.0e38);
    let ten = Variant::Single(10.0);
    assert_eq!(
        ops::arith(ArithOp::Mul, &big, &ten, Error),
        Err(VbaError::OVERFLOW)
    );
    let wide = ops::arith(ArithOp::Mul, &big, &ten, Promote).unwrap();
    assert!(matches!(wide, Variant::Double(d) if d > 1.0e39));

    let last = Variant::Date(Date::from_serial(2_958_465.0).unwrap());
    let one = Variant::Long(1);
    assert_eq!(
        ops::arith(ArithOp::Add, &last, &one, Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        ops::arith(ArithOp::Add, &last, &one, Promote),
        Ok(Variant::Double(2_958_466.0))
    );
    assert_eq!(
        ops::negate(&last, Promote),
        Ok(Variant::Double(-2_958_465.0))
    );
    let first = Variant::Date(Date::from_serial(-657_434.0).unwrap());
    assert_eq!(
        ops::arith(ArithOp::Sub, &first, &one, Error),
        Err(VbaError::OVERFLOW)
    );
    assert_eq!(
        ops::arith(ArithOp::Sub, &first, &one, Promote),
        Ok(Variant::Double(-657_435.0))
    );
}

// ------------------------------------------------------------ Date edges

#[test]
fn date_range_edges() {
    assert!(Date::from_serial(-657_434.0).is_ok());
    assert!(Date::from_serial(-657_434.000_001).is_err());
    assert!(Date::from_serial(2_958_465.999_999).is_ok());
    assert_eq!(Date::from_serial(2_958_466.0), Err(VbaError::OVERFLOW));
    assert_eq!(Date::from_serial(f64::NAN), Err(VbaError::OVERFLOW));
    assert_eq!(Date::from_serial(f64::INFINITY), Err(VbaError::OVERFLOW));
}

// ------------------------------------------------------- string coercion

proptest! {
    #[test]
    fn blanks_around_the_sign_do_not_change_the_value(
        n in -1_000_000i64..1_000_000,
        pre in "[ \t]{0,3}",
        mid in "[ \t]{0,3}",
        post in "[ \t]{0,3}",
    ) {
        let plain = Variant::from(n.to_string().as_str());
        let sign = if n < 0 { "-" } else { "+" };
        let spaced = Variant::from(format!("{pre}{sign}{mid}{}{post}", n.unsigned_abs()).as_str());
        prop_assert_eq!(to_double(&spaced).unwrap(), to_double(&plain).unwrap());
        prop_assert_eq!(to_longlong(&spaced).unwrap(), n);
    }

    #[test]
    fn integral_values_survive_every_widening_path(n in any::<i16>()) {
        let v = Variant::Integer(n);
        let via_double = to_integer(&Variant::Double(to_double(&v).unwrap())).unwrap();
        let via_currency = to_integer(&Variant::Currency(to_currency(&v).unwrap())).unwrap();
        let via_decimal = to_integer(&Variant::Decimal(to_decimal(&v).unwrap())).unwrap();
        let via_text = to_integer(&Variant::String(to_vba_string(&v).unwrap())).unwrap();
        let via_long = to_integer(&Variant::Long(to_long(&v).unwrap())).unwrap();
        for back in [via_double, via_currency, via_decimal, via_text, via_long] {
            prop_assert_eq!(back, n);
        }
    }

    #[test]
    fn long_converts_to_byte_exactly_when_in_range(n in any::<i32>()) {
        match to_byte(&Variant::Long(n)) {
            Ok(b) => {
                prop_assert!((0..=255).contains(&n));
                prop_assert_eq!(i32::from(b), n);
            }
            Err(e) => {
                prop_assert_eq!(e, VbaError::OVERFLOW);
                prop_assert!(!(0..=255).contains(&n));
            }
        }
    }

    #[test]
    fn currency_scaling_is_ten_thousandths(c in any::<i64>()) {
        let cur = Currency::from_scaled(c);
        prop_assert_eq!(cur.scaled(), c);
        prop_assert_eq!(Decimal::from_currency(cur).to_currency().unwrap(), cur);
        // The Decimal view has exactly the four fractional digits of Currency.
        let d = Decimal::from_currency(cur);
        prop_assert_eq!(d.numeric_cmp(&Decimal::from_i64(0)).is_lt(), c < 0);
        if let Some(whole) = c.checked_mul(1).filter(|_| c % 10_000 == 0) {
            prop_assert_eq!(Currency::from_i64(whole / 10_000).unwrap(), cur);
        }
    }

    #[test]
    fn currency_integer_range_is_exact(n in -922_337_203_685_477i64..=922_337_203_685_477) {
        let cur = Currency::from_i64(n).unwrap();
        prop_assert_eq!(cur.scaled(), n * 10_000);
        prop_assert_eq!(cur.round_to_i64(), n);
    }

    #[test]
    fn decimal_rounding_is_idempotent_and_bounded(
        m in 0u128..(1u128 << 90), s in 0u8..=20, neg in any::<bool>(), places in 0u8..=10,
    ) {
        let d = Decimal::from_parts(m, s, neg).unwrap();
        let r = d.round(places);
        prop_assert_eq!(r.round(places).numeric_cmp(&r), std::cmp::Ordering::Equal);
        let half = Decimal::from_parts(5, places + 1, false).unwrap();
        let err = d.checked_sub(&r).unwrap().abs();
        prop_assert!(err.numeric_cmp(&half) != std::cmp::Ordering::Greater);
    }

    #[test]
    fn decimal_and_currency_agree_on_ordering(a in any::<i64>(), b in any::<i64>()) {
        let (ca, cb) = (Currency::from_scaled(a), Currency::from_scaled(b));
        let by_decimal = Decimal::from_currency(ca).numeric_cmp(&Decimal::from_currency(cb));
        prop_assert_eq!(by_decimal, a.cmp(&b));
        // The Variant comparison uses the same exact order.
        let lt = ops::compare(
            CompareOp::Lt,
            &Variant::Currency(ca),
            &Variant::Decimal(Decimal::from_currency(cb)),
            CompareOptions::default(),
        ).unwrap();
        prop_assert_eq!(lt, Variant::Boolean(a < b));
    }

    #[test]
    fn longlong_compares_exactly_with_its_text(a in any::<i64>(), b in any::<i64>()) {
        let text = Variant::from(b.to_string().as_str());
        let opts = CompareOptions { string_number: ops::StringNumberRule::Coerce, ..Default::default() };
        let got = ops::compare(CompareOp::Lt, &Variant::LongLong(a), &text, opts).unwrap();
        prop_assert_eq!(got, Variant::Boolean(a < b));
        let eq = ops::compare(CompareOp::Eq, &Variant::LongLong(a), &Variant::from(a.to_string().as_str()), opts).unwrap();
        prop_assert_eq!(eq, Variant::Boolean(true));
    }

    #[test]
    fn variant_addition_matches_wide_integer_arithmetic(a in any::<i16>(), b in any::<i16>()) {
        let wide = i64::from(a) + i64::from(b);
        let typed = ops::arith(ArithOp::Add, &Variant::Integer(a), &Variant::Integer(b), Overflow::Error);
        let var = ops::arith(ArithOp::Add, &Variant::Integer(a), &Variant::Integer(b), Overflow::Promote).unwrap();
        match typed {
            Ok(v) => prop_assert_eq!(v, Variant::Integer(wide as i16)),
            Err(e) => {
                prop_assert_eq!(e, VbaError::OVERFLOW);
                prop_assert!(i16::try_from(wide).is_err());
                prop_assert_eq!(var, Variant::Long(wide as i32));
            }
        }
    }

    #[test]
    fn date_plus_then_minus_returns_the_date(days in -600_000i32..2_900_000, k in -50_000i32..50_000) {
        let d = Variant::Date(Date::from_serial(f64::from(days)).unwrap());
        let n = Variant::Long(k);
        if let Ok(sum) = ops::arith(ArithOp::Add, &d, &n, Overflow::Error) {
            let back = ops::arith(ArithOp::Sub, &sum, &n, Overflow::Error).unwrap();
            prop_assert_eq!(back, d);
        } else {
            let wide = ops::arith(ArithOp::Add, &d, &n, Overflow::Promote).unwrap();
            prop_assert!(matches!(wide, Variant::Double(_)));
        }
    }
}

#[test]
fn date_converts_through_double_and_text() {
    for serial in [-657_434.0, -1.5, 0.0, 0.25, 36_526.75, 2_958_465.0] {
        let d = Date::from_serial(serial).unwrap();
        assert_eq!(to_double(&Variant::Date(d)).unwrap(), serial);
        assert_eq!(to_date(&Variant::Double(serial)).unwrap(), d);
    }
}
