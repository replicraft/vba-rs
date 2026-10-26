//! String and array builders refuse an oversized result before allocating it,
//! as the trappable error 7, instead of materialising it and failing after.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{HeadlessHost, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig};

fn out(body: &str) -> String {
    let src = format!("Sub Main()\nOn Error Resume Next\n{body}End Sub\n");
    let ir = compile(&[module("M", ModuleKind::Standard, &src)]);
    let sink = BufferSink::new();
    let host = Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(1_000_000),
        ..ResourceLimits::default()
    }));
    let mut vm = Vm::new(
        ir,
        host,
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig {
            max_string_units: 1000,
            max_array_elements: 10,
            ..VmConfig::default()
        },
    )
    .expect("vm");
    let main = vm.find_procedure("Main").expect("Main");
    vm.run_procedure(main, vec![]).expect("trappable");
    sink.contents()
}

#[test]
fn builders_fail_with_error_7_before_building_an_oversized_string() {
    let body = "\
Dim s As String, p() As String, a(1 To 3) As String, n As Long
s = String$(1001, \"a\")
Debug.Print \"string$ \" & Err.Number
Err.Clear
s = Space$(1001)
Debug.Print \"space$ \" & Err.Number
Err.Clear
s = Replace(String$(900, \"a\"), \"a\", \"bb\")
Debug.Print \"replace \" & Err.Number
Err.Clear
a(1) = String$(600, \"a\"): a(2) = String$(600, \"b\"): a(3) = \"\"
s = Join(a, \"\")
Debug.Print \"join \" & Err.Number
Err.Clear
p = Split(String$(20, \",\"), \",\")
Debug.Print \"split \" & Err.Number
Err.Clear
s = Replace(String$(900, \"a\"), \"a\", \"b\")
Debug.Print \"replace ok \" & Len(s)
s = String$(1000, \"a\")
Debug.Print \"string$ ok \" & Len(s)
";
    assert_eq!(
        out(body),
        "string$ 7\nspace$ 7\nreplace 7\njoin 7\nsplit 7\nreplace ok 900\nstring$ ok 1000\n"
    );
}
