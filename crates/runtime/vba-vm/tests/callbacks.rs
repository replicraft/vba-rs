//! `AddressOf` tokens and the callbacks `Declare` shims make into VBA.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{Capability, HeadlessHost, Policy, ResourceLimits};
use vba_profile::Target;
use vba_project::ModuleKind;
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, LimitKind, NoServices, Vm, VmConfig, VmError};
use vba_winapi::{CallContext, Identity, ManualClock, Registry, Window};

const DECLS: &str = "\
Private Declare PtrSafe Function EnumWindows Lib \"user32\" (ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long
Private Declare PtrSafe Function EnumChildWindows Lib \"user32\" (ByVal hParent As LongPtr, ByVal lpEnumFunc As LongPtr, ByVal lParam As LongPtr) As Long
Private Declare PtrSafe Function SetTimer Lib \"user32\" (ByVal hwnd As LongPtr, ByVal id As LongPtr, ByVal ms As Long, ByVal proc As LongPtr) As LongPtr
Private Declare PtrSafe Function KillTimer Lib \"user32\" (ByVal hwnd As LongPtr, ByVal id As LongPtr) As Long
Private Declare PtrSafe Sub Sleep Lib \"kernel32\" (ByVal ms As Long)
Private Declare PtrSafe Sub qsort Lib \"msvcrt\" (ByRef base() As Long, ByVal n As LongPtr, ByVal size As LongPtr, ByVal cmp As LongPtr)
Private Declare PtrSafe Sub CopyMemory Lib \"kernel32\" Alias \"RtlMoveMemory\" (ByRef dst As Long, ByVal src As LongPtr, ByVal n As LongPtr)
";

fn windows() -> Vec<Window> {
    [(0x100, 0), (0x110, 0x100), (0x200, 0)]
        .into_iter()
        .map(|(hwnd, parent)| Window {
            hwnd,
            parent,
            thread: 1,
        })
        .collect()
}

fn granted() -> Policy {
    Policy::deny_all().grant(Capability::NativeDeclare)
}

fn limits() -> ResourceLimits {
    ResourceLimits::default().with_max_steps(Some(2_000_000))
}

fn build(src: &str, policy: Policy, limits: ResourceLimits) -> (Vm, BufferSink) {
    let full = format!("{DECLS}{src}");
    let ir = compile(&[module("M", ModuleKind::Standard, &full)]);
    let sink = BufferSink::new();
    let host = Rc::new(HeadlessHost::new().with_policy(policy).with_limits(limits));
    let mut vm = Vm::new(
        ir,
        host,
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    let mut ctx = CallContext::new(
        Target::WIN64,
        Box::new(ManualClock::at(0)),
        Identity::new("user", "BOX").unwrap(),
    );
    for w in windows() {
        assert!(ctx.windows.add(w));
    }
    vm.set_native_bridge(Registry::with_native_shims(), ctx);
    (vm, sink)
}

fn run_main(src: &str) -> (Result<(), VmError>, String, Vm) {
    let (mut vm, sink) = build(src, granted(), limits());
    let main = vm.find_procedure("Main").expect("Main");
    let r = vm.run_procedure(main, vec![]).map(|_| ());
    (r, sink.contents(), vm)
}

fn runtime_number(r: &Result<(), VmError>) -> Option<i32> {
    match r {
        Err(VmError::Runtime(e)) => Some(e.error.number),
        _ => None,
    }
}

#[test]
fn enum_windows_calls_the_procedure_with_lparam() {
    let (r, out, _) = run_main(
        "Dim total As Long
Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    total = total + 1
    Debug.Print CStr(hwnd) & \":\" & CStr(lParam)
    Visit = 1
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 42)
    Debug.Print total
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, "256:42\n512:42\n 1 \n 2 \n");
}

#[test]
fn a_false_result_stops_the_enumeration() {
    let (r, out, _) = run_main(
        "Dim seen As Long
Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    seen = seen + 1
    Visit = 0
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0) & \"/\" & seen
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, "0/1\n");
}

