//! Random multi-module programs: helper procedures (recursion, `ByRef`,
//! `ParamArray`, optional arguments, error handlers), classes with
//! `Class_Initialize`/`Class_Terminate`, an interface and an implementer,
//! arrays of several ranks, user-defined types, `Variant` arithmetic over
//! mixed subtypes, collections and `On Error` in every form.
//!
//! The interpreter must never panic, never report an internal error, stay
//! within its limits and be deterministic (same output, result and step
//! count when the same program runs twice). Most generated programs must
//! compile, otherwise the run proves little.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::*;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestCaseError, TestRunner};
use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig, VmError};

const MAX_STEPS: u64 = 80_000;

const STANDARD: &str = r#"Option Explicit
Private Type Pt
    X As Long
    S As String
    D As Double
End Type
Public gl As Long
Public gs As String
Public gv As Variant
Dim garr(1 To 4, 0 To 2) As Long
Dim dyn() As Variant
Function Dbl(ByVal x As Long) As Long
    Dbl = x * 2
End Function
Function Fact(ByVal n As Long) As Double
    If n <= 1 Then
        Fact = 1
    Else
        Fact = n * Fact(n - 1)
    End If
End Function
Sub Swap(ByRef p As Long, ByRef q As Long)
    Dim t As Long
    t = p: p = q: q = t
End Sub
Sub Thrower(ByVal n As Long)
    If n Mod 3 = 0 Then Err.Raise 1000 + (n Mod 50), "Thrower", "boom"
End Sub
Function Safe(ByVal n As Long) As Long
    On Error Resume Next
    Safe = 100 \ n
    If Err.Number <> 0 Then Safe = -1
End Function
Function Guarded(ByVal n As Long) As String
    On Error GoTo bad
    Thrower n
    Guarded = "ok" & n
    Exit Function
bad:
    Guarded = "caught " & Err.Number
End Function
Function Loopy(ByVal n As Long) As Long
    Dim i As Long, t As Long
    For i = 1 To n Mod 20
        t = t + i
        If t > 100 Then Exit For
    Next
    Loopy = t
End Function
Function Va(ParamArray xs() As Variant) As Long
    Dim v As Variant, t As Long
    For Each v In xs
        t = t + 1
    Next
    Va = t
End Function
Function Opt(ByVal a As Long, Optional ByVal b As Long = 5, Optional c As Variant) As Long
    If IsMissing(c) Then
        Opt = a + b
    Else
        Opt = a + b + c
    End If
End Function
Function Boom(ByVal n As Long) As Long
    Boom = Boom(n + 1)
End Function
Function SumArr(ByRef xs() As Long) As Long
    Dim i As Long, r As Long
    For i = LBound(xs) To UBound(xs)
        r = r + xs(i)
    Next
    SumArr = r
End Function
Sub Main()
    Dim a As Long, b As Long, c As Long, i As Long, j As Long
    Dim d As Double, sg As Single
    Dim s As String, t As String
    Dim v As Variant, w As Variant
    Dim arr(1 To 5) As Long
    Dim m(0 To 2, 1 To 3) As Double
    Dim p As Pt, q As Pt
    Dim col As New Collection
    Dim o As Acc
    Dim th As IThing
    Dim ob As Object
    ReDim dyn(1 To 3)
    Set o = New Acc
    MODE
BODY
    Exit Sub
sub1:
    gl = gl + 1
    Return
h:
    Debug.Print "handler " & Err.Number
    Resume Next
End Sub
"#;

const ACC: &str = r#"Public Total As Long
Public Name As String
Private mItems As New Collection
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
Private Sub Class_Terminate()
    Debug.Print "term " & Total
End Sub
Public Sub Add(ByVal n As Long)
    Total = Total + n
    mItems.Add n
End Sub
Public Property Get Count() As Long
    Count = mItems.Count
End Property
Public Property Let Label(ByVal s As String)
    Name = s
End Property
Public Property Get Label() As String
    Label = Name
End Property
Public Function Fail(ByVal n As Long) As Long
    If n > 2 Then Err.Raise 5
    Fail = n
End Function
"#;

const ITHING: &str = "Public Function Area() As Double\nEnd Function\n";

const SQ: &str = r#"Implements IThing
Public Side As Double
Private Function IThing_Area() As Double
    IThing_Area = Side * Side
