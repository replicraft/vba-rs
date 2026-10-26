//! Smoke tests of the interpreter.

mod common;

use common::*;

#[test]
fn hello() {
    assert_eq!(
        output("Sub Main()\nDebug.Print \"hi\"; 1 + 2\nEnd Sub\n"),
        "hi 3 \n"
    );
}

#[test]
fn tour() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/t1.bas"
    ))
    .unwrap();
    let o = run(&src);
    println!("{}", o.out);
    println!("{:?}", o.result);
}