#[test]
fn the_token_is_not_an_address_and_is_stable() {
    let (r, out, _) = run_main(
        "Public Function A() As Long
End Function
Public Function B() As Long
End Function
Public Sub Main()
    Dim p As LongPtr, q As LongPtr
    p = AddressOf A
    q = AddressOf B
    Debug.Print (p = AddressOf A); (p <> q); (p <> 0); (p > 0); (p < 2147483648#)
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, "TrueTrueTrueTrueTrue\n");
}

#[test]
fn a_callback_needs_the_grant_of_the_declare() {
    let (mut vm, sink) = build(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"ran\"
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0)
End Sub
",
        Policy::deny_all(),
        limits(),
    );
    let main = vm.find_procedure("Main").unwrap();
    let r = vm.run_procedure(main, vec![]).map(|_| ());
    assert_eq!(runtime_number(&r), Some(70), "{r:?}");
    assert_eq!(sink.contents(), "");
}

#[test]
fn a_scoped_grant_that_misses_the_declare_denies_the_callback() {
    let (mut vm, sink) = build(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"ran\"
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0)
End Sub
",
        Policy::deny_all().grant_prefixes(Capability::NativeDeclare, ["kernel32"]),
        limits(),
    );
    let main = vm.find_procedure("Main").unwrap();
    let r = vm.run_procedure(main, vec![]).map(|_| ());
    assert_eq!(runtime_number(&r), Some(70), "{r:?}");
    assert_eq!(sink.contents(), "");
}

#[test]
fn an_unhandled_error_in_a_callback_surfaces_at_the_declare_and_is_trappable() {
    let (r, out, _) = run_main(
        "Dim n As Long
Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    n = n + 1
    Err.Raise 1234, \"cb\", \"boom\"
End Function
Public Sub Main()
    On Error Resume Next
    Dim r As Long
    Err.Clear
    r = EnumWindows(AddressOf Visit, 0)
    Debug.Print Err.Number; Err.Description; n
    On Error GoTo 0
    Debug.Print \"after\"
End Sub
",
    );
    r.unwrap();
    // The enumeration stopped at the first window.
    assert_eq!(out, " 1234 boom 1 \nafter\n");
}

#[test]
fn an_unhandled_callback_error_without_a_handler_ends_the_run_with_that_error() {
    let (r, out, _) = run_main(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Err.Raise 1234, \"cb\", \"boom\"
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0)
    Debug.Print \"not reached\"
End Sub
",
    );
    assert_eq!(runtime_number(&r), Some(1234), "{r:?}");
    assert_eq!(out, "");
}

#[test]
fn a_callback_handles_its_own_error() {
    let (r, out, _) = run_main(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    On Error Resume Next
    Dim x As Long
    x = 1 / 0
    Visit = 1
End Function
Public Sub Main()
    Debug.Print EnumWindows(AddressOf Visit, 0)
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, " 1 \n");
}