End Function
Private Sub Class_Terminate()
    Debug.Print "sq gone"
End Sub
"#;

fn int_expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        Just("a".to_owned()),
        Just("b".to_owned()),
        Just("c".to_owned()),
        Just("i".to_owned()),
        Just("gl".to_owned()),
        Just("Len(s)".to_owned()),
        Just("p.X".to_owned()),
        Just("col.Count".to_owned()),
        Just("UBound(dyn)".to_owned()),
        Just("o.Count".to_owned()),
        Just("o.Total".to_owned()),
        Just("arr(2)".to_owned()),
        Just("garr(2, 1)".to_owned()),
        Just("Asc(\"A\")".to_owned()),
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
                    Just("Or"),
                    Just("Xor")
                ],
                inner.clone()
            )
                .prop_map(|(l, o, r)| format!("({l} {o} {r})")),
            inner.clone().prop_map(|e| format!("Abs({e})")),
            inner.clone().prop_map(|e| format!("arr(({e}) Mod 7)")),
            inner.clone().prop_map(|e| format!("m(({e}) Mod 4, ({e}) Mod 5)")),
            inner.clone().prop_map(|e| format!("garr(({e}) Mod 5, ({e}) Mod 4)")),
            inner.clone().prop_map(|e| format!("Dbl({e})")),
            inner.clone().prop_map(|e| format!("Safe({e})")),
            inner.clone().prop_map(|e| format!("Loopy({e})")),
            inner.clone().prop_map(|e| format!("Opt({e})")),
            (inner.clone(), inner.clone()).prop_map(|(x, y)| format!("Opt({x}, {y}, v)")),
            inner.clone().prop_map(|e| format!("Va({e}, s, v)")),
            inner.clone().prop_map(|e| format!("o.Fail({e})")),
            inner.clone().prop_map(|e| format!("Rec(({e}) Mod 40)")),
            inner.clone().prop_map(|e| format!("CLng(Fact(({e}) Mod 12))")),
            inner.clone().prop_map(|e| format!("CLng(Sqr(Abs({e})))")),
            inner.clone().prop_map(|e| format!("CLng(v) + ({e})")),
            inner.clone().prop_map(|e| format!("Len(Space(({e}) Mod 5000))")),
            inner.clone().prop_map(|e| format!("InStr(s & \"a\", Chr(97 + (({e}) Mod 3)))")),
            inner.prop_map(|e| format!("Int(Val(Format({e}, \"0\")))")),
        ]
    })
}

fn double_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("d".to_owned()),
        Just("sg".to_owned()),
        Just("p.D".to_owned()),
        Just("1.5".to_owned()),
        Just("1E+300".to_owned()),
        int_expr().prop_map(|e| format!("({e}) / 3#")),
        int_expr().prop_map(|e| format!("({e}) ^ 0.5")),
        int_expr().prop_map(|e| format!("CDbl({e}) * 1E+200")),
        int_expr().prop_map(|e| format!("Fact(({e}) Mod 30)")),
        Just("m(1, 2)".to_owned()),
    ]
}

fn str_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("s".to_owned()),
        Just("t".to_owned()),
        Just("\"ab\"".to_owned()),
        Just("\"\"".to_owned()),
        Just("p.S".to_owned()),
        Just("gs".to_owned()),
        Just("o.Label".to_owned()),
        int_expr().prop_map(|e| format!("CStr({e})")),
        int_expr().prop_map(|e| format!("Left(s, {e})")),
        int_expr().prop_map(|e| format!("Right(s, {e})")),
        int_expr().prop_map(|e| format!("Mid(s, {e}, 3)")),
        int_expr().prop_map(|e| format!("Space(({e}) Mod 300)")),
        int_expr().prop_map(|e| format!("String(({e}) Mod 300, \"z\")")),
        int_expr().prop_map(|e| format!("Guarded({e})")),
        int_expr().prop_map(|e| format!("Hex({e})")),
        int_expr().prop_map(|e| format!("Format({e}, \"#,##0.00\")")),
        int_expr().prop_map(|e| format!("Str({e})")),
        int_expr().prop_map(|e| format!("Chr(65 + (({e}) Mod 26))")),
        Just("(s & t)".to_owned()),
        Just("Replace(s, \"a\", \"bb\")".to_owned()),
        Just("UCase(s) & LCase(t)".to_owned()),
        Just("Trim(s)".to_owned()),
        Just("StrReverse(s)".to_owned()),
        Just("Join(Split(s, \"a\"), \"-\")".to_owned()),
        Just("TypeName(v)".to_owned()),
        Just("CStr(v)".to_owned()),
    ]
}

