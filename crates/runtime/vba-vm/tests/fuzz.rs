//! Random programs: the interpreter never panics, never reports an internal
//! error, stays within its limits and is deterministic.

mod common;

use std::rc::Rc;

use common::*;
use proptest::prelude::*;
use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig, VmError};

const MAX_STEPS: u64 = 60_000;

const HEADER: &str = "Private Type Pt\nX As Long\nS As String\nEnd Type\nDim g As Long\nDim dyn() As Long\nFunction F1(ByVal x As Long) As Long\nF1 = x * 2 + 1\nEnd Function\nSub S1(ByRef x As Long)\nx = x + 1\nEnd Sub\nFunction Rec(ByVal n As Long) As Long\nIf n <= 0 Then\nRec = 0\nElse\nRec = 1 + Rec(n - 1)\nEnd If\nEnd Function\nFunction Boom(ByVal n As Long) As Long\nBoom = Boom(n + 1)\nEnd Function\n";

const DECLS: &str = "Dim a As Long, b As Long, c As Long, i As Long\nDim d As Double\nDim s As String\nDim v As Variant\nDim arr(1 To 5) As Long\nDim p As Pt\nDim col As New Collection\nReDim dyn(1 To 3)\n";

fn int_expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("a".to_owned()),
        Just("b".to_owned()),
        Just("c".to_owned()),
        Just("i".to_owned()),
        Just("g".to_owned()),
        Just("Len(s)".to_owned()),
        Just("p.X".to_owned()),
        Just("col.Count".to_owned()),
        Just("UBound(dyn)".to_owned()),
        (-5i32..40).prop_map(|n| format!("{n}")),
        (1_000_000i32..2_000_000_000).prop_map(|n| format!("{n}")),
    ];
    leaf.prop_recursive(3, 24, 3, |inner| {
        prop_oneof![
            (
                inner.clone(),
                prop_oneof![
                    Just("+"),
                    Just("-"),
                    Just("*"),
                    Just("\\"),
                    Just("Mod"),
                    Just("And"),
                    Just("Or")
                ],
                inner.clone()
            )
                .prop_map(|(l, o, r)| format!("({l} {o} {r})")),
            inner.clone().prop_map(|e| format!("Abs({e})")),
            inner.clone().prop_map(|e| format!("arr(({e}) Mod 7)")),
            inner.clone().prop_map(|e| format!("F1({e})")),
            inner.clone().prop_map(|e| format!("Rec(({e}) Mod 40)")),
            inner.clone().prop_map(|e| format!("CLng(Sqr(Abs({e})))")),
            inner.prop_map(|e| format!("Len(Space({e}))")),
        ]
    })
}

fn str_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("s".to_owned()),
        Just("\"ab\"".to_owned()),
        Just("\"\"".to_owned()),
        Just("p.S".to_owned()),
        int_expr().prop_map(|e| format!("CStr({e})")),
        int_expr().prop_map(|e| format!("Left(s, {e})")),
        int_expr().prop_map(|e| format!("Right(s, {e})")),
        int_expr().prop_map(|e| format!("Space({e})")),
        int_expr().prop_map(|e| format!("String({e}, \"z\")")),
        Just("(s & s)".to_owned()),
        Just("Replace(s, \"a\", \"bb\")".to_owned()),
        Just("UCase(s) & LCase(s)".to_owned()),
        Just("Join(Split(s, \"a\"), \"-\")".to_owned()),
    ]
}

