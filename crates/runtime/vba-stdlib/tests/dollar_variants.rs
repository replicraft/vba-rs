//! Every `$` and `B` variant of the documented function inventory is callable
//! and behaves like its base function (a `$` form returns a plain `String`).

mod common;
use common::*;
use vba_stdlib::lookup;
use vba_value::Variant;

/// Functions documented with a `$` form that the registry implements.
const DOLLAR: &[&str] = &[
    "Left", "LeftB", "Mid", "MidB", "Right", "RightB", "Trim", "LTrim", "RTrim", "UCase", "LCase",
    "Chr", "ChrB", "ChrW", "Space", "String", "Str", "Hex", "Oct", "Format", "Error", "Environ",
    "Date", "Time",
];

/// The `B` (byte) variants of functions that exist in the registry.
const BYTE: &[(&str, &str)] = &[
    ("Left", "LeftB"),
    ("Mid", "MidB"),
    ("Right", "RightB"),
    ("Chr", "ChrB"),
    ("Asc", "AscB"),
    ("Len", "LenB"),
    ("InStr", "InStrB"),
];

fn n(x: i32) -> Variant {
    Variant::Long(x)
}

fn cases() -> Vec<(&'static str, Vec<Variant>)> {
    let t = || s("  Hello  ");
    vec![
        ("Left", vec![s("hello"), n(2)]),
        ("LeftB", vec![s("hello"), n(4)]),
        ("Mid", vec![s("hello"), n(2), n(3)]),
        ("Mid", vec![s("hello"), n(2)]),
        ("MidB", vec![s("hello"), n(3), n(4)]),
        ("Right", vec![s("hello"), n(2)]),
        ("RightB", vec![s("hello"), n(4)]),
        ("Trim", vec![t()]),
        ("LTrim", vec![t()]),
        ("RTrim", vec![t()]),
        ("UCase", vec![s("abc")]),
        ("LCase", vec![s("ABC")]),
        ("Chr", vec![n(65)]),
        ("ChrB", vec![n(65)]),
        ("ChrW", vec![n(955)]),
        ("Space", vec![n(3)]),
        ("String", vec![n(3), s("xy")]),
        ("Str", vec![n(42)]),
        ("Hex", vec![n(255)]),
        ("Oct", vec![n(8)]),
        ("Format", vec![n(5), s("000")]),
        ("Error", vec![n(5)]),
        ("Environ", vec![s("HOME")]),
        ("Date", vec![]),
        ("Time", vec![]),
    ]
}

#[test]
fn the_registry_has_exactly_the_documented_dollar_forms() {
    for name in DOLLAR {
        let r = lookup(&format!("{name}$")).unwrap_or_else(|| panic!("{name}$ is missing"));
        assert!(
            r.dollar && r.info.name.eq_ignore_ascii_case(name),
            "{name}$"
        );
    }
    for f in vba_stdlib::registry::functions() {
        if f.dollar {
            assert!(
                DOLLAR.contains(&f.name),
                "{} has a $ form that is not in the documented list",
                f.name
            );
        }
    }
}

#[test]
fn a_dollar_form_behaves_like_its_base_function() {
    let mut e = Env::new();
    for (name, args) in cases() {
        let base = e.call(name, &args);
        let dollar = e.call(&format!("{name}$"), &args);
        if base.is_err() {
            // `ChrB` is unsupported (a one-byte string has no UTF-16 form): both
            // spellings must fail the same way.
            assert_eq!(base, dollar, "{name}{args:?}");
            assert_eq!(name, "ChrB", "{name} fails: {base:?}");
            continue;
        }
        let (Ok(b), Ok(d)) = (&base, &dollar) else {
            panic!("{name}({args:?}): {base:?} / {dollar:?}");
        };
        assert!(matches!(d, Variant::String(_)), "{name}$ returns {d:?}");
        // Same text; the base function may return a Variant of another subtype
        // only where its documentation says so (`Date`, `Time`).
        if !matches!(b, Variant::Date(_)) {
            assert_eq!(b, d, "{name}{args:?}");
        }
    }
}

#[test]
fn a_dollar_form_rejects_null_where_the_base_function_returns_it() {
    let mut e = Env::new();
    for name in ["Left", "Mid", "Right", "Trim", "UCase", "LCase"] {
        let args = if name == "Left" || name == "Right" {
            vec![Variant::Null, n(1)]
        } else if name == "Mid" {
            vec![Variant::Null, n(1), n(1)]
        } else {
            vec![Variant::Null]
        };
        assert_eq!(e.call(name, &args), Ok(Variant::Null), "{name}");
        assert_eq!(e.call(&format!("{name}$"), &args), Err(err(94)), "{name}$");
    }
}

#[test]
fn the_byte_variants_exist_and_agree_on_ascii_text() {
    let mut e = Env::new();
    for (base, byte) in BYTE {
        assert!(lookup(base).is_some() && lookup(byte).is_some(), "{byte}");
    }
    // On single-byte text the B form of a character function counts bytes.
    assert_eq!(e.call("LeftB", &[s("abc"), n(2)]), Ok(s("a")));
    assert_eq!(e.call("AscB", &[s("A")]), Ok(Variant::Integer(65)));
    assert_eq!(e.call("LenB", &[s("abc")]), Ok(n(6)));
}
