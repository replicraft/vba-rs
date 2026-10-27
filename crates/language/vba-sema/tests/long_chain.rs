//! An operator chain over the parser's limit is an error, not a crash. The
//! tree that remains is 512 operators deep, and analysing it recurses that
//! deep, which needs more than the 2 MiB stack of a test thread in a debug
//! build: the documented advice is to analyse untrusted source on a thread
//! with a large stack, and this test does.

mod common;

use common::*;

#[test]
fn a_chain_over_the_limit_is_an_error_on_a_large_stack() {
    let worker = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            for n in [400usize, 600, 5_000] {
                let expr = vec!["1"; n].join(" + ");
                let a = analyze(&[bas(
                    "M",
                    &format!("Function F() As Long\nF = {expr}\nEnd Function\n"),
                )]);
                assert_eq!(a.has_errors(), n > 512, "n={n}");
            }
        })
        .unwrap();
    worker.join().unwrap();
}