fn stmt() -> impl Strategy<Value = String> {
    let simple = prop_oneof![
        (
            prop_oneof![Just("a"), Just("b"), Just("c"), Just("g"), Just("p.X")],
            int_expr()
        )
            .prop_map(|(t, e)| format!("{t} = {e}\n")),
        str_expr().prop_map(|e| format!("s = {e}\n")),
        str_expr().prop_map(|e| format!("p.S = {e}\n")),
        int_expr().prop_map(|e| format!("d = {e} / 3#\n")),
        (int_expr(), int_expr()).prop_map(|(i, e)| format!("arr({i}) = {e}\n")),
        int_expr().prop_map(|e| format!("v = {e}\n")),
        Just("v = s\n".to_owned()),
        Just("v = Array(1, \"x\", 2.5)\n".to_owned()),
        Just("v = Null\n".to_owned()),
        Just("S1 a\n".to_owned()),
        Just("S1 arr(2)\n".to_owned()),
        Just("S1 p.X\n".to_owned()),
        int_expr().prop_map(|e| format!("ReDim dyn(1 To {e})\n")),
        int_expr().prop_map(|e| format!("ReDim Preserve dyn(1 To {e})\n")),
        Just("Erase dyn\n".to_owned()),
        Just("Erase arr\n".to_owned()),
        int_expr().prop_map(|e| format!("col.Add {e}\n")),
        Just("col.Add s, s\n".to_owned()),
        Just("col.Remove 1\n".to_owned()),
        int_expr().prop_map(|e| format!("Debug.Print col({e})\n")),
        int_expr().prop_map(|e| format!("Err.Raise {e}\n")),
        int_expr().prop_map(|e| format!("Mid(s, {e}, 2) = \"zz\"\n")),
        (int_expr(), int_expr()).prop_map(|(l, r)| format!("Debug.Print {l}; {r}\n")),
        str_expr().prop_map(|e| format!("Debug.Print {e}\n")),
        Just("Debug.Print Err.Number & Err.Description\n".to_owned()),
        Just("Err.Clear\n".to_owned()),
        Just("a = Boom(0)\n".to_owned()),
        Just("v = v + 1\n".to_owned()),
        Just("s = v\n".to_owned()),
        Just("Debug.Print TypeName(v) & VarType(v)\n".to_owned()),
    ];
    simple.prop_recursive(3, 30, 4, |inner| {
        prop_oneof![
            (int_expr(), prop::collection::vec(inner.clone(), 1..4)).prop_map(|(c, body)| {
                format!("If ({c}) > 0 Then\n{}Else\nb = b + 1\nEnd If\n", body.concat())
            }),
            (0i32..6, prop::collection::vec(inner.clone(), 1..4))
                .prop_map(|(n, body)| format!("For i = 1 To {n}\n{}Next\n", body.concat())),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "a = 0\nDo While a < 5\na = a + 1\n{}Loop\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "For Each v In col\n{}Next\n",
                body.concat()
            )),
            (int_expr(), prop::collection::vec(inner.clone(), 1..3)).prop_map(|(e, body)| format!(
                "Select Case {e}\nCase 0\nb = 1\nCase Is > 10\n{}Case Else\nc = c + 1\nEnd Select\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "With p\n{}.X = .X + 1\nEnd With\n",
                body.concat()
            )),
            prop::collection::vec(inner, 1..3).prop_map(|body| format!(
                "GoSub sub1\n{}",
                body.concat()
            )),
        ]
    })
}

fn program(mode: u8, body: &[String]) -> String {
    let on_error = match mode {
        0 => "",
        1 => "On Error Resume Next\n",
        _ => "On Error GoTo h\n",
    };
    format!(
        "{HEADER}Sub Main()\n{DECLS}{on_error}{}Exit Sub\nsub1:\ng = g + 1\nReturn\nh:\nDebug.Print \"handler \" & Err.Number\nResume Next\nEnd Sub\n",
        body.concat()
    )
}

struct Run {
    out: String,
    result: Result<vba_vm::RunOutput, VmError>,
    steps: u64,
}

fn execute(src: &str) -> Option<Run> {
    let a = vba_sema::analyze_sources(
        &[module("M", ModuleKind::Standard, src)],
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    if a.has_errors() {
        return None;
    }
    let ir = vba_ir::lower(&a).ok()?;
    vba_ir::verify(&ir).expect("lowered programs verify");
    let host = Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(MAX_STEPS),
        max_memory_bytes: Some(8 << 20),
        max_call_depth: 60,
    }));
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        Rc::new(ir),
        host.clone(),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig {
            max_string_units: 1 << 16,
            max_array_elements: 1 << 16,
            ..VmConfig::default()
        },
    )
    .expect("vm");
    let main = vm.find_procedure("Main").expect("Main");
    let result = vm.run_procedure(main, vec![]);
    Some(Run {
        out: sink.contents(),
        result,
        steps: host.budget().steps_used(),
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn random_programs_never_panic_and_stay_bounded(
        mode in 0u8..3,
        body in prop::collection::vec(stmt(), 1..10),
    ) {
        let src = program(mode, &body);
        let Some(first) = execute(&src) else {
            return Ok(());
        };
        prop_assert!(first.steps <= MAX_STEPS + 1, "steps {}", first.steps);
        if let Err(e) = &first.result {
            prop_assert!(!matches!(e, VmError::Internal(_)), "internal error: {e}\n{src}");
        }
        // Determinism: a second run is identical.
        let second = execute(&src).expect("compiles again");
        prop_assert_eq!(&first.out, &second.out);
        prop_assert_eq!(&first.result, &second.result);
        prop_assert_eq!(first.steps, second.steps);
    }
}

#[test]
fn nested_loops_with_resume_next_stay_within_the_step_budget() {
    let body = "a = 0\nDo While a < 5\na = a + 1\nFor i = 1 To 3\nFor i = 1 To 1\ncol.Add Len(Space(Len(Space(col.Count))))\nv = v + 1\nNext\nNext\nLoop\n";
    let src = program(1, &[body.to_owned()]);
    let run = execute(&src).expect("compiles");
    assert!(run.steps <= MAX_STEPS + 1, "steps {}", run.steps);
}

#[test]
fn memory_is_bounded_by_the_configured_limits() {
    // A program that tries to grow without bound is stopped by a limit and
    // does not exhaust the machine.
    let src = "Sub Main()\nDim s As String, a() As Long, i As Long\ns = \"x\"\nReDim a(1 To 10)\nDo\ns = s & s\nLoop\nEnd Sub\n";
    let r = execute(src).expect("compiles");
    assert!(matches!(r.result, Err(VmError::Limit(_))), "{:?}", r.result);
}
