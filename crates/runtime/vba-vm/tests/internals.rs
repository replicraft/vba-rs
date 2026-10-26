//! Behaviour that is about the interpreter's own bookkeeping: reclamation,
//! unsupported features and late-bound property assignment.

mod common;

use common::*;
use vba_host::{HeadlessHost, ResourceLimits};
use vba_vm::VmError;

#[test]
fn user_defined_type_values_are_reclaimed() {
    let o = run_limited(
        "Private Type P\nX As Long\nS As String\nEnd Type\nFunction Make(n As Long) As P\nDim r As P\nr.X = n\nMake = r\nEnd Function\nSub Main()\nDim i As Long, a As P, total As Long\nFor i = 1 To 20000\na = Make(i)\ntotal = total + a.X Mod 7\nNext\nDebug.Print total\nEnd Sub\n",
        ResourceLimits {
            max_steps: Some(5_000_000),
            max_memory_bytes: Some(1 << 20),
            ..ResourceLimits::default()
        },
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert!(
        o.vm.live_udts() < 10_000,
        "live user-defined values: {}",
        o.vm.live_udts()
    );
}

#[test]
fn objects_are_not_leaked_by_loops() {
    let o = run_with(
        &[
            module("Item", vba_project::ModuleKind::Class, "Public N As Long\n"),
            module(
                "M",
                vba_project::ModuleKind::Standard,
                "Sub Main()\nDim i As Long, o As Item\nFor i = 1 To 500\nSet o = New Item\no.N = i\nNext\nEnd Sub\n",
            ),
        ],
        "Main",
        vec![],
        std::rc::Rc::new(HeadlessHost::new()),
        Box::new(vba_vm::NoServices),
    );
    assert!(o.result.is_ok());
    assert_eq!(o.vm.live_objects(), 0);
}

#[test]
fn address_of_is_an_explicit_unsupported_error() {
    let o = run(
        "Private Declare PtrSafe Function Cb Lib \"k\" (ByVal f As LongPtr) As Long\nFunction Target() As Long\nEnd Function\nSub Main()\nOn Error Resume Next\nDim n As Long\nn = Cb(AddressOf Target)\nDebug.Print \"trapped\"\nEnd Sub\n",
    );
    assert!(
        matches!(o.result, Err(VmError::Unsupported(ref m)) if m.contains("AddressOf")),
        "{:?}",
        o.result
    );
    assert_eq!(o.out, "", "unsupported features are not trappable");
}

#[test]
fn late_bound_property_let_and_set_on_vm_objects() {
    check_modules(&[mcase(
        "late_property_assignment",
        &[
            (
                "Counter",
                true,
                "Private n As Long\nPublic Property Get Value() As Long\nValue = n\nEnd Property\nPublic Property Let Value(v As Long)\nn = v\nEnd Property\n",
            ),
            (
                "Holder",
                true,
                "Private c As Object\nPublic Property Set Inner(v As Object)\nSet c = v\nEnd Property\nPublic Property Get Inner() As Object\nSet Inner = c\nEnd Property\n",
            ),
            (
                "M",
                false,
                "Sub Main()\nDim h As Object, k As Object\nSet h = New Holder\nSet k = New Counter\nk.Value = 9\nSet h.Inner = k\nDebug.Print h.Inner.Value\nh.Inner.Value = h.Inner.Value + 1\nDebug.Print k.Value\nEnd Sub\n",
            ),
        ],
        " 9 \n 10 \n",
    )]);
}

#[test]
fn steps_scale_linearly_with_work() {
    // A million-step budget runs a loop of a few hundred thousand
    // iterations; the same loop with a 10x smaller budget stops.
    let src = "Sub Main()\nDim i As Long, s As Long\nFor i = 1 To 50000\ns = s + i Mod 3\nNext\nDebug.Print s\nEnd Sub\n";
    let big = run_limited(
        src,
        ResourceLimits {
            max_steps: Some(1_000_000),
            ..ResourceLimits::default()
        },
    );
    assert!(big.result.is_ok(), "{:?}", big.result);
    let small = run_limited(
        src,
        ResourceLimits {
            max_steps: Some(100_000),
            ..ResourceLimits::default()
        },
    );
    assert!(matches!(small.result, Err(VmError::Limit(_))));
}
