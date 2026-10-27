//! `ObjPtr`, `StrPtr` and `VarPtr`: opaque identity tokens, never addresses.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::HeadlessHost;
use vba_project::ModuleKind;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig};

fn tokens_with(salt: u64) -> String {
    let src = "Sub Main()\nDim a As New Collection\nDim b As New Collection\n\
               Debug.Print ObjPtr(a); ObjPtr(b); StrPtr(\"abc\")\nEnd Sub\n";
    let ir = compile(&[module("M", ModuleKind::Standard, src)]);
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        ir,
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig {
            identity_salt: salt,
            ..VmConfig::default()
        },
    )
    .expect("vm");
    let p = vm.find_procedure("Main").expect("Main");
    vm.run_procedure(p, vec![]).expect("run");
    sink.contents()
}

#[test]
fn object_identity_is_stable_and_distinct() {
    let out = output(
        "Sub Main()\n\
         Dim a As New Collection\nDim b As New Collection\nDim c As Object\nDim n As Object\n\
         Set c = a\n\
         Debug.Print ObjPtr(a) = ObjPtr(c); ObjPtr(a) = ObjPtr(b); ObjPtr(a) <> 0; ObjPtr(n)\n\
         End Sub\n",
    );
    assert_eq!(out, "TrueFalseTrue 0 \n");
}

#[test]
fn many_objects_get_distinct_tokens() {
    let out = output(
        "Sub Main()\nDim seen As New Collection\nDim i As Long, o As Object\n\
         On Error Resume Next\n\
         For i = 1 To 300\nSet o = New Collection\nseen.Add i, CStr(ObjPtr(o))\nNext\n\
         Debug.Print seen.Count; Err.Number\nEnd Sub\n",
    );
    assert_eq!(out, " 300  0 \n");
}

#[test]
fn tokens_are_odd_nonzero_and_not_small_handles() {
    // The low bit is a fixed tag, so a token is never a plausible aligned
    // address, and the payload is mixed with the salt.
    let out = output(
        "Sub Main()\nDim a As New Collection\nDim p As Variant\np = ObjPtr(a)\n\
         Debug.Print (p Mod 2); p > 4096\nEnd Sub\n",
    );
    assert_eq!(out, " 1 True\n");
}

#[test]
fn string_tokens() {
    let out = output(
        "Sub Main()\nDim s As String, t As String\ns = \"hello\"\nt = \"hello\"\n\
         Debug.Print StrPtr(vbNullString); StrPtr(\"\"); StrPtr(s) <> 0; \
         StrPtr(s) = StrPtr(t); StrPtr(s) <> StrPtr(\"world\")\nEnd Sub\n",
    );
    assert_eq!(out, " 0  0 TrueTrueTrue\n");
}

#[test]
fn variable_tokens_follow_the_variable() {
    let out = output(
        "Dim g As Long\n\
         Sub Probe(ByRef x As Long, ByVal want As Variant)\nDebug.Print VarPtr(x) = want\nEnd Sub\n\
         Sub Main()\nDim a As Long, b As Long, arr(1 To 3) As Long\n\
         Debug.Print VarPtr(a) = VarPtr(a); VarPtr(a) <> VarPtr(b); VarPtr(g) <> VarPtr(a); \
         VarPtr(arr(1)) <> VarPtr(arr(2)); VarPtr(arr(2)) = VarPtr(arr(2))\n\
         Probe a, VarPtr(a)\nProbe g, VarPtr(g)\nProbe arr(3), VarPtr(arr(3))\nEnd Sub\n",
    );
    assert_eq!(out, "TrueTrueTrueTrueTrue\nTrue\nTrue\nTrue\n");
}

#[test]
fn non_object_argument_is_a_type_mismatch() {
    let o = run(
        "Sub Main()\nDim v As Variant\nv = 5\nOn Error Resume Next\nDebug.Print ObjPtr(v)\n\
         Debug.Print Err.Number\nEnd Sub\n",
    );
    assert_eq!(o.out, " 13 \n");
}

#[test]
fn the_salt_changes_the_values_and_the_default_is_reproducible() {
    let a = tokens_with(1);
    let b = tokens_with(2);
    assert_ne!(a, b);
    assert_eq!(a, tokens_with(1));
    assert_eq!(
        tokens_with(VmConfig::default().identity_salt),
        tokens_with(VmConfig::default().identity_salt)
    );
}