#[test]
fn callbacks_recurse_through_enum_child_windows() {
    let (r, out, _) = run_main(
        "Public Function Child(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"  child \" & CStr(hwnd) & \" of \" & CStr(lParam)
    Child = 1
End Function
Public Function Top(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"top \" & CStr(hwnd)
    Call EnumChildWindows(hwnd, AddressOf Child, hwnd)
    Top = 1
End Function
Public Sub Main()
    Call EnumWindows(AddressOf Top, 0)
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, "top 256\n  child 272 of 256\ntop 512\n");
}

#[test]
fn unbounded_recursion_through_callbacks_hits_a_depth_limit() {
    let (r, _, mut vm) = run_main(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Call EnumWindows(AddressOf Visit, 0)
    Visit = 1
End Function
Public Sub Main()
    Call EnumWindows(AddressOf Visit, 0)
End Sub
",
    );
    match &r {
        Err(VmError::Limit(LimitKind::CallDepth)) => {}
        Err(VmError::Runtime(e)) if e.error.number == 28 => {}
        other => panic!("not a depth limit: {other:?}"),
    }
    // The session recovers.
    vm.reset().unwrap();
}

#[test]
fn a_callback_cancelled_by_the_step_limit_is_not_trappable() {
    let (mut vm, sink) = build(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Do
    Loop
End Function
Public Sub Main()
    On Error Resume Next
    Call EnumWindows(AddressOf Visit, 0)
    Debug.Print \"trapped\"
End Sub
",
        granted(),
        ResourceLimits::default().with_max_steps(Some(10_000)),
    );
    let main = vm.find_procedure("Main").unwrap();
    let r = vm.run_procedure(main, vec![]);
    assert!(
        matches!(r, Err(VmError::Limit(LimitKind::Steps))),
        "{:?}",
        r.map(|_| ())
    );
    assert_eq!(sink.contents(), "");
    vm.reset().unwrap();
}

const TOKEN_FN: &str =
    "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"visit\"
    Visit = 1
End Function
Public Function Tok() As LongPtr
    Tok = AddressOf Visit
End Function
Public Function Use(ByVal t As LongPtr) As Long
    Dim r As Long
    Err.Clear
    On Error Resume Next
    r = EnumWindows(t, 0)
    Use = r + 1000 * Err.Number
End Function
";

fn token_of(vm: &mut Vm) -> i64 {
    let p = vm.find_procedure("Tok").unwrap();
    match vm.run_procedure(p, vec![]).unwrap().value {
        Variant::LongLong(v) => v,
        other => panic!("{other:?}"),
    }
}

fn use_token(vm: &mut Vm, t: i64) -> i32 {
    let p = vm.find_procedure("Use").unwrap();
    match vm
        .run_procedure(p, vec![Arg::Value(Variant::LongLong(t))])
        .unwrap()
        .value
    {
        Variant::Long(v) => v,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_live_token_works_and_a_stale_one_is_refused_after_reset() {
    let (mut vm, sink) = build(TOKEN_FN, granted(), limits());
    let t = token_of(&mut vm);
    assert_eq!(use_token(&mut vm, t), 1);
    assert_eq!(sink.contents(), "visit\nvisit\n");
    vm.reset().unwrap();
    // Error 5: invalid procedure call.
    assert_eq!(use_token(&mut vm, t), 5000, "stale token must be refused");
    let t2 = token_of(&mut vm);
    assert_ne!(t, t2, "a new session mints new tokens");
    assert_eq!(use_token(&mut vm, t2), 1);
}

#[test]
fn end_invalidates_tokens_too() {
    let (mut vm, _) = build(
        &format!(
            "{TOKEN_FN}Public Sub Stop_()
    End
End Sub
"
        ),
        granted(),
        limits(),
    );
    let t = token_of(&mut vm);
    let p = vm.find_procedure("Stop_").unwrap();
    vm.run_procedure(p, vec![]).unwrap();
    assert_eq!(use_token(&mut vm, t), 5000);
}

#[test]
fn tokens_forged_by_arithmetic_or_guessing_are_refused() {
    let (mut vm, sink) = build(TOKEN_FN, granted(), limits());
    let t = token_of(&mut vm);
    let mut accepted = 0;
    for forged in [
        0,
        1,
        -1,
        t + 1,
        t - 1,
        t * 2,
        t ^ 0x400,
        t & 0xFFFF,
        t | 0x8000_0000,
        t + (1 << 32),
        i64::MAX,
        i64::MIN,
        0x1234_5678,
        0x7FFF_FFFF,
    ] {
        if use_token(&mut vm, forged) == 1 {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 0);
    // Only the genuine calls ran the procedure; no forged one did.
    assert_eq!(sink.contents(), "");
    assert_eq!(use_token(&mut vm, t), 1);
}

#[test]
fn forging_inside_the_program_is_refused_too() {
    let (r, out, _) = run_main(
        "Public Function Visit(ByVal hwnd As LongPtr, ByVal lParam As LongPtr) As Long
    Debug.Print \"ran\"
End Function
Public Sub Main()
    On Error Resume Next
    Call EnumWindows(AddressOf Visit + 4, 0)
    Debug.Print Err.Number
    Err.Clear
    Call EnumWindows(AddressOf Visit * 2, 0)
    Debug.Print Err.Number
    Err.Clear
    Call EnumWindows(12345, 0)
    Debug.Print Err.Number
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, " 5 \n 5 \n 5 \n");
}

#[test]
fn a_procedure_that_does_not_fit_the_signature_is_error_49() {
    for (name, decl) in [
        (
            "arity",
            "Function Visit(ByVal h As LongPtr) As Long\nVisit = 1\nEnd Function",
        ),
        (
            "byref",
            "Function Visit(ByRef h As LongPtr, ByVal l As LongPtr) As Long\nVisit = 1\nEnd Function",
        ),
        (
            "type",
            "Function Visit(ByVal h As Long, ByVal l As LongPtr) As Long\nVisit = 1\nEnd Function",
        ),
        (
            "sub",
            "Sub Visit(ByVal h As LongPtr, ByVal l As LongPtr)\nEnd Sub",
        ),
        (
            "result",
            "Function Visit(ByVal h As LongPtr, ByVal l As LongPtr) As Double\nVisit = 1\nEnd Function",
        ),
    ] {
        let (r, out, _) = run_main(&format!(
            "{decl}
Public Sub Main()
    On Error Resume Next
    Call EnumWindows(AddressOf Visit, 0)
    Debug.Print Err.Number
End Sub
"
        ));
        r.unwrap();
        assert_eq!(out, " 49 \n", "{name}");
    }
}

#[test]
fn qsort_sorts_a_vba_array_with_a_vba_comparison_that_uses_declare() {
    let (r, out, _) = run_main(
        "Public Function Cmp(ByVal a As LongPtr, ByVal b As LongPtr) As Long
    Dim x As Long, y As Long
    CopyMemory x, a, 4
    CopyMemory y, b, 4
    If x < y Then
        Cmp = -1
    ElseIf x > y Then
        Cmp = 1
    End If
End Function
Public Sub Main()
    Dim v(0 To 5) As Long, i As Long, s As String
    v(0) = 5: v(1) = -3: v(2) = 9: v(3) = 0: v(4) = 9: v(5) = 1
    qsort v, 6, 4, AddressOf Cmp
    For i = 0 To 5
        s = s & v(i) & \" \"
    Next
    Debug.Print s
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, "-3 0 1 5 9 9 \n");
}

#[test]
fn timers_fire_from_doevents_and_stop_after_killtimer() {
    let (r, out, _) = run_main(
        "Dim ticks As Long
Public Sub Tick(ByVal hwnd As LongPtr, ByVal msg As Long, ByVal id As LongPtr, ByVal t As Long)
    ticks = ticks + 1
End Sub
Public Sub Main()
    Dim id As LongPtr
    id = SetTimer(0, 0, 100, AddressOf Tick)
    DoEvents
    Debug.Print ticks
    Sleep 150
    DoEvents
    Debug.Print ticks
    DoEvents
    Debug.Print ticks
    Debug.Print KillTimer(0, id)
    Sleep 500
    DoEvents
    Debug.Print ticks
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, " 0 \n 1 \n 1 \n 1 \n 1 \n");
}

#[test]
fn a_timer_callback_error_is_raised_by_doevents() {
    let (r, out, _) = run_main(
        "Public Sub Tick(ByVal hwnd As LongPtr, ByVal msg As Long, ByVal id As LongPtr, ByVal t As Long)
    Err.Raise 777
End Sub
Public Sub Main()
    Dim id As LongPtr
    id = SetTimer(0, 0, 100, AddressOf Tick)
    Sleep 200
    On Error Resume Next
    DoEvents
    Debug.Print Err.Number
End Sub
",
    );
    r.unwrap();
    assert_eq!(out, " 777 \n");
}

#[test]
fn timers_do_not_survive_a_reset() {
    let (mut vm, sink) = build(
        "Dim ticks As Long
Public Sub Tick(ByVal hwnd As LongPtr, ByVal msg As Long, ByVal id As LongPtr, ByVal t As Long)
    Debug.Print \"tick\"
End Sub
Public Sub Arm()
    Call SetTimer(0, 0, 100, AddressOf Tick)
End Sub
Public Sub Pump()
    Sleep 500
    DoEvents
End Sub
",
        granted(),
        limits(),
    );
    let arm = vm.find_procedure("Arm").unwrap();
    let pump = vm.find_procedure("Pump").unwrap();
    vm.run_procedure(arm, vec![]).unwrap();
    vm.reset().unwrap();
    vm.run_procedure(pump, vec![]).unwrap();
    assert_eq!(sink.contents(), "");
}
