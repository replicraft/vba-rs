//! Deep recursion must end in a limit error, never in a native stack
//! overflow, and long ownership chains must be released without recursion.

mod common;

use common::*;
use vba_host::ResourceLimits;
use vba_project::ModuleKind;
use vba_vm::{LimitKind, VmError};

fn deep(src_main: &str, class: &str, depth: u32) -> Outcome {
    run_with_limits(
        &[
            module("Rec", ModuleKind::Class, class),
            module("M", ModuleKind::Standard, src_main),
        ],
        ResourceLimits {
            max_steps: Some(50_000_000),
            max_call_depth: depth,
            ..ResourceLimits::default()
        },
    )
}

#[test]
fn late_bound_recursion_is_bounded_by_the_call_depth() {
    let o = deep(
        "Sub Main()\nDim o As Object\nSet o = New Rec\no.Down 100000\nEnd Sub\n",
        "Public Sub Down(n As Long)\nDim o As Object\nSet o = Me\nIf n > 0 Then o.Down n - 1\nEnd Sub\n",
        3000,
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::CallDepth)));
}

#[test]
fn late_bound_calls_do_not_nest_the_native_stack() {
    // Deeper than the native nesting limit, within the call depth.
    let o = deep(
        "Sub Main()\nDim o As Object\nSet o = New Rec\nDebug.Print o.Down(500)\nEnd Sub\n",
        "Public Function Down(n As Long) As Long\nDim o As Object\nSet o = Me\nIf n > 0 Then Down = o.Down(n - 1) + 1\nEnd Function\n",
        1000,
    );
    assert_eq!(o.out, " 500 \n", "{:?}", o.result);
}

#[test]
fn long_ownership_chains_are_released_iteratively() {
    let o = deep(
        "Sub Main()\nDim head As Rec, n As Rec, i As Long\nFor i = 1 To 20000\nSet n = New Rec\nSet n.Nxt = head\nSet head = n\nNext\nSet n = Nothing\nSet head = Nothing\nDebug.Print \"released\"\nEnd Sub\n",
        "Public Nxt As Rec\nPrivate Sub Class_Terminate()\nEnd Sub\n",
        1000,
    );
    assert_eq!(o.out, "released\n", "{:?}", o.result);
    assert_eq!(o.vm.live_objects(), 0);
}

#[test]
fn release_order_is_depth_first_in_declaration_order() {
    let o = deep(
        "Sub Main()\nDim r As New Rec\nr.Tag = \"root\"\nSet r.A = New Rec\nr.A.Tag = \"a\"\nSet r.A.A = New Rec\nr.A.A.Tag = \"a.a\"\nSet r.B = New Rec\nr.B.Tag = \"b\"\nSet r = Nothing\nEnd Sub\n",
        "Public Tag As String\nPublic A As Rec\nPublic B As Rec\nPrivate Sub Class_Terminate()\nDebug.Print \"term \" & Tag\nEnd Sub\n",
        1000,
    );
    assert_eq!(
        o.out, "term root\nterm a\nterm a.a\nterm b\n",
        "{:?}",
        o.result
    );
}

#[test]
fn recursion_through_class_initialize_is_bounded() {
    let o = deep(
        "Sub Main()\nDim r As New Rec\nr.Tag = \"x\"\nEnd Sub\n",
        "Public Tag As String\nPublic Child As Rec\nPrivate Sub Class_Initialize()\nSet Child = New Rec\nEnd Sub\n",
        100_000,
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::CallDepth)));
}

#[test]
fn recursion_through_events_is_bounded() {
    let o = run_with_limits(
        &[
            module(
                "Src",
                ModuleKind::Class,
                "Public Event Ping(ByVal n As Long)\nPublic Sub Fire(n As Long)\nRaiseEvent Ping(n)\nEnd Sub\n",
            ),
            module(
                "Sink",
                ModuleKind::Class,
                "Public WithEvents S As Src\nPrivate Sub S_Ping(ByVal n As Long)\nS.Fire n + 1\nEnd Sub\n",
            ),
            module(
                "M",
                ModuleKind::Standard,
                "Sub Main()\nDim s As New Src, k As New Sink\nSet k.S = s\ns.Fire 0\nEnd Sub\n",
            ),
        ],
        ResourceLimits {
            max_steps: Some(50_000_000),
            max_call_depth: 100_000,
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::CallDepth)));
}

#[test]
fn ordinary_deep_recursion_works_within_the_limit() {
    let o = deep(
        "Function Down(n As Long) As Long\nIf n > 0 Then Down = Down(n - 1) + 1\nEnd Function\nSub Main()\nDebug.Print Down(900)\nEnd Sub\n",
        "",
        1000,
    );
    assert_eq!(o.out, " 900 \n", "{:?}", o.result);
}

#[test]
fn long_chains_of_collections_are_released_iteratively() {
    // Each collection holds the previous one; releasing the last must not
    // recurse once per link on the native stack.
    let o = deep(
        "Sub Main()\nDim c As Collection, d As Collection, i As Long\nSet c = New Collection\nFor i = 1 To 100000\nSet d = New Collection\nd.Add c\nSet c = d\nNext\nSet c = Nothing\nSet d = Nothing\nDebug.Print \"released\"\nEnd Sub\n",
        "Public X As Long\n",
        1000,
    );
    assert_eq!(o.out, "released\n", "{:?}", o.result);
    assert_eq!(o.vm.live_objects(), 0);
}

fn type_chain(n: usize) -> String {
    let mut s = String::new();
    for i in 0..n {
        s.push_str(&format!("Private Type T{i}\n x As T{}\nEnd Type\n", i + 1));
    }
    s.push_str(&format!("Private Type T{n}\n y As Long\nEnd Type\n"));
    s.push_str("Sub Main()\nDim a As T0\nDebug.Print \"allocated\"\nEnd Sub\n");
    s
}

#[test]
fn user_defined_types_nested_absurdly_deep_are_a_limit_not_a_stack_overflow() {
    let o = run_with_limits(
        &[module("M", ModuleKind::Standard, &type_chain(100))],
        ResourceLimits::default(),
    );
    assert_eq!(o.out, "allocated\n", "{:?}", o.result);
    let o = run_with_limits(
        &[module("M", ModuleKind::Standard, &type_chain(1000))],
        ResourceLimits::default(),
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::CallDepth)));
}
