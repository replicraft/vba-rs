//! Lowering and verification cost grows linearly with the size of a
//! procedure, so hostile input with tens of thousands of labels cannot keep
//! the compiler busy for minutes.

mod common;

use std::time::{Duration, Instant};

use common::*;
use vba_project::ModuleKind;

fn many_labels(n: usize) -> String {
    let mut s = String::from("Public Sub Main()\nDim i As Long\n");
    for k in 0..n {
        s.push_str(&format!("If i = {k} Then GoTo L{k}\n"));
    }
    s.push_str("GoTo Done\n");
    for k in 0..n {
        s.push_str(&format!("L{k}:\ni = i + 1\n"));
    }
    s.push_str("Done:\nEnd Sub\n");
    s
}

#[test]
fn tens_of_thousands_of_labels_lower_and_verify_in_linear_time() {
    let src = many_labels(30_000);
    let start = Instant::now();
    let ir = lower_ok(&[module("M", ModuleKind::Standard, &src)]);
    let took = start.elapsed();
    assert!(!ir.functions.is_empty());
    assert!(took < Duration::from_secs(15), "took {took:?}");
}

#[test]
fn doubling_the_labels_about_doubles_the_time() {
    let time = |n: usize| {
        let src = many_labels(n);
        let start = Instant::now();
        let _ = lower_ok(&[module("M", ModuleKind::Standard, &src)]);
        start.elapsed()
    };
    let small = time(8_000).max(Duration::from_millis(5));
    let big = time(32_000);
    // Four times the input: a quadratic algorithm needs 16 times the time.
    assert!(big < small * 10, "8000: {small:?}, 32000: {big:?}");
}
