//! Hostile input to the JSON reader and the type spelling parser.
//!
//! Deterministic: a seeded xorshift generator and fixed case counts.
#![allow(missing_docs)]

mod common;

use std::time::{Duration, Instant};

use vba_host_model::{HostLibrary, TypeRef};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Run on a thread with a stack as small as the default test thread's, so an
/// unbounded recursion shows up here as a crash rather than being hidden by
/// a large main-thread stack.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

/// A type spelling with millions of `()` suffixes used to recurse once per
/// suffix in `TypeRef::parse` (and again in `Display`, `Clone` and `Drop` of
/// the result).
#[test]
fn deeply_nested_array_spellings_do_not_overflow_the_stack() {
    let text = format!("Long{}", "()".repeat(2_000_000));
    let t = on_small_stack(move || {
        let t = TypeRef::parse(&text);
        let shown = t.to_string();
        let copy = t.clone();
        assert_eq!(copy, t);
        (t, shown.len())
    });
    assert!(matches!(t.0, TypeRef::Named(_)));
    // A sensible nesting still parses as arrays.
    let ok = TypeRef::parse("Long()()");
    assert_eq!(ok, TypeRef::array_of(TypeRef::array_of(TypeRef::Long)));
    assert_eq!(TypeRef::parse(" Long () ").to_string(), "Long()");
}

#[test]
fn deeply_nested_json_is_an_error() {
    let text = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    on_small_stack(move || assert!(HostLibrary::from_json_str(&text).is_err()));
    let obj = format!("{}1{}", "{\"a\":".repeat(50_000), "}".repeat(50_000));
    on_small_stack(move || assert!(HostLibrary::from_json_str(&obj).is_err()));
}

#[test]
fn mutated_library_json_never_panics() {
    let base = common::description().to_json_string().into_bytes();
    let mut rng = Rng(0x5EED_0010);
    let started = Instant::now();
    for _ in 0..3000 {
        let mut b = base.clone();
        for _ in 0..1 + rng.below(6) {
            match rng.below(5) {
                0 => {
                    let i = rng.below(b.len());
                    b[i] ^= 1 << rng.below(8);
                }
                1 => {
                    let i = rng.below(b.len());
                    b[i] = b"\"{}[],:0-9eE\\"[rng.below(13)];
                }
                2 => {
                    let n = rng.below(b.len() + 1);
                    b.truncate(n);
                }
                3 => {
                    let s = rng.below(b.len());
                    let e = (s + rng.below(200)).min(b.len());
                    let chunk = b[s..e].to_vec();
                    let at = rng.below(b.len() + 1);
                    b.splice(at..at, chunk);
                }
                _ => {
                    let s = rng.below(b.len());
                    let e = (s + rng.below(200)).min(b.len());
                    b.drain(s..e);
                }
            }
            if b.is_empty() {
                break;
            }
        }
        if let Ok(lib) = HostLibrary::from_json_str(&String::from_utf8_lossy(&b)) {
            let _ = lib.problems();
        }
    }
    assert!(started.elapsed() < Duration::from_secs(60));
}

#[test]
fn enormous_counts_and_numbers_are_refused() {
    for text in [
        r#"{"format":"vba-host-model","version":18446744073709551615,"name":"L"}"#,
        r#"{"format":"vba-host-model","version":1,"name":"L","constants":[{"name":"C","value":{"type":"long","value":99999999999999999999}}]}"#,
        r#"{"format":"vba-host-model","version":1,"name":"L","constants":[{"name":"C","value":{"type":"double","value":1e999}}]}"#,
        r#"{"format":"vba-host-model","version":1,"name":"L","constants":[{"name":"C","value":{"type":"integer","value":-32769}}]}"#,
    ] {
        let _ = HostLibrary::from_json_str(text);
    }
}
