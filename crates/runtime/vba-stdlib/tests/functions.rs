//! Table-driven tests of the intrinsics through the registry.

mod common;
use common::*;
use vba_stdlib::{Kind, StdError, lookup};
use vba_value::{CompareMode, Variant};

type Case = (&'static str, Vec<Variant>, Result<Variant, i32>);

fn run(cases: Vec<Case>) {
    let mut e = Env::new();
    for (name, args, want) in cases {
        let got = e.call(name, &args);
        let want = want.map_err(err);
        assert_eq!(got, want, "{name}({args:?})");
    }
}

fn st(x: &str) -> Result<Variant, i32> {
    Ok(s(x))
}
fn i(x: i16) -> Result<Variant, i32> {
    Ok(Variant::Integer(x))
}
fn l(x: i32) -> Result<Variant, i32> {
    Ok(Variant::Long(x))
}
fn dbl(x: f64) -> Result<Variant, i32> {
    Ok(Variant::Double(x))
}
fn b(x: bool) -> Result<Variant, i32> {
    Ok(Variant::Boolean(x))
}
const NULL: Result<Variant, i32> = Ok(Variant::Null);
fn n(x: i32) -> Variant {
    Variant::Long(x)
}

#[test]
fn instr_with_omitted_start_and_compare() {
    // The shape a compiler binds `InStr("abcabc", "c")` to: four slots.
    let mut e = Env::new();
    let args = [None, Some(s("abcabc")), Some(s("c")), None];
    let got = e.call_opt("InStr", &args);
    assert_eq!(got, Ok(Variant::Long(3)));
    let args = [Some(n(4)), Some(s("abcabc")), Some(s("c")), None];
    assert_eq!(e.call_opt("InStr", &args), Ok(Variant::Long(6)));
}

#[test]
fn left_right_mid() {
    run(vec![
        ("Left", vec![s("abc"), n(2)], st("ab")),
        ("left", vec![s("abc"), n(5)], st("abc")),
        ("Left", vec![s("abc"), n(0)], st("")),
        ("Left", vec![s("abc"), n(-1)], Err(5)),
        ("Left", vec![Variant::Null, n(1)], NULL),
        ("Left$", vec![Variant::Null, n(1)], Err(94)),
        ("Left", vec![Variant::Long(12345), n(2)], st("12")),
        ("Right", vec![s("abc"), n(2)], st("bc")),
        ("Right", vec![s("abc"), n(9)], st("abc")),
        ("Right$", vec![s("abc"), n(-2)], Err(5)),
        ("Mid", vec![s("abcdef"), n(3), n(2)], st("cd")),
        ("Mid", vec![s("abcdef"), n(3)], st("cdef")),
        ("Mid", vec![s("abc"), n(4)], st("")),
        ("Mid", vec![s("abc"), n(0)], Err(5)),
        ("Mid", vec![s("abc"), n(1), n(-1)], Err(5)),
        ("Mid", vec![s("abc"), n(2), n(99)], st("bc")),
        ("Mid$", vec![Variant::Null, n(1)], Err(94)),
        ("LeftB", vec![s("abc"), n(4)], st("ab")),
        ("MidB", vec![s("abcd"), n(3), n(4)], st("bc")),
        ("RightB", vec![s("abc"), n(2)], st("c")),
    ]);
    let mut e = Env::new();
    assert!(matches!(
        e.call("LeftB", &[s("abc"), n(3)]),
        Err(StdError::Unsupported(_))
    ));
}

#[test]
fn len_trim_case() {
    run(vec![
        ("Len", vec![s("abc")], l(3)),
        ("Len", vec![s("")], l(0)),
        ("Len", vec![Variant::Null], NULL),
        ("Len", vec![n(123)], l(3)),
        ("LenB", vec![s("abc")], l(6)),
        ("LenB", vec![Variant::Integer(1)], l(2)),
        ("LenB", vec![Variant::Double(1.0)], l(8)),
        ("Trim", vec![s("  a b  ")], st("a b")),
        ("Trim", vec![s("\ta\t")], st("\ta\t")),
        ("LTrim", vec![s("  a ")], st("a ")),
        ("RTrim$", vec![s("  a ")], st("  a")),
        ("Trim", vec![Variant::Null], NULL),
        ("Trim$", vec![Variant::Null], Err(94)),
        ("UCase", vec![s("abc\u{e9}")], st("ABC\u{c9}")),
        ("LCase", vec![s("ABC")], st("abc")),
        ("LCase", vec![Variant::Null], NULL),
        ("UCase$", vec![s("x")], st("X")),
    ]);
}

#[test]
fn instr_family() {
    run(vec![
        ("InStr", vec![s("abcabc"), s("c")], l(3)),
        ("InStr", vec![n(4), s("abcabc"), s("c")], l(6)),
        ("InStr", vec![n(1), s("abc"), s("B"), n(1)], l(2)),
        ("InStr", vec![n(1), s("abc"), s("B"), n(0)], l(0)),
        ("InStr", vec![n(1), s("abc"), s("B"), n(2)], Err(5)),
        ("InStr", vec![s("abc"), s("")], l(1)),
        ("InStr", vec![s(""), s("a")], l(0)),
        ("InStr", vec![n(9), s("abc"), s("a")], l(0)),
        ("InStr", vec![n(0), s("abc"), s("a")], Err(5)),
        ("InStr", vec![s("abc"), Variant::Null], NULL),
        ("InStr", vec![Variant::Null, s("abc"), s("a")], Err(94)),
        ("InStrB", vec![s("abc"), s("c")], l(5)),
        ("InStrRev", vec![s("abcabc"), s("bc")], l(5)),
        ("InStrRev", vec![s("abcabc"), s("bc"), n(4)], l(2)),
        ("InStrRev", vec![s("abcabc"), s("BC"), n(-1), n(1)], l(5)),
        ("InStrRev", vec![s("abc"), s("x")], l(0)),
        ("InStrRev", vec![s("abc"), s("a"), n(0)], Err(5)),
        ("InStrRev", vec![s("abc"), s("")], l(3)),
        ("InStrRev", vec![Variant::Null, s("a")], NULL),
    ]);
}

#[test]
fn compare_option_applies_when_omitted() {
    let mut e = Env::new();
    e.compare = CompareMode::Text;
    assert_eq!(e.call("InStr", &[s("abc"), s("B")]), Ok(n(2)));
    assert_eq!(
        e.call("StrComp", &[s("a"), s("A")]),
        Ok(Variant::Integer(0))
    );
    assert_eq!(e.call("InStr", &[n(1), s("abc"), s("B"), n(0)]), Ok(n(0)));
    assert_eq!(e.call("InStr", &[n(1), s("abc"), s("B"), n(-1)]), Ok(n(2)));
}

#[test]
fn replace_space_string_reverse() {
    run(vec![
        ("Replace", vec![s("banana"), s("an"), s("AN")], st("bANANa")),
        (
            "Replace",
            vec![s("banana"), s("an"), s("AN"), n(1), n(1)],
            st("bANana"),
        ),
        (
            "Replace",
            vec![s("banana"), s("an"), s("AN"), n(3)],
            st("nANa"),
        ),
        (
            "Replace",
            vec![s("banana"), s("AN"), s("x"), n(1), n(-1), n(1)],
            st("bxxa"),
        ),
        ("Replace", vec![s("abc"), s(""), s("x")], st("abc")),
        ("Replace", vec![s("abc"), s("b"), s("")], st("ac")),
        ("Replace", vec![s("abc"), s("b"), s("x"), n(0)], Err(5)),
        (
            "Replace",
            vec![s("abc"), s("b"), s("x"), n(1), n(-2)],
            Err(5),
        ),
        ("Replace", vec![Variant::Null, s("b"), s("x")], Err(94)),
        ("Replace", vec![s(""), s("b"), s("x")], st("")),
        ("Space", vec![n(3)], st("   ")),
        ("Space", vec![n(-1)], Err(5)),
        ("Space", vec![Variant::Null], NULL),
        ("Space$", vec![Variant::Null], Err(94)),
        ("String", vec![n(3), s("ab")], st("aaa")),
        ("String", vec![n(3), n(65)], st("AAA")),
        ("String", vec![n(2), n(321)], st("AA")),
        ("String", vec![n(2), n(-1)], Err(5)),
        ("String", vec![n(2), s("")], Err(5)),
        ("String", vec![n(-2), s("a")], Err(5)),
        ("StrReverse", vec![s("abc")], st("cba")),
        ("StrReverse", vec![s("")], st("")),
        ("StrReverse", vec![Variant::Null], Err(94)),
    ]);
}

#[test]
fn asc_chr() {
    run(vec![
        ("Asc", vec![s("A")], i(65)),
        ("Asc", vec![s("ABC")], i(65)),
        ("Asc", vec![s("")], Err(5)),
        ("Asc", vec![Variant::Null], Err(94)),
        ("Asc", vec![s("\u{20ac}")], i(128)),
        ("Asc", vec![s("\u{e9}")], i(233)),
        ("AscW", vec![s("\u{20ac}")], i(8364)),
        ("AscW", vec![s("\u{ffff}")], i(-1)),
        ("AscB", vec![s("A")], i(65)),
        ("Chr", vec![n(65)], st("A")),
        ("Chr$", vec![n(65)], st("A")),
        ("Chr", vec![n(128)], st("\u{20ac}")),
        ("Chr", vec![n(256)], Err(5)),
        ("Chr", vec![n(-1)], Err(5)),
        ("Chr", vec![Variant::Null], Err(94)),
        ("ChrW", vec![n(8364)], st("\u{20ac}")),
        ("ChrW", vec![n(-1)], st("\u{ffff}")),
        ("ChrW", vec![n(65536)], Err(5)),
        ("ChrW", vec![n(-32769)], Err(5)),
    ]);
    let mut e = Env::new();
    assert!(matches!(
        e.call("Asc", &[s("\u{4e2d}")]),
        Err(StdError::Unsupported(_))
    ));
    assert!(matches!(
        e.call("ChrB", &[n(65)]),
        Err(StdError::Unsupported(_))
    ));
    assert_eq!(e.call("ChrB", &[n(256)]), Err(err(5)));
}

/// Examples of the Microsoft Learn pages for `Round` and `Split`.
#[test]
fn learn_round_and_split_examples() {
    let d = |x: f64, want: f64| ("Round", vec![Variant::Double(x), n(4)], dbl(want));
    run(vec![
        d(0.12335, 0.1234),
        d(0.12345, 0.1234),
        d(0.12355, 0.1236),
        d(0.12365, 0.1236),
        d(0.00005, 0.0),
    ]);
    let mut e = Env::new();
    let full = "Dow - Fonseca - Graham - Kopke - Noval - Offley - Sandeman - Taylor - Warre";
    let Variant::Array(a) = e.call("Split", &[s(full), s("-")]).unwrap() else {
        panic!()
    };
    assert_eq!((a.lbound(1), a.ubound(1)), (Ok(0), Ok(8)));
    assert_eq!(a.get(&[0]).unwrap(), &s("Dow "));
    assert_eq!(a.get(&[1]).unwrap(), &s(" Fonseca "));
    let Variant::Array(a) = e.call("Split", &[s(full), s(" - ")]).unwrap() else {
        panic!()
    };
    assert_eq!(a.ubound(1), Ok(8));
    assert_eq!(a.get(&[0]).unwrap(), &s("Dow"));
    assert_eq!(a.get(&[3]).unwrap(), &s("Kopke"));
    // Zero-length expression: "an empty array, that is, an array with no
    // elements"; zero-length delimiter: a single-element array.
    let Variant::Array(a) = e.call("Split", &[s("")]).unwrap() else {
        panic!()
    };
    assert_eq!((a.len(), a.ubound(1)), (0, Ok(-1)));
    let Variant::Array(a) = e.call("Split", &[s("abc"), s("")]).unwrap() else {
        panic!()
    };
    assert_eq!((a.len(), a.get(&[0]).unwrap()), (1, &s("abc")));
}

/// Behavioral rules of the Microsoft Learn `Rnd` page (not the generator's
/// exact arithmetic, which stays unverified).
#[test]
fn learn_rnd_rules() {
    let mut e = Env::new();
    let sng = |x: f64| Variant::Single(x as f32);
    let a = e.call("Rnd", &[sng(-3.0)]).unwrap();
    let b = e.call("Rnd", &[sng(-3.0)]).unwrap();
    assert_eq!(a, b, "negative argument: the same number every time");
    let next = e.call("Rnd", &[sng(1.0)]).unwrap();
    assert_ne!(next, a);
    assert_eq!(
        e.call("Rnd", &[sng(0.0)]).unwrap(),
        next,
        "zero: last number"
    );
    let Variant::Single(x) = next else { panic!() };
    assert!((0.0..1.0).contains(&x));
    // Rnd(-1) then Randomize n repeats the sequence.
    let mut seq = || {
        e.call("Rnd", &[sng(-1.0)]).unwrap();
        e.call("Randomize", &[n(7)]).unwrap();
        (0..3)
            .map(|_| e.call("Rnd", &[]).unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(seq(), seq());
}

#[test]
fn text_compare_ignores_case_width_and_kana() {
    let mut e = Env::new();
    // vbTextCompare = 1
    assert_eq!(
        e.call("StrComp", &[s("ABC"), s("ａｂｃ"), n(1)]),
        Ok(Variant::Integer(0))
    );
    assert_eq!(
        e.call("StrComp", &[s("ABC"), s("ａｂｃ"), n(0)]),
        Ok(Variant::Integer(-1))
    );
    assert_eq!(
        e.call("StrComp", &[s("ひら"), s("ヒラ"), n(1)]),
        Ok(Variant::Integer(0))
    );
    assert_eq!(
        e.call("StrComp", &[s("e"), s("é"), n(1)]),
        Ok(Variant::Integer(-1))
    );
    assert_eq!(
        e.call("InStr", &[n(1), s("xxＡＢx"), s("ab"), n(1)]),
        Ok(Variant::Long(3))
    );
    assert_eq!(
        e.call(
            "Replace",
            &[s("カタカナ"), s("かな"), s("-"), n(1), n(-1), n(1)]
        ),
        Ok(s("カタ-"))
    );
}

#[test]
fn custom_collation_is_honored() {
    use std::cmp::Ordering;
    use vba_stdlib::strings::{instr_with, split_with, str_comp_with};
    use vba_value::collation::Collation;
    // Treats every ASCII digit as the same character.
    struct Digits;
    impl Collation for Digits {
        fn fold(&self, units: &[u16]) -> Vec<u16> {
            units
                .iter()
                .map(|&u| if (0x30..=0x39).contains(&u) { 0x30 } else { u })
                .collect()
        }
        fn compare(&self, a: &[u16], b: &[u16]) -> Ordering {
            self.fold(a).cmp(&self.fold(b))
        }
    }
    let v = |x: &str| vba_value::VbaString::from(x);
    let t = CompareMode::Text;
    assert_eq!(instr_with(1, &v("ab7"), &v("1"), t, &Digits), Ok(3));
    assert_eq!(
        instr_with(1, &v("ab7"), &v("1"), CompareMode::Binary, &Digits),
        Ok(0)
    );
    assert_eq!(str_comp_with(&v("a1"), &v("a9"), t, &Digits), 0);
    assert_eq!(
        split_with(&v("a1b2c"), &v("9"), -1, t, &Digits)
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn byte_view_of_strings() {
    // Byte semantics are the UTF-16LE bytes: LenB = 2 * Len in code units.
    let smile = "\u{1F600}"; // two code units, four bytes
    let mut e = Env::new();
    assert_eq!(e.call("LenB", &[s(smile)]), Ok(Variant::Long(4)));
    assert_eq!(e.call("Len", &[s(smile)]), Ok(Variant::Long(2)));
    assert_eq!(e.call("AscB", &[s("A")]), Ok(Variant::Integer(0x41)));
    assert_eq!(e.call("AscB", &[s("\u{0141}")]), Ok(Variant::Integer(0x41)));
    assert_eq!(e.call("LeftB", &[s("abc"), n(2)]), Ok(s("a")));
    assert_eq!(e.call("RightB", &[s("abc"), n(4)]), Ok(s("bc")));
    assert_eq!(e.call("MidB", &[s("abc"), n(3), n(2)]), Ok(s("b")));
    assert_eq!(e.call("InStrB", &[s("abc"), s("b")]), Ok(Variant::Long(3)));
    // Policy: results with an odd byte count cannot be a UTF-16 string and
    // are an explicit Unsupported error, never silently rounded.
    for (f, args) in [
        ("LeftB", vec![s("abc"), n(3)]),
        ("RightB", vec![s("abc"), n(1)]),
        ("MidB", vec![s("abc"), n(2), n(3)]),
    ] {
        assert!(
            matches!(e.call(f, &args), Err(StdError::Unsupported(_))),
            "{f}"
        );
    }
    assert_eq!(e.call("LeftB", &[s("abc"), n(-1)]), Err(err(5)));
}

#[test]
fn strcomp_strconv() {
    run(vec![
        (
            "StrComp",
            vec![s("a"), s("B"), n(0)],
            Ok(Variant::Integer(1)),
        ),
        (
            "StrComp",
            vec![s("a"), s("B"), n(1)],
            Ok(Variant::Integer(-1)),
        ),
        ("StrComp", vec![s("a"), s("a")], Ok(Variant::Integer(0))),
        ("StrComp", vec![Variant::Null, s("a")], NULL),
        ("StrComp", vec![s("a"), s("b"), n(5)], Err(5)),
        ("StrConv", vec![s("hello world"), n(3)], st("Hello World")),
        ("StrConv", vec![s("Hello"), n(1)], st("HELLO")),
        ("StrConv", vec![s("Hello"), n(2)], st("hello")),
        ("StrConv", vec![s("Hello"), n(-5)], Err(5)),
    ]);
    let mut e = Env::new();
    // Width and kana.
    run(vec![
        ("StrConv", vec![s("Ab1"), n(4)], st("Ａｂ１")),
        ("StrConv", vec![s("Ａｂ１"), n(8)], st("Ab1")),
        ("StrConv", vec![s("ひら"), n(16)], st("ヒラ")),
        ("StrConv", vec![s("ヒラ"), n(32)], st("ひら")),
        ("StrConv", vec![s("ab"), n(4 | 1)], st("ＡＢ")),
        ("StrConv", vec![s("ｶﾞ"), n(4 | 32)], st("が")),
        ("StrConv", vec![s("a"), n(4 | 8)], Err(5)),
        ("StrConv", vec![s("a"), n(16 | 32)], Err(5)),
        ("StrConv", vec![s("a"), n(65)], Err(5)),
        ("StrConv", vec![s("a"), n(64 | 128)], Err(5)),
        ("StrConv", vec![s("a"), n(2048)], Err(5)),
    ]);
    // Unicode <-> ANSI bytes: two bytes pack into one UTF-16 unit.
    let units = |u: &[u16]| Variant::String(vba_value::VbaString::from_units(u.to_vec()));
    assert_eq!(e.call("StrConv", &[s("ab"), n(128)]), Ok(units(&[0x6261])));
    assert_eq!(e.call("StrConv", &[units(&[0x6261]), n(64)]), Ok(s("ab")));
    assert_eq!(
        e.call("StrConv", &[s("\u{e9}\u{3a9}"), n(128)]),
        Ok(units(&[0x3FE9]))
    );
    // Japanese LocaleID selects code page 932.
    assert_eq!(
        e.call("StrConv", &[s("\u{3042}\u{3044}"), n(128), n(0x411)]),
        Ok(units(&[0xA082, 0xA282]))
    );
    assert_eq!(
        e.call("StrConv", &[units(&[0xA082, 0xA282]), n(64), n(0x411)]),
        Ok(s("\u{3042}\u{3044}"))
    );
    // Odd byte counts and unsupported pages/flags are explicit errors.
    for (text, mode, lcid) in [
        ("a", 128, 0),
        ("a", 256, 0),
        ("a", 512, 0),
        ("ab", 128, 0x1234),
    ] {
        assert!(
            matches!(
                e.call("StrConv", &[s(text), n(mode), n(lcid)]),
                Err(StdError::Unsupported(_))
            ),
            "{mode} {lcid}"
        );
    }
    // vbLinguisticCasing (1024) is valid with the upper/lower case modes only.
    assert_eq!(e.call("StrConv", &[s("aB"), n(1025)]), Ok(s("AB")));
    assert_eq!(e.call("StrConv", &[s("aB"), n(1026)]), Ok(s("ab")));
    assert_eq!(e.call("StrConv", &[s("aB"), n(1024)]), Err(err(5)));
    assert_eq!(e.call("StrConv", &[s("aB"), n(1027)]), Err(err(5)));
    // Typed API with explicit code pages.
    use vba_stdlib::strings::{asc_cp, chr_cp, str_conv_cp};
    let v = |x: &str| vba_value::VbaString::from(x);
    assert_eq!(asc_cp(&v("\u{3042}"), 932).unwrap(), 0x82A0u16 as i16);
    assert_eq!(asc_cp(&v("A"), 932).unwrap(), 65);
    assert_eq!(asc_cp(&v("\u{0416}"), 1251).unwrap(), 0xC6);
    assert!(matches!(
        asc_cp(&v("\u{3042}"), 1252),
        Err(StdError::Unsupported(_))
    ));
    assert!(matches!(
        asc_cp(&v("a"), 437),
        Err(StdError::Unsupported(_))
    ));
    assert_eq!(chr_cp(0x82A0, 932).unwrap(), v("\u{3042}"));
    assert_eq!(chr_cp(0xC6, 1251).unwrap(), v("\u{0416}"));
    assert!(chr_cp(0x82A0, 1251).is_err());
    assert!(chr_cp(256, 1252).is_err());
    assert_eq!(
        str_conv_cp(&v("\u{3042}\u{3044}"), 128, 932)
            .unwrap()
            .as_units(),
        &[0xA082, 0xA282]
    );
}

#[test]
fn split_join_filter() {
    let mut e = Env::new();
    let parts = e.call("Split", &[s("a,b,c"), s(",")]).unwrap();
    let Variant::Array(a) = &parts else { panic!() };
    assert_eq!((a.lbound(1).unwrap(), a.ubound(1).unwrap()), (0, 2));
    assert_eq!(a.get(&[1]).unwrap(), &s("b"));
    assert_eq!(e.call("Join", &[parts.clone(), s("-")]), Ok(s("a-b-c")));
    assert_eq!(e.call("Join", std::slice::from_ref(&parts)), Ok(s("a b c")));
    let two = e.call("Split", &[s("a,b,c"), s(","), n(2)]).unwrap();
    let Variant::Array(a) = &two else { panic!() };
    assert_eq!(a.get(&[1]).unwrap(), &s("b,c"));
    let one = e.call("Split", &[s("a b")]).unwrap();
    let Variant::Array(a) = &one else { panic!() };
    assert_eq!(a.ubound(1), Ok(1));
    let whole = e.call("Split", &[s("abc"), s("")]).unwrap();
    let Variant::Array(a) = &whole else { panic!() };
    assert_eq!(a.ubound(1), Ok(0));
    // Empty input: an allocated array without elements.
    let Variant::Array(a) = e.call("Split", &[s("")]).unwrap() else {
        panic!()
    };
    assert!(a.is_allocated());
    assert_eq!((a.lbound(1), a.ubound(1)), (Ok(0), Ok(-1)));
    // Text-compare delimiter.
    let t = e.call("Split", &[s("aXbxc"), s("x"), n(-1), n(1)]).unwrap();
    let Variant::Array(a) = &t else { panic!() };
    assert_eq!(a.ubound(1), Ok(2));
    assert_eq!(e.call("Split", &[s("a"), s(","), n(-5)]), Err(err(5)));
    assert_eq!(e.call("Split", &[Variant::Null]), Err(err(94)));
    assert_eq!(e.call("Join", &[s("x")]), Err(err(5)));

    let src = arr(&[s("apple"), s("banana"), s("cherry"), s("Banana split")]);
    let f = e.call("Filter", &[src.clone(), s("an")]).unwrap();
    let Variant::Array(a) = &f else { panic!() };
    assert_eq!((a.ubound(1), a.get(&[0]).unwrap()), (Ok(1), &s("banana")));
    let f = e
        .call(
            "Filter",
            &[src.clone(), s("BAN"), Variant::Boolean(true), n(1)],
        )
        .unwrap();
    let Variant::Array(a) = &f else { panic!() };
    assert_eq!(a.ubound(1), Ok(1));
    let f = e
        .call("Filter", &[src.clone(), s("an"), Variant::Boolean(false)])
        .unwrap();
    let Variant::Array(a) = &f else { panic!() };
    assert_eq!(a.ubound(1), Ok(1));
    assert_eq!(e.call("Filter", &[s("x"), s("a")]), Err(err(5)));
}

#[test]
fn conversion_functions() {
    run(vec![
        ("Hex", vec![n(255)], st("FF")),
        ("Hex$", vec![n(255)], st("FF")),
        ("Hex", vec![Variant::Integer(-1)], st("FFFF")),
        ("Hex", vec![n(-1)], st("FFFFFFFF")),
        ("Hex", vec![Variant::Byte(255)], st("FF")),
        ("Hex", vec![Variant::Double(255.5)], st("100")),
        ("Hex", vec![Variant::Double(254.5)], st("FE")),
        ("Hex", vec![Variant::Double(3_000_000_000.0)], Err(6)),
        ("Hex", vec![Variant::Empty], st("0")),
        ("Hex", vec![Variant::Null], NULL),
        ("Hex$", vec![Variant::Null], Err(94)),
        ("Hex", vec![Variant::LongLong(-1)], st("FFFFFFFFFFFFFFFF")),
        ("Oct", vec![n(8)], st("10")),
        ("Oct", vec![Variant::Integer(-1)], st("177777")),
        ("Str", vec![n(5)], st(" 5")),
        ("Str", vec![n(-5)], st("-5")),
        ("Str", vec![Variant::Double(1.5)], st(" 1.5")),
        ("Str", vec![Variant::Boolean(true)], st("-1")),
        ("Str", vec![s("7")], st(" 7")),
        ("Str", vec![s("x")], Err(13)),
        ("Str", vec![Variant::Null], Err(94)),
        ("Val", vec![s("  12abc")], dbl(12.0)),
        ("Val", vec![s("&H1F")], dbl(31.0)),
        ("Val", vec![Variant::Null], Err(94)),
        ("CInt", vec![s("12")], i(12)),
        ("CInt", vec![Variant::Double(2.5)], i(2)),
        ("CInt", vec![Variant::Null], Err(94)),
        ("CInt", vec![n(40000)], Err(6)),
        ("CLng", vec![s("x")], Err(13)),
        ("CStr", vec![n(5)], st("5")),
        ("CBool", vec![n(2)], b(true)),
        ("CDbl", vec![s("1.5")], dbl(1.5)),
        ("CVar", vec![Variant::Null], NULL),
        (
            "CVErr",
            vec![n(5)],
            Ok(Variant::Error(vba_value::VbaError::new(5))),
        ),
        ("CVErr", vec![n(-1)], Err(5)),
        ("Error", vec![n(11)], st("Division by zero")),
        ("Error$", vec![n(0)], st("")),
        ("Int", vec![Variant::Double(-2.5)], dbl(-3.0)),
        ("Fix", vec![Variant::Double(-2.5)], dbl(-2.0)),
        ("Int", vec![n(7)], l(7)),
        ("Fix", vec![Variant::Null], NULL),
        ("Int", vec![s("2.7")], dbl(2.0)),
        (
            "Int",
            vec![Variant::Currency(vba_value::Currency::from_scaled(-25_000))],
            Ok(Variant::Currency(vba_value::Currency::from_scaled(-30_000))),
        ),
        (
            "Fix",
            vec![Variant::Currency(vba_value::Currency::from_scaled(-25_000))],
            Ok(Variant::Currency(vba_value::Currency::from_scaled(-20_000))),
        ),
    ]);
}

#[test]
fn math_functions() {
    run(vec![
        ("Abs", vec![Variant::Integer(-5)], i(5)),
        ("Abs", vec![Variant::Double(-2.5)], dbl(2.5)),
        ("Abs", vec![Variant::Integer(-32768)], Err(6)),
        ("Abs", vec![Variant::Null], NULL),
        ("Abs", vec![s("-3")], dbl(3.0)),
        ("Abs", vec![Variant::Empty], i(0)),
        ("Abs", vec![s("x")], Err(13)),
        ("Sgn", vec![Variant::Double(-0.1)], i(-1)),
        ("Sgn", vec![n(0)], i(0)),
        ("Sgn", vec![Variant::Double(9.0)], i(1)),
        ("Sgn", vec![Variant::Null], NULL),
        ("Sqr", vec![Variant::Double(9.0)], dbl(3.0)),
        ("Sqr", vec![Variant::Double(-1.0)], Err(5)),
        ("Log", vec![Variant::Double(0.0)], Err(5)),
        ("Log", vec![Variant::Double(1.0)], dbl(0.0)),
        ("Exp", vec![Variant::Double(0.0)], dbl(1.0)),
        ("Exp", vec![Variant::Double(710.0)], Err(6)),
        ("Sin", vec![Variant::Double(0.0)], dbl(0.0)),
        ("Cos", vec![Variant::Double(0.0)], dbl(1.0)),
        ("Tan", vec![Variant::Double(0.0)], dbl(0.0)),
        ("Atn", vec![Variant::Double(0.0)], dbl(0.0)),
        ("Sin", vec![Variant::Null], Err(94)),
        ("Round", vec![Variant::Double(2.5)], dbl(2.0)),
        ("Round", vec![Variant::Double(3.5)], dbl(4.0)),
        ("Round", vec![Variant::Double(-2.5)], dbl(-2.0)),
        (
            "Round",
            vec![Variant::Double(1234.5678), n(2)],
            dbl(1234.57),
        ),
        ("Round", vec![Variant::Double(0.125), n(2)], dbl(0.12)),
        ("Round", vec![Variant::Double(1.0), n(-1)], Err(5)),
        ("Round", vec![n(7), n(2)], l(7)),
    ]);
    let mut e = Env::new();
    let Ok(Variant::Double(pi)) = e.call("Atn", &[Variant::Double(1.0)]) else {
        panic!()
    };
    assert!((pi * 4.0 - std::f64::consts::PI).abs() < 1e-15);
}

#[test]
fn rnd_through_registry() {
    let mut e = Env::new();
    let first = e.call("Rnd", &[]).unwrap();
    let Variant::Single(f) = first else { panic!() };
    assert_eq!(format!("{f:.7}"), "0.7055475");
    assert_eq!(e.call("Rnd", &[n(0)]), Ok(first.clone()));
    assert_eq!(
        e.call("Rnd", &[Variant::Single(1.0)])
            .map(|v| matches!(v, Variant::Single(_))),
        Ok(true)
    );
    // Negative argument: same seed, same number.
    let a = e.call("Rnd", &[n(-1)]).unwrap();
    let b2 = e.call("Rnd", &[n(-1)]).unwrap();
    assert_eq!(a, b2);
    // Randomize with a number is deterministic.
    let mut e1 = Env::new();
    e1.call("Randomize", &[Variant::Double(42.0)]).unwrap();
    let x = e1.call("Rnd", &[]).unwrap();
    let mut e2 = Env::new();
    e2.call("Randomize", &[Variant::Double(42.0)]).unwrap();
    assert_eq!(e2.call("Rnd", &[]).unwrap(), x);
    // Randomize without argument uses the host's Timer (6:00 = 21600 s).
    let mut e3 = Env::new();
    e3.call_opt("Randomize", &[]).unwrap();
    let mut e4 = Env::new();
    e4.call("Randomize", &[Variant::Double(21600.0)]).unwrap();
    assert_eq!(e3.call("Rnd", &[]), e4.call("Rnd", &[]));
}

#[test]
fn datetime_functions() {
    let dt = d(2024, 3, 5, 14, 7, 9);
    run(vec![
        ("Year", vec![dt.clone()], i(2024)),
        ("Month", vec![dt.clone()], i(3)),
        ("Day", vec![dt.clone()], i(5)),
        ("Hour", vec![dt.clone()], i(14)),
        ("Minute", vec![dt.clone()], i(7)),
        ("Second", vec![dt.clone()], i(9)),
        ("Day", vec![s("3/5/2024")], i(5)),
        ("Day", vec![Variant::Null], NULL),
        ("Day", vec![s("nope")], Err(13)),
        ("Weekday", vec![dt.clone()], i(3)),
        ("Weekday", vec![dt.clone(), n(2)], i(2)),
        ("Weekday", vec![dt.clone(), n(9)], Err(5)),
        ("WeekdayName", vec![n(1)], st("Sunday")),
        ("WeekdayName", vec![n(1), Variant::Boolean(true)], st("Sun")),
        (
            "WeekdayName",
            vec![n(1), Variant::Boolean(true), n(2)],
            st("Mon"),
        ),
        ("WeekdayName", vec![n(8)], Err(5)),
        ("MonthName", vec![n(3)], st("March")),
        ("MonthName", vec![n(3), Variant::Boolean(true)], st("Mar")),
        ("MonthName", vec![n(13)], Err(5)),
        (
            "DateSerial",
            vec![n(2024), n(3), n(5)],
            Ok(d(2024, 3, 5, 0, 0, 0)),
        ),
        (
            "DateSerial",
            vec![n(2024), n(14), n(1)],
            Ok(d(2025, 2, 1, 0, 0, 0)),
        ),
        (
            "DateSerial",
            vec![n(2024), n(3), n(0)],
            Ok(d(2024, 2, 29, 0, 0, 0)),
        ),
        (
            "DateSerial",
            vec![n(50), n(1), n(1)],
            Ok(d(1950, 1, 1, 0, 0, 0)),
        ),
        (
            "DateSerial",
            vec![n(5), n(1), n(1)],
            Ok(d(2005, 1, 1, 0, 0, 0)),
        ),
        ("DateSerial", vec![n(40000), n(1), n(1)], Err(6)),
        (
            "TimeSerial",
            vec![n(14), n(7), n(9)],
            Ok(Variant::Date(
                vba_value::Date::from_serial((14.0 * 3600.0 + 7.0 * 60.0 + 9.0) / 86400.0).unwrap(),
            )),
        ),
        (
            "DateValue",
            vec![d(2024, 3, 5, 14, 7, 9)],
            Ok(d(2024, 3, 5, 0, 0, 0)),
        ),
        (
            "DateValue",
            vec![s("3/5/2024 2:07 PM")],
            Ok(d(2024, 3, 5, 0, 0, 0)),
        ),
        ("DateValue", vec![Variant::Null], NULL),
        (
            "DateAdd",
            vec![s("m"), n(1), s("1/31/1995")],
            Ok(d(1995, 2, 28, 0, 0, 0)),
        ),
        (
            "DateAdd",
            vec![s("YYYY"), n(1), d(2020, 2, 29, 0, 0, 0)],
            Ok(d(2021, 2, 28, 0, 0, 0)),
        ),
        (
            "DateAdd",
            vec![s("d"), Variant::Double(1.5), d(2020, 1, 1, 0, 0, 0)],
            Ok(d(2020, 1, 3, 0, 0, 0)),
        ),
        ("DateAdd", vec![s("zz"), n(1), dt.clone()], Err(5)),
        ("DateAdd", vec![s("d"), n(1), Variant::Null], NULL),
        (
            "DateDiff",
            vec![s("d"), d(2020, 1, 1, 0, 0, 0), d(2020, 12, 31, 0, 0, 0)],
            l(365),
        ),
        (
            "DateDiff",
            vec![s("yyyy"), d(2020, 12, 31, 0, 0, 0), d(2021, 1, 1, 0, 0, 0)],
            l(1),
        ),
        (
            "DateDiff",
            vec![s("ww"), d(2020, 1, 4, 0, 0, 0), d(2020, 1, 5, 0, 0, 0)],
            l(1),
        ),
        (
            "DateDiff",
            vec![
                s("ww"),
                d(2020, 1, 4, 0, 0, 0),
                d(2020, 1, 5, 0, 0, 0),
                n(2),
            ],
            l(0),
        ),
        ("DatePart", vec![s("q"), dt.clone()], l(1)),
        ("DatePart", vec![s("ww"), dt.clone()], l(10)),
        ("DatePart", vec![s("w"), dt.clone(), n(2)], l(2)),
        ("DatePart", vec![s("y"), dt.clone()], l(65)),
        (
            "DatePart",
            vec![s("ww"), d(2021, 1, 1, 0, 0, 0), n(2), n(2)],
            l(53),
        ),
        (
            "DatePart",
            vec![s("ww"), d(2021, 1, 1, 0, 0, 0), n(2), n(9)],
            Err(5),
        ),
    ]);
    let mut e = Env::new();
    assert_eq!(e.call("Now", &[]), Ok(d(2000, 1, 1, 6, 0, 0)));
    assert_eq!(e.call("Date", &[]), Ok(d(2000, 1, 1, 0, 0, 0)));
    assert_eq!(e.call("Date$", &[]), Ok(s("01-01-2000")));
    assert_eq!(e.call("Time$", &[]), Ok(s("06:00:00")));
    assert_eq!(e.call("Timer", &[]), Ok(Variant::Single(21600.0)));
    let Ok(Variant::Date(t)) = e.call("Time", &[]) else {
        panic!()
    };
    assert_eq!(t.serial(), 0.25);
    let Ok(Variant::Date(t)) = e.call("TimeValue", &[s("6:00 AM")]) else {
        panic!()
    };
    assert_eq!(t.serial(), 0.25);
}

#[test]
fn format_functions() {
    let dt = d(2024, 3, 5, 14, 7, 9);
    run(vec![
        ("Format", vec![Variant::Null, s("0")], NULL),
        ("Format$", vec![Variant::Null, s("0")], Err(94)),
        ("Format", vec![n(5)], st("5")),
        (
            "Format",
            vec![Variant::Double(1234.5), s("Standard")],
            st("1,234.50"),
        ),
        ("Format", vec![s("x"), s("")], st("x")),
        (
            "Format",
            vec![dt.clone(), s("yyyy-mm-dd")],
            st("2024-03-05"),
        ),
        ("Format", vec![dt.clone()], st("3/5/2024 2:07:09 PM")),
        ("Format", vec![dt.clone(), s("ww"), n(2), n(2)], st("10")),
        (
            "FormatNumber",
            vec![Variant::Double(1234.567), n(2)],
            st("1,234.57"),
        ),
        (
            "FormatNumber",
            vec![Variant::Double(-5.0), n(2)],
            st("-5.00"),
        ),
        (
            "FormatNumber",
            vec![Variant::Double(0.5), n(2), Variant::Boolean(false)],
            st(".50"),
        ),
        (
            "FormatNumber",
            vec![Variant::Double(-5.0), n(2), n(-2), Variant::Boolean(true)],
            st("(5.00)"),
        ),
        (
            "FormatNumber",
            vec![
                Variant::Double(1234.5),
                n(0),
                n(-2),
                n(-2),
                Variant::Boolean(false),
            ],
            st("1235"),
        ),
        ("FormatNumber", vec![Variant::Null], Err(94)),
        (
            "FormatCurrency",
            vec![Variant::Double(1234.5)],
            st("$1,234.50"),
        ),
        ("FormatCurrency", vec![Variant::Double(-5.0)], st("($5.00)")),
        (
            "FormatCurrency",
            vec![Variant::Double(-5.0), n(-1), n(-2), Variant::Boolean(false)],
            st("-$5.00"),
        ),
        ("FormatPercent", vec![Variant::Double(0.256)], st("25.60%")),
        (
            "FormatPercent",
            vec![Variant::Double(0.256), n(0)],
            st("26%"),
        ),
        (
            "FormatDateTime",
            vec![dt.clone()],
            st("3/5/2024 2:07:09 PM"),
        ),
        (
            "FormatDateTime",
            vec![dt.clone(), n(1)],
            st("Tuesday, March 05, 2024"),
        ),
        ("FormatDateTime", vec![dt.clone(), n(2)], st("3/5/2024")),
        ("FormatDateTime", vec![dt.clone(), n(3)], st("2:07:09 PM")),
        ("FormatDateTime", vec![dt.clone(), n(4)], st("14:07")),
        ("FormatDateTime", vec![dt.clone(), n(5)], Err(5)),
        ("FormatDateTime", vec![s("3/5/2024"), n(2)], st("3/5/2024")),
    ]);
}

#[test]
fn information_functions() {
    run(vec![
        ("IsNumeric", vec![s("12")], b(true)),
        ("IsNumeric", vec![s("1,234.5")], b(true)),
        ("IsNumeric", vec![s("abc")], b(false)),
        ("IsNumeric", vec![s("")], b(false)),
        ("IsNumeric", vec![Variant::Empty], b(true)),
        ("IsNumeric", vec![Variant::Null], b(false)),
        ("IsNumeric", vec![Variant::Boolean(true)], b(true)),
        ("IsNumeric", vec![d(2020, 1, 1, 0, 0, 0)], b(false)),
        ("IsDate", vec![s("1/1/2020")], b(true)),
        ("IsDate", vec![s("12")], b(false)),
        ("IsDate", vec![s("hello")], b(false)),
        ("IsDate", vec![d(2020, 1, 1, 0, 0, 0)], b(true)),
        ("IsDate", vec![n(5)], b(false)),
        ("IsNull", vec![Variant::Null], b(true)),
        ("IsNull", vec![Variant::Empty], b(false)),
        ("IsEmpty", vec![Variant::Empty], b(true)),
        ("IsEmpty", vec![s("")], b(false)),
        (
            "IsError",
            vec![Variant::Error(vba_value::VbaError::new(5))],
            b(true),
        ),
        ("IsError", vec![n(5)], b(false)),
        ("IsArray", vec![arr(&[n(1)])], b(true)),
        ("IsArray", vec![n(1)], b(false)),
        (
            "IsObject",
            vec![Variant::Object(vba_value::ObjectRef::Nothing)],
            b(true),
        ),
        ("IsObject", vec![n(1)], b(false)),
        ("TypeName", vec![n(1)], st("Long")),
        ("TypeName", vec![Variant::Integer(1)], st("Integer")),
        ("TypeName", vec![Variant::Double(1.0)], st("Double")),
        ("TypeName", vec![s("")], st("String")),
        ("TypeName", vec![Variant::Null], st("Null")),
        ("TypeName", vec![Variant::Empty], st("Empty")),
        ("TypeName", vec![Variant::Boolean(true)], st("Boolean")),
        ("TypeName", vec![d(2020, 1, 1, 0, 0, 0)], st("Date")),
        ("TypeName", vec![arr(&[n(1)])], st("Variant()")),
        (
            "TypeName",
            vec![Variant::Object(vba_value::ObjectRef::Nothing)],
            st("Nothing"),
        ),
        ("VarType", vec![n(1)], i(3)),
        ("VarType", vec![s("")], i(8)),
        ("VarType", vec![Variant::Null], i(1)),
        ("VarType", vec![arr(&[n(1)])], i(8204)),
        ("LBound", vec![arr(&[n(1), n(2)])], l(0)),
        ("UBound", vec![arr(&[n(1), n(2)])], l(1)),
        ("UBound", vec![arr(&[n(1), n(2)]), n(2)], Err(9)),
        ("UBound", vec![arr(&[n(1), n(2)]), n(0)], Err(9)),
        ("UBound", vec![n(1)], Err(13)),
    ]);
    let mut e = Env::new();
    assert!(matches!(
        e.call("IsMissing", &[n(1)]),
        Err(StdError::VmRequired(_))
    ));
    assert!(matches!(
        e.call(
            "TypeName",
            &[Variant::Object(vba_value::ObjectRef::Handle(
                vba_value::ObjectHandle(1)
            ))]
        ),
        Err(StdError::VmRequired(_))
    ));
    // Array honours Option Base.
    let mut ctx = vba_stdlib::Ctx::new(&e.host, &mut e.state);
    ctx.array_base = 1;
    let Ok(Variant::Array(a)) =
        vba_stdlib::call_function("Array", &mut ctx, &[Some(n(7)), Some(n(8))])
    else {
        panic!()
    };
    assert_eq!((a.lbound(1), a.ubound(1)), (Ok(1), Ok(2)));
}

#[test]
fn interaction_functions() {
    run(vec![
        ("Choose", vec![n(2), s("a"), s("b")], st("b")),
        ("Choose", vec![n(3), s("a"), s("b")], NULL),
        (
            "Choose",
            vec![Variant::Double(1.5), s("a"), s("b")],
            st("b"),
        ),
        ("IIf", vec![Variant::Boolean(true), n(1), n(2)], l(1)),
        ("IIf", vec![n(0), n(1), n(2)], l(2)),
        (
            "Switch",
            vec![Variant::Boolean(false), n(1), Variant::Boolean(true), n(2)],
            l(2),
        ),
        ("Switch", vec![Variant::Boolean(false), n(1)], NULL),
        ("Switch", vec![Variant::Boolean(false)], Err(5)),
        ("Environ", vec![s("home")], st("/home/x")),
        ("Environ$", vec![s("NOPE")], st("")),
        ("Environ", vec![n(1)], st("HOME=/home/x")),
        ("Environ", vec![n(2)], st("PATH=/bin")),
        ("Environ", vec![n(3)], st("")),
        ("Environ", vec![n(0)], Err(5)),
    ]);
}

#[test]
fn financial_through_registry() {
    let mut e = Env::new();
    let dbl = |v: Result<Variant, StdError>| match v {
        Ok(Variant::Double(x)) => x,
        other => panic!("{other:?}"),
    };
    let x = dbl(e.call("Pmt", &[Variant::Double(0.08 / 12.0), n(10), n(10000)]));
    assert!((x + 1037.03).abs() < 0.005);
    let v = arr(&[
        Variant::Double(-10000.0),
        Variant::Double(3000.0),
        Variant::Double(4200.0),
        Variant::Double(6800.0),
    ]);
    let x = dbl(e.call("NPV", &[Variant::Double(0.1), v]));
    assert!((x - 1188.44).abs() < 0.005);
    assert_eq!(e.call("SLN", &[n(30000), n(7500), n(0)]), Err(err(5)));
    assert_eq!(e.call("IRR", &[arr(&[n(1), n(2)])]), Err(err(5)));
    assert_eq!(e.call("IRR", &[n(1)]), Err(err(5)));
    let x = dbl(e.call("DDB", &[n(2400), n(300), n(10), Variant::Double(1.5)]));
    assert!((x - 2400.0 * 0.8f64.powf(0.5) * 0.2).abs() < 1e-9);
    let x = dbl(e.call("DDB", &[n(2400), n(300), n(10), n(1)]));
    assert_eq!(x, 480.0);
}

#[test]
fn registry_lookup_and_arity() {
    for name in [
        "Left",
        "left",
        "LEFT$",
        "Strings.Left",
        "VBA.Strings.Left",
        "vba.left",
        "VBA.strings.left$",
    ] {
        let r = lookup(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(r.info.name, "Left");
        assert_eq!(r.dollar, name.ends_with('$'));
    }
    assert!(lookup("Len$").is_none());
    assert!(lookup("Strings.Abs").is_none());
    assert!(lookup("Nope").is_none());
    let mid = lookup("Mid").unwrap().info;
    assert_eq!((mid.min_args(), mid.max_args()), (2, Some(3)));
    assert!(mid.params[2].optional && mid.dollar && mid.module == "Strings");
    let arr_fn = lookup("Array").unwrap().info;
    assert_eq!(arr_fn.max_args(), None);
    assert!(arr_fn.params[0].param_array);
    assert_eq!(lookup("IsMissing").unwrap().info.kind, Kind::Vm);
    assert_eq!(lookup("IIf").unwrap().info.kind, Kind::Eager);
    assert_eq!(lookup("Tab").unwrap().info.kind, Kind::PrintHelper);
    assert_eq!(lookup("Replace").unwrap().info.params[3].default, Some("1"));
    let mut e = Env::new();
    assert_eq!(e.call("Left", &[s("a")]), Err(err(450)));
    assert_eq!(e.call("Left", &[s("a"), n(1), n(2)]), Err(err(450)));
    assert_eq!(e.call("Nope", &[]), Err(err(35)));
    assert_eq!(e.call_opt("Left", &[Some(s("a")), None]), Err(err(449)));
    // Every registered function resolves under every spelling.
    for f in vba_stdlib::registry::functions() {
        assert!(lookup(f.name).is_some());
        assert!(lookup(&format!("VBA.{}.{}", f.module, f.name)).is_some());
        assert_eq!(
            lookup(&format!("{}$", f.name)).is_some(),
            f.dollar,
            "{}",
            f.name
        );
    }
}