fn variant_expr() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("v".to_owned()),
        Just("w".to_owned()),
        Just("Null".to_owned()),
        Just("Empty".to_owned()),
        Just("CVErr(2007)".to_owned()),
        Just("#1/2/2020#".to_owned()),
        Just("CDate(1.5)".to_owned()),
        Just("CCur(12.5)".to_owned()),
        Just("CByte(200)".to_owned()),
        Just("CInt(300)".to_owned()),
        Just("True".to_owned()),
        Just("Array(1, \"x\", 2.5)".to_owned()),
        Just("Array()".to_owned()),
        int_expr(),
        double_expr(),
        str_expr(),
        Just("dyn".to_owned()),
        Just("o".to_owned()),
        Just("Nothing".to_owned()),
    ]
}

fn stmt() -> impl Strategy<Value = String> {
    let simple = prop_oneof![
        (
            prop_oneof![Just("a"), Just("b"), Just("c"), Just("gl"), Just("p.X"), Just("j")],
            int_expr()
        )
            .prop_map(|(t, e)| format!("{t} = {e}\n")),
        double_expr().prop_map(|e| format!("d = {e}\n")),
        double_expr().prop_map(|e| format!("sg = {e}\n")),
        double_expr().prop_map(|e| format!("p.D = {e}\n")),
        (prop_oneof![Just("s"), Just("t"), Just("gs"), Just("p.S")], str_expr())
            .prop_map(|(t, e)| format!("{t} = {e}\n")),
        (prop_oneof![Just("v"), Just("w"), Just("gv")], variant_expr())
            .prop_map(|(t, e)| format!("{t} = {e}\n")),
        (variant_expr(), prop_oneof![Just("+"), Just("-"), Just("*"), Just("/"), Just("&"), Just("\\"), Just("Mod"), Just("^")], variant_expr())
            .prop_map(|(l, o, r)| format!("w = ({l}) {o} ({r})\n")),
        (variant_expr(), prop_oneof![Just("="), Just("<"), Just(">="), Just("<>"), Just("Like"), Just("And"), Just("Or")], variant_expr())
            .prop_map(|(l, o, r)| format!("Debug.Print ({l}) {o} ({r})\n")),
        variant_expr().prop_map(|e| format!("Debug.Print TypeName({e}); VarType({e}); IsNumeric({e}); IsNull({e})\n")),
        variant_expr().prop_map(|e| format!("Debug.Print -({e}); Not ({e})\n")),
        Just("v = v + 1\n".to_owned()),
        Just("s = v\n".to_owned()),
        Just("v = s\n".to_owned()),
        Just("v = v & v\n".to_owned()),
        Just("q = p\n".to_owned()),
        Just("p = q\n".to_owned()),
        (int_expr(), int_expr()).prop_map(|(i, e)| format!("arr({i}) = {e}\n")),
        (int_expr(), int_expr(), double_expr())
            .prop_map(|(i, j, e)| format!("m(({i}) Mod 4, ({j}) Mod 5) = {e}\n")),
        (int_expr(), int_expr(), int_expr())
            .prop_map(|(i, j, e)| format!("garr(({i}) Mod 5, ({j}) Mod 4) = {e}\n")),
        (int_expr(), variant_expr()).prop_map(|(i, e)| format!("dyn({i}) = {e}\n")),
        int_expr().prop_map(|e| format!("ReDim dyn(1 To {e})\n")),
        int_expr().prop_map(|e| format!("ReDim Preserve dyn(1 To {e})\n")),
        Just("Erase dyn\n".to_owned()),
        Just("Erase arr\n".to_owned()),
        Just("a = SumArr(arr)\n".to_owned()),
        Just("v = dyn\n".to_owned()),
        Just("v = Split(s, \"a\")\n".to_owned()),
        Just("Debug.Print UBound(v); LBound(v)\n".to_owned()),
        Just("Swap a, b\n".to_owned()),
        Just("Swap arr(1), c\n".to_owned()),
        Just("Swap p.X, a\n".to_owned()),
        Just("Swap garr(1, 1), gl\n".to_owned()),
        int_expr().prop_map(|e| format!("Thrower {e}\n")),
        int_expr().prop_map(|e| format!("col.Add {e}\n")),
        Just("col.Add s, s\n".to_owned()),
        Just("col.Add o, \"obj\"\n".to_owned()),
        Just("col.Remove 1\n".to_owned()),
        int_expr().prop_map(|e| format!("Debug.Print col({e})\n")),
        int_expr().prop_map(|e| format!("o.Add {e}\n")),
        str_expr().prop_map(|e| format!("o.Label = {e}\n")),
        Just("Set o = New Acc\n".to_owned()),
        Just("Set o = Nothing\n".to_owned()),
        Just("Set ob = o\n".to_owned()),
        int_expr().prop_map(|e| format!("ob.Add {e}\n")),
        Just("Debug.Print ob.Total; TypeName(ob)\n".to_owned()),
        int_expr().prop_map(|e| format!("Call CallByName(o, \"Add\", VbMethod, {e})\n")),
        Just("Set th = New Sq\n".to_owned()),
        Just("Set th = Nothing\n".to_owned()),
        Just("Debug.Print th.Area\n".to_owned()),
        Just("Debug.Print TypeOf th Is Sq; o Is ob\n".to_owned()),
        int_expr().prop_map(|e| format!("Err.Raise {e}\n")),
        Just("Err.Clear\n".to_owned()),
        Just("Debug.Print Err.Number; Err.Description; Erl\n".to_owned()),
        Just("On Error Resume Next\n".to_owned()),
        Just("On Error GoTo h\n".to_owned()),
        Just("On Error GoTo 0\n".to_owned()),
        Just("a = Boom(0)\n".to_owned()),
        (int_expr(), int_expr()).prop_map(|(l, r)| format!("Debug.Print {l}; {r}; Tab(5); Spc(2); \"|\"\n")),
        str_expr().prop_map(|e| format!("Debug.Print {e}\n")),
        int_expr().prop_map(|e| format!("Mid(s, {e}, 2) = \"zz\"\n")),
        double_expr().prop_map(|e| format!("Debug.Print Format({e}, \"0.###\"); Int({e}); Fix({e}); Round({e}, 2)\n")),
        Just("Debug.Print DateAdd(\"d\", a, #1/1/2000#); Weekday(#1/1/2000#)\n".to_owned()),
    ];
    simple.prop_recursive(3, 40, 4, |inner| {
        prop_oneof![
            (int_expr(), prop::collection::vec(inner.clone(), 1..4), prop::collection::vec(inner.clone(), 0..3))
                .prop_map(|(c, a, b)| {
                    format!("If ({c}) > 0 Then\n{}ElseIf ({c}) < -3 Then\nb = b + 1\nElse\n{}End If\n", a.concat(), b.concat())
                }),
            (0i32..6, prop::collection::vec(inner.clone(), 1..4))
                .prop_map(|(n, body)| format!("For i = 1 To {n}\n{}Next\n", body.concat())),
            (1i32..5, prop::collection::vec(inner.clone(), 1..3))
                .prop_map(|(n, body)| format!("For j = {n} To 1 Step -1\n{}Next j\n", body.concat())),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "a = 0\nDo While a < 4\na = a + 1\n{}Loop\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "b = 0\nDo\nb = b + 1\n{}Loop Until b >= 3\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "c = 0\nWhile c < 3\nc = c + 1\n{}Wend\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "For Each v In col\n{}Next\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "For Each v In arr\n{}Next\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "For i = 1 To 3\nIf i = 2 Then Exit For\n{}Next\n",
                body.concat()
            )),
            (str_expr(), prop::collection::vec(inner.clone(), 1..3)).prop_map(|(e, body)| format!(
                "Select Case {e}\nCase \"ab\", \"\"\nb = 1\nCase Is > \"m\"\n{}Case Else\nc = c + 1\nEnd Select\n",
                body.concat()
            )),
            (int_expr(), prop::collection::vec(inner.clone(), 1..3)).prop_map(|(e, body)| format!(
                "Select Case {e}\nCase 0\nb = 1\nCase 1 To 10, 20\n{}Case Else\nc = c + 1\nEnd Select\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "With p\n{}.X = .X + 1\nEnd With\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "With o\n.Add 1\n{}End With\n",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "GoSub sub1\n{}",
                body.concat()
            )),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|body| format!(
                "On Error Resume Next\n{}On Error GoTo 0\n",
                body.concat()
            )),
            prop::collection::vec(inner, 1..3).prop_map(|body| format!(
                "On Error GoTo h\n{}On Error GoTo 0\n",
                body.concat()
            )),
        ]
    })
}

