//! Hostile text through the standalone module reader (`.bas`, `.cls`,
//! `.frm`). Deterministic: seeded xorshift, fixed counts, no dependencies.
#![allow(missing_docs)]

use std::time::{Duration, Instant};

use vba_project::Module;

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

const PIECES: &[&str] = &[
    "VERSION 1.0 CLASS\r\n",
    "VERSION 5.00\r\n",
    "BEGIN\r\n",
    "Begin {C62A69F0-16DC-11CE-9E98-00AA00574A4F} UserForm1\r\n",
    "  Begin Frame F\r\n",
    "END\r\n",
    "End\r\n",
    "   MultiUse = -1  'True\r\n",
    "Attribute VB_Name = \"A\"\r\n",
    "attribute vb_name=\"\"\r\n",
    "Attribute VB_Name = \"é\u{feff}\"\r\n",
    "Attribute",
    "\u{feff}",
    "\n",
    "\r",
    "Sub X()\r\nEnd Sub\r\n",
    "é",
    "\0",
];

fn text(rng: &mut Rng, n: usize) -> String {
    (0..n).map(|_| PIECES[rng.below(PIECES.len())]).collect()
}

#[test]
fn random_export_text_never_panics() {
    let mut rng = Rng(0x5EED_0080);
    let started = Instant::now();
    for _ in 0..30_000 {
        let n = rng.below(30);
        let body = text(&mut rng, n);
        for name in ["a.bas", "a.cls", "a.frm", ".cls", "dir/b.FRM", "x", "é.cls", ""] {
            if let Ok(m) = Module::from_source_file(name, &body) {
                // Writing it back out never panics either (a byte-order
                // mark repeated at the start is not normalized, so the
                // result is not required to read back identically).
                let _ = Module::from_source_file(name, &m.to_source_file());
            }
        }
    }
    assert!(started.elapsed() < Duration::from_secs(60));
}

/// Deeply nested and very long headers are linear work, not recursion.
#[test]
fn nested_and_long_headers_are_linear() {
    let nested = format!(
        "VERSION 5.00\r\n{}{}Attribute VB_Name = \"N\"\r\n",
        "Begin X\r\n".repeat(200_000),
        "End\r\n".repeat(200_000)
    );
    let started = Instant::now();
    let m = Module::from_source_file("N.frm", &nested).expect("balanced header");
    assert_eq!(m.name, "N");
    let unbalanced = format!("Begin X\r\n{}", "Begin Y\r\n".repeat(200_000));
    assert!(Module::from_source_file("U.cls", &unbalanced).is_err());
    let long_line = format!("Attribute VB_Name = \"{}\"\r\n", "x".repeat(5_000_000));
    assert!(Module::from_source_file("L.bas", &long_line).is_ok());
    assert!(started.elapsed() < Duration::from_secs(20));
}
