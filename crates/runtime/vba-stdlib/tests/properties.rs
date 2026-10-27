//! Property tests.

mod common;
use common::*;
use proptest::prelude::*;
use vba_stdlib::format::{FormatEnv, format};
use vba_stdlib::host::Locale;
use vba_stdlib::like::like;
use vba_stdlib::strings;
use vba_value::{CompareMode, Date, Variant, VbaString};

fn vs(x: &str) -> VbaString {
    VbaString::from(x)
}

fn units(v: &VbaString) -> Vec<u16> {
    v.as_units().to_vec()
}

proptest! {
    #[test]
    fn split_join_round_trip(parts in proptest::collection::vec("[a-c]{0,3}", 1..6)) {
        let items: Vec<VbaString> = parts.iter().map(|p| vs(p)).collect();
        let arr = strings::string_array(items.clone()).unwrap();
        let joined = strings::join(&arr, &vs(",")).unwrap();
        let back = strings::split(&joined, &vs(","), -1, CompareMode::Binary).unwrap();
        if joined.is_empty() {
            prop_assert!(back.is_empty());
        } else {
            prop_assert_eq!(back, items);
        }
    }

    #[test]
    fn split_limit_keeps_the_rest(parts in proptest::collection::vec("[a-c]{1,3}", 2..6), limit in 1i32..6) {
        let joined = parts.join(",");
        let got = strings::split(&vs(&joined), &vs(","), limit, CompareMode::Binary).unwrap();
        prop_assert!(got.len() <= limit as usize);
        let rejoined = got.iter().map(|g| g.to_string_lossy()).collect::<Vec<_>>().join(",");
        prop_assert_eq!(rejoined, joined);
    }

    #[test]
    fn instr_is_the_first_occurrence(h in "[abAB]{0,10}", n in "[abAB]{1,3}", start in 1i32..12) {
        for mode in [CompareMode::Binary, CompareMode::Text] {
            let r = strings::instr(start, &vs(&h), &vs(&n), mode).unwrap();
            let fold = |s: &str| -> Vec<char> {
                s.chars().map(|c| if mode == CompareMode::Text { c.to_ascii_lowercase() } else { c }).collect()
            };
            let (hc, nc) = (fold(&h), fold(&n));
            let naive = (start as usize - 1..hc.len())
                .find(|&i| i + nc.len() <= hc.len() && hc[i..i + nc.len()] == nc[..])
                .map_or(0, |i| i as i32 + 1);
            let naive = if (start as usize) > hc.len() { 0 } else { naive };
            prop_assert_eq!(r, naive);
        }
    }

    #[test]
    fn instr_rev_is_the_last_occurrence(h in "[ab]{0,10}", n in "[ab]{1,3}") {
        let r = strings::instr_rev(&vs(&h), &vs(&n), -1, CompareMode::Binary).unwrap();
        let naive = h.rfind(&n).map_or(0, |i| i as i32 + 1);
        prop_assert_eq!(r, naive);
    }

    #[test]
    fn left_mid_partition(s in "[a-z]{0,12}", k in 0i32..14) {
        let v = vs(&s);
        let l = strings::left(&v, k).unwrap();
        let m = strings::mid(&v, k + 1, None).unwrap();
        prop_assert_eq!(units(&l.concat(&m)), units(&v));
        let r = strings::right(&v, k).unwrap();
        prop_assert_eq!(r.len(), (k as usize).min(v.len()));
    }

    #[test]
    fn replace_removing_a_char_shortens_by_its_count(s in "[abc]{0,12}") {
        let r = strings::replace(&vs(&s), &vs("a"), &vs(""), 1, -1, CompareMode::Binary).unwrap();
        prop_assert_eq!(r.len(), s.chars().filter(|&c| c != 'a').count());
        let again = strings::replace(&r, &vs("a"), &vs("x"), 1, -1, CompareMode::Binary).unwrap();
        prop_assert_eq!(units(&again), units(&r));
    }

    #[test]
    fn like_matches_reference(text in "[ab1]{0,6}", pat in proptest::collection::vec(
        prop_oneof![Just("a"), Just("b"), Just("1"), Just("?"), Just("*"), Just("#"), Just("[ab]"), Just("[!a]"), Just("[a-b]"), Just("[1]")], 0..6)) {
        let pattern: String = pat.concat();
        let got = like(&vs(&text), &vs(&pattern), CompareMode::Binary).unwrap();
        let t: Vec<char> = text.chars().collect();
        let p: Vec<char> = pattern.chars().collect();
        prop_assert_eq!(got, reference_like(&t, &p), "{} Like {}", text, pattern);
    }

    /// An invalid element after a literal that cannot match is never reached
    /// (MS-VBAL 5.6.9.6), and it is error 93 when matching gets there.
    #[test]
    fn like_invalid_tail_only_errors_when_reached(text in "[ab]{1,5}", tail in prop_oneof![Just("["), Just("[z-a]")]) {
        let first = text.chars().next().unwrap();
        let other = if first == 'a' { 'b' } else { 'a' };
        let miss = format!("{other}{tail}");
        prop_assert_eq!(like(&vs(&text), &vs(&miss), CompareMode::Binary), Ok(false));
        let hit = format!("{first}*{tail}");
        prop_assert!(like(&vs(&text), &vs(&hit), CompareMode::Binary).is_err());
    }

    #[test]
    fn like_literal_is_equality(a in "[abAB]{0,5}", b in "[abAB]{0,5}") {
        prop_assert_eq!(like(&vs(&a), &vs(&b), CompareMode::Binary).unwrap(), a == b);
        prop_assert_eq!(
            like(&vs(&a), &vs(&b), CompareMode::Text).unwrap(),
            a.to_lowercase() == b.to_lowercase()
        );
    }

    #[test]
    fn format_zero_matches_integer_text(x in -1_000_000i32..1_000_000) {
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        let got = format(&Variant::Long(x), Some(&vs("0")), &env).unwrap();
        prop_assert_eq!(got, Variant::from_text(&x.to_string()));
    }

    #[test]
    fn format_grouped_matches_manual_grouping(x in 0u32..1_000_000_000) {
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        let got = format(&Variant::Long(x as i32), Some(&vs("#,##0")), &env).unwrap();
        let digits = x.to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        prop_assert_eq!(got, Variant::from_text(&out));
    }

    #[test]
    fn format_fixed_decimals(cents in -10_000_000i64..10_000_000) {
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        let x = cents as f64 / 100.0;
        let got = format(&Variant::Double(x), Some(&vs("0.00")), &env).unwrap();
        let sign = if cents < 0 { "-" } else { "" };
        let a = cents.abs();
        prop_assert_eq!(got, Variant::from_text(&format!("{sign}{}.{:02}", a / 100, a % 100)));
    }

    #[test]
    fn date_add_diff_round_trip(day in 36_526i32..60_000, delta in -5_000i32..5_000) {
        let mut e = Env::new();
        let base = Variant::Date(Date::from_serial(f64::from(day)).unwrap());
        let moved = e.call("DateAdd", &[s("d"), Variant::Long(delta), base.clone()]).unwrap();
        let diff = e.call("DateDiff", &[s("d"), base, moved]).unwrap();
        prop_assert_eq!(diff, Variant::Long(delta));
    }

    #[test]
    fn date_add_month_clamps_and_diffs(day in 36_526i32..60_000, months in -240i32..240) {
        let mut e = Env::new();
        let base = Variant::Date(Date::from_serial(f64::from(day)).unwrap());
        let moved = e.call("DateAdd", &[s("m"), Variant::Long(months), base.clone()]).unwrap();
        let diff = e.call("DateDiff", &[s("m"), base, moved]).unwrap();
        prop_assert_eq!(diff, Variant::Long(months));
    }

    #[test]
    fn hex_round_trips(x in any::<i32>()) {
        let mut e = Env::new();
        let h = e.call("Hex", &[Variant::Long(x)]).unwrap();
        let Variant::String(h) = h else { panic!() };
        let back = u32::from_str_radix(&h.to_string_lossy(), 16).unwrap();
        prop_assert_eq!(back as i32, x);
        let v = e.call("Val", &[Variant::from_text(&format!("&H{}&", h.to_string_lossy()))]).unwrap();
        prop_assert_eq!(v, Variant::Double(f64::from(x)));
    }

    #[test]
    fn rnd_is_in_unit_interval_and_deterministic(seed in -1000i32..0, n in 1usize..50) {
        let mut a = Env::new();
        let mut b = Env::new();
        a.call("Rnd", &[Variant::Long(seed)]).unwrap();
        b.call("Rnd", &[Variant::Long(seed)]).unwrap();
        for _ in 0..n {
            let (x, y) = (a.call("Rnd", &[]).unwrap(), b.call("Rnd", &[]).unwrap());
            prop_assert_eq!(&x, &y);
            let Variant::Single(f) = x else { panic!() };
            prop_assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn round_is_idempotent_and_close(x in -1.0e6f64..1.0e6, p in 0i32..6) {
        let mut e = Env::new();
        let r = e.call("Round", &[Variant::Double(x), Variant::Long(p)]).unwrap();
        let again = e.call("Round", &[r.clone(), Variant::Long(p)]).unwrap();
        prop_assert_eq!(&r, &again);
        let Variant::Double(r) = r else { panic!() };
        prop_assert!((r - x).abs() <= 0.5 * 10f64.powi(-p) + 1e-9);
    }

    #[test]
    fn pmt_pv_nper_consistent(rate in 0.001f64..0.05, n in 6u32..120, pv in 1000.0f64..100000.0) {
        use vba_stdlib::financial::{nper, pmt, fv};
        let p = pmt(rate, f64::from(n), pv, 0.0, false).unwrap();
        prop_assert!((nper(rate, p, pv, 0.0, false).unwrap() - f64::from(n)).abs() < 1e-6);
        prop_assert!(fv(rate, f64::from(n), p, pv, false).unwrap().abs() < 1e-4);
    }
}

/// An independent backtracking implementation of `Like` for the restricted
/// pattern alphabet used above (`a b 1 ? * # [ab] [!a] [a-b] [1]`).
fn reference_like(t: &[char], p: &[char]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    match p[0] {
        '*' => (0..=t.len()).any(|k| reference_like(&t[k..], &p[1..])),
        '?' => !t.is_empty() && reference_like(&t[1..], &p[1..]),
        '#' => !t.is_empty() && t[0].is_ascii_digit() && reference_like(&t[1..], &p[1..]),
        '[' => {
            let close = p.iter().position(|&c| c == ']').unwrap();
            let body = &p[1..close];
            let (neg, body) = match body.first() {
                Some('!') => (true, &body[1..]),
                _ => (false, body),
            };
            let hit = |c: char| {
                if body.len() == 3 && body[1] == '-' {
                    body[0] <= c && c <= body[2]
                } else {
                    body.contains(&c)
                }
            };
            !t.is_empty() && (hit(t[0]) != neg) && reference_like(&t[1..], &p[close + 1..])
        }
        c => !t.is_empty() && t[0] == c && reference_like(&t[1..], &p[1..]),
    }
}