fn program(mode: u8, body: &[String]) -> String {
    let mode = match mode {
        0 => "",
        1 => "On Error Resume Next",
        _ => "On Error GoTo h",
    };
    // `Rec` is defined once, here, so statements can call it.
    STANDARD
        .replace("MODE", mode)
        .replace("BODY", &body.concat())
        + "Function Rec(ByVal n As Long) As Long\nIf n <= 0 Then\nRec = 0\nElse\nRec = 1 + Rec(n - 1)\nEnd If\nEnd Function\n"
}

struct Run {
    out: String,
    result: Result<vba_vm::RunOutput, VmError>,
    steps: u64,
}

fn mods(src: &str) -> Vec<vba_sema::SourceModule> {
    vec![
        module("M", ModuleKind::Standard, src),
        module("Acc", ModuleKind::Class, ACC),
        module("IThing", ModuleKind::Class, ITHING),
        module("Sq", ModuleKind::Class, SQ),
    ]
}

/// `None` when the program does not compile.
fn execute(src: &str) -> Option<Run> {
    let a = vba_sema::analyze_sources(
        &mods(src),
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

#[test]
fn random_programs_never_panic_stay_bounded_and_are_deterministic() {
    let compiled = Cell::new(0u32);
    let total = Cell::new(0u32);
    let kinds = std::cell::RefCell::new(std::collections::BTreeMap::<&'static str, u32>::new());
    let mut runner = TestRunner::new(Config {
        cases: std::env::var("VBA_FUZZ_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300),
        failure_persistence: None,
        ..Config::default()
    });
    let strategy = (0u8..3, prop::collection::vec(stmt(), 1..12));
    let outcome = runner.run(&strategy, |(mode, body)| {
        let src = program(mode, &body);
        total.set(total.get() + 1);
        let Some(first) = execute(&src) else {
            return Ok(());
        };
        compiled.set(compiled.get() + 1);
        let kind = match &first.result {
            Ok(_) => "completed",
            Err(VmError::Runtime(_)) => "unhandled error",
            Err(VmError::Limit(_)) => "limit",
            Err(_) => "other",
        };
        *kinds.borrow_mut().entry(kind).or_default() += 1;
        prop_assert!(first.steps <= MAX_STEPS + 1, "steps {}", first.steps);
        if let Err(e) = &first.result {
            prop_assert!(!matches!(e, VmError::Internal(_)), "internal error: {e}\n{src}");
        }
        let second = execute(&src).expect("compiles again");
        prop_assert_eq!(&first.out, &second.out);
        prop_assert_eq!(&first.result, &second.result);
        prop_assert_eq!(first.steps, second.steps);
        Ok::<(), TestCaseError>(())
    });
    if let Err(e) = outcome {
        panic!("{e}");
    }
    if std::env::var_os("VBA_FUZZ_STATS").is_some() {
        eprintln!("compiled {} of {}: {:?}", compiled.get(), total.get(), kinds.borrow());
    }
    assert!(
        compiled.get() * 10 >= total.get() * 7,
        "only {} of {} generated programs compile",
        compiled.get(),
        total.get()
    );
}

#[test]
fn the_template_itself_compiles_and_runs() {
    let src = program(0, &["a = Dbl(3) + Safe(0) + Loopy(7)\nDebug.Print a; Guarded(3); Guarded(4); Va(1, 2, 3)\nSet th = New Sq\nDebug.Print th.Area\n".to_owned()]);
    let a = vba_sema::analyze_sources(
        &mods(&src),
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    let errors: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .map(|d| vba_diagnostics::render_plain(d, &a.sources))
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let run = execute(&src).expect("the template compiles");
    assert!(run.result.is_ok(), "{:?}\n{}", run.result, run.out);
    assert!(run.out.contains("caught 1003"), "{}", run.out);
}
