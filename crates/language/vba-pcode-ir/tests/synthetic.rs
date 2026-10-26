//! Lowering and execution of hand-assembled caches (no local project needed).
//!
//! Every test assembles instruction streams from the documented layouts,
//! lowers them with `vba-pcode-ir`, runs the result on `vba-vm` and compares
//! with an expectation worked out by hand from the VBA semantics.
#![allow(missing_docs)]

mod common;

use std::rc::Rc;

use common::synth::*;
use vba_host::HeadlessHost;
use vba_pcode::Opcode;
use vba_pcode_ir::{LoweredProject, ModuleInput, Outcome, lower_project};
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, RunOutput, Vm, VmConfig, VmError};

fn lower(t: &Tables, lines: &[Line]) -> LoweredProject {
    let project = build_project_stream(&t.idents);
    let cache = build_cache(lines, &t.ind);
    let lp = lower_project(
        &project,
        1,
        &[ModuleInput {
            name: "Calc",
            class_like: false,
            cache: &cache,
        }],
    )
    .expect("lowers");
    vba_ir::verify(&lp.ir).expect("verified");
    lp
}

fn run(lp: &LoweredProject, entry: &str, args: &[Variant]) -> (Result<RunOutput, VmError>, String) {
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        Rc::new(lp.ir.clone()),
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    let p = vm.find_procedure(entry).expect("entry");
    let r = vm.run_procedure(p, args.iter().cloned().map(Arg::Value).collect());
    (r, sink.contents())
}

fn value(lp: &LoweredProject, entry: &str, args: &[Variant]) -> Variant {
    let (r, _) = run(lp, entry, args);
    r.expect("runs").value
}

fn l(v: i32) -> Variant {
    Variant::Long(v)
}

fn s(v: &str) -> Variant {
    Variant::String(v.into())
}

/// `Function name(params) As ret` plus `End Function`, with the body lines
/// in between.
fn function(
    t: &mut Tables,
    name: &str,
    ret: u8,
    params: &[(&str, u8, bool)],
    body: Vec<Line>,
) -> Vec<Line> {
    let f = t.function(name, Some(ret), params);
    let mut v = vec![Line::new().rec(Opcode::ProcHeader, 6, f)];
    v.extend(body);
    v.push(Line::new().op(Opcode::ProcEndFunction));
    v
}

#[test]
fn arithmetic_operators_and_operand_order() {
    let mut t = Tables::default();
    let (a, b) = (t.name("a"), t.name("b"));
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Calc = a * b + (a - b) \ 2          (7, 3 -> 23)
    let n = t.name("Calc");
    lines.extend(function(
        &mut t,
        "Calc",
        T_LONG,
        &[("a", T_LONG, true), ("b", T_LONG, true)],
        vec![
            Line::new()
                .read(a)
                .read(b)
                .op(Opcode::NumMultiply)
                .read(a)
                .read(b)
                .op(Opcode::NumSubtract)
                .op(Opcode::ExprParentheses)
                .i16(2)
                .op(Opcode::NumIntDivide)
                .op(Opcode::NumAdd)
                .write(n),
        ],
    ));
    // Quot = a / b  and  Rem = a Mod b  (7, 2 -> 3.5 and 1)
    let q = t.name("Quot");
    lines.extend(function(
        &mut t,
        "Quot",
        T_DOUBLE,
        &[("a", T_LONG, true), ("b", T_LONG, true)],
        vec![Line::new().read(a).read(b).op(Opcode::NumDivide).write(q)],
    ));
    let r = t.name("Remn");
    lines.extend(function(
        &mut t,
        "Remn",
        T_LONG,
        &[("a", T_LONG, true), ("b", T_LONG, true)],
        vec![
            Line::new()
                .read(a)
                .read(b)
                .op(Opcode::NumRemainder)
                .write(r),
        ],
    ));
    // Neg = -a - b  (5, 2 -> -7): unary minus binds before the subtraction.
    let ng = t.name("Neg");
    lines.extend(function(
        &mut t,
        "Neg",
        T_LONG,
        &[("a", T_LONG, true), ("b", T_LONG, true)],
        vec![
            Line::new()
                .read(a)
                .op(Opcode::NumNegate)
                .read(b)
                .op(Opcode::NumSubtract)
                .write(ng),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 4, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Calc.Calc", &[l(7), l(3)]), l(23));
    assert_eq!(value(&lp, "Calc.Quot", &[l(7), l(2)]), Variant::Double(3.5));
    assert_eq!(value(&lp, "Calc.Remn", &[l(7), l(2)]), l(1));
    assert_eq!(value(&lp, "Calc.Neg", &[l(5), l(2)]), l(-7));
}

#[test]
fn comparison_logic_and_text_operators() {
    let mut t = Tables::default();
    let x = t.name("x");
    let sn = t.name("s");
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Flag = x > 3 And x <> 5 Or Not x = 0        (4 -> True, 5 -> True, 0 -> False)
    let fl = t.name("Flag");
    lines.extend(function(
        &mut t,
        "Flag",
        T_BOOLEAN,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .read(x)
                .i16(3)
                .op(Opcode::CompareGreater)
                .read(x)
                .i16(5)
                .op(Opcode::CompareNotEqual)
                .op(Opcode::BoolConjunction)
                .read(x)
                .i16(0)
                .op(Opcode::CompareEqual)
                .op(Opcode::BoolNegate)
                .op(Opcode::BoolDisjunction)
                .write(fl),
        ],
    ));
    // Cat = "<" & s & ">"
    let c = t.name("Cat");
    lines.extend(function(
        &mut t,
        "Cat",
        T_STRING,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .text("<")
                .read(sn)
                .op(Opcode::TextJoin)
                .text(">")
                .op(Opcode::TextJoin)
                .write(c),
        ],
    ));
    // Lt = x <= 2 Or x >= 9
    let lt = t.name("Edge");
    lines.extend(function(
        &mut t,
        "Edge",
        T_BOOLEAN,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .read(x)
                .i16(2)
                .op(Opcode::CompareLessEqual)
                .read(x)
                .i16(9)
                .op(Opcode::CompareGreaterEqual)
                .op(Opcode::BoolDisjunction)
                .write(lt),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Flag", &[l(4)]), Variant::Boolean(true));
    // (5 > 3 And 5 <> 5) is False but Not (5 = 0) is True.
    assert_eq!(value(&lp, "Flag", &[l(5)]), Variant::Boolean(true));
    assert_eq!(value(&lp, "Flag", &[l(0)]), Variant::Boolean(false));
    assert_eq!(value(&lp, "Cat", &[s("ab")]), s("<ab>"));
    assert_eq!(value(&lp, "Edge", &[l(1)]), Variant::Boolean(true));
    assert_eq!(value(&lp, "Edge", &[l(5)]), Variant::Boolean(false));
    assert_eq!(value(&lp, "Edge", &[l(9)]), Variant::Boolean(true));
}

#[test]
fn if_elseif_else_and_single_line_if() {
    let mut t = Tables::default();
    let x = t.name("x");
    let mut lines = vec![];
    let sg = t.name("Sign3");
    lines.extend(function(
        &mut t,
        "Sign3",
        T_STRING,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .read(x)
                .i16(0)
                .op(Opcode::CompareLess)
                .op(Opcode::IfBlockOpen),
            Line::new().text("neg").write(sg),
            Line::new()
                .read(x)
                .i16(0)
                .op(Opcode::CompareEqual)
                .op(Opcode::BlockElseifBranch),
            Line::new().text("zero").write(sg),
            Line::new().op(Opcode::BlockElseBranch),
            Line::new().text("pos").write(sg),
            Line::new().op(Opcode::BlockIfClose),
        ],
    ));
    // Clamp: Clamp = x : If x > 10 Then Clamp = 10
    let cl = t.name("Clamp");
    lines.extend(function(
        &mut t,
        "Clamp",
        T_LONG,
        &[("x", T_LONG, true)],
        vec![
            Line::new().read(x).write(cl),
            Line::new()
                .read(x)
                .i16(10)
                .op(Opcode::CompareGreater)
                .op(Opcode::IfSingleLine)
                .op(Opcode::StmtBeginImplicit)
                .i16(10)
                .write(cl)
                .op(Opcode::EndifSingleLine),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Sign3", &[l(-4)]), s("neg"));
    assert_eq!(value(&lp, "Sign3", &[l(0)]), s("zero"));
    assert_eq!(value(&lp, "Sign3", &[l(8)]), s("pos"));
    assert_eq!(value(&lp, "Clamp", &[l(3)]), l(3));
    assert_eq!(value(&lp, "Clamp", &[l(30)]), l(10));
}

#[test]
fn for_next_with_step_and_exit_for() {
    let mut t = Tables::default();
    let n = t.name("n");
    let (i, sm) = (t.name("i"), t.name("s"));
    let vi = t.variable("i", Some(T_LONG));
    let vs = t.variable("s", Some(T_LONG));
    let r = t.name("SumTo");
    let for_var = |i: u16| {
        Line::new()
            .op(Opcode::ForVarExprOpen)
            .read(i)
            .op(Opcode::ForVarExprClose)
    };
    let lines = function(
        &mut t,
        "SumTo",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vi)
                .rec(Opcode::VarDecl, 1, vs),
            // For i = 1 To n Step 2
            Line(for_var(i).0)
                .i16(1)
                .read(n)
                .i16(2)
                .op(Opcode::ForCountedStep),
            // s = s + i
            Line::new().read(sm).read(i).op(Opcode::NumAdd).write(sm),
            // If s > 20 Then Exit For
            Line::new()
                .read(sm)
                .i16(20)
                .op(Opcode::CompareGreater)
                .op(Opcode::IfSingleLine)
                .op(Opcode::StmtBeginImplicit)
                .op(Opcode::LeaveFor)
                .op(Opcode::EndifSingleLine),
            // Next i
            Line(for_var(i).0).op(Opcode::ForAdvanceNamed),
            Line::new().read(sm).write(r),
        ],
    );
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    // i = 1,3,5,7,9: s = 1,4,9,16,25 -> exits after 25 > 20.
    assert_eq!(value(&lp, "SumTo", &[l(10)]), l(25));
    // n = 5: i = 1,3,5 -> 9.
    assert_eq!(value(&lp, "SumTo", &[l(5)]), l(9));
    // n = 0: the loop body never runs.
    assert_eq!(value(&lp, "SumTo", &[l(0)]), l(0));
}

#[test]
fn do_loops_and_while() {
    let mut t = Tables::default();
    let n = t.name("n");
    let c = t.name("c");
    let vc = t.variable("c", Some(T_LONG));
    let mut lines = vec![];
    // Do While n > 0 : n = n - 1 : c = c + 1 : Loop
    let r1 = t.name("CountDown");
    lines.extend(function(
        &mut t,
        "CountDown",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vc),
            Line::new()
                .read(n)
                .i16(0)
                .op(Opcode::CompareGreater)
                .op(Opcode::DoOpenWhile),
            Line::new().read(n).i16(1).op(Opcode::NumSubtract).write(n),
            Line::new().read(c).i16(1).op(Opcode::NumAdd).write(c),
            Line::new().op(Opcode::DoClose),
            Line::new().read(c).write(r1),
        ],
    ));
    // Do : c = c + 2 : Loop Until c >= n   (post-test: runs at least once)
    let vc2 = t.variable("c", Some(T_LONG));
    let r2 = t.name("AtLeastOnce");
    lines.extend(function(
        &mut t,
        "AtLeastOnce",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vc2),
            Line::new().op(Opcode::DoOpen),
            Line::new().read(c).i16(2).op(Opcode::NumAdd).write(c),
            Line::new()
                .read(c)
                .read(n)
                .op(Opcode::CompareGreaterEqual)
                .op(Opcode::DoCloseUntil),
            Line::new().read(c).write(r2),
        ],
    ));
    // While c < n : c = c + 3 : Wend
    let vc3 = t.variable("c", Some(T_LONG));
    let r3 = t.name("Wend3");
    lines.extend(function(
        &mut t,
        "Wend3",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vc3),
            Line::new()
                .read(c)
                .read(n)
                .op(Opcode::CompareLess)
                .op(Opcode::WhileOpen),
            Line::new().read(c).i16(3).op(Opcode::NumAdd).write(c),
            Line::new().op(Opcode::WhileClose),
            Line::new().read(c).write(r3),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "CountDown", &[l(4)]), l(4));
    assert_eq!(value(&lp, "CountDown", &[l(0)]), l(0));
    // c = 2, 4, 6 -> stops when c >= 5 at 6.
    assert_eq!(value(&lp, "AtLeastOnce", &[l(5)]), l(6));
    // Runs once even when the condition holds at the start.
    assert_eq!(value(&lp, "AtLeastOnce", &[l(-9)]), l(2));
    // c = 0, 3, 6 -> stops at 6 for n = 5.
    assert_eq!(value(&lp, "Wend3", &[l(5)]), l(6));
}

#[test]
fn select_case_values_ranges_and_else() {
    let mut t = Tables::default();
    let x = t.name("x");
    let g = t.name("Grade");
    let lines = function(
        &mut t,
        "Grade",
        T_STRING,
        &[("x", T_LONG, true)],
        vec![
            Line::new().read(x).op(Opcode::SwitchBegin),
            Line::new()
                .i16(1)
                .op(Opcode::CaseValue)
                .i16(2)
                .op(Opcode::CaseValue)
                .op(Opcode::CaseListEnd),
            Line::new().text("low").write(g),
            Line::new()
                .i16(3)
                .i16(5)
                .op(Opcode::CaseRange)
                .op(Opcode::CaseListEnd),
            Line::new().text("mid").write(g),
            Line::new().op(Opcode::CaseDefault),
            Line::new().text("high").write(g),
            Line::new().op(Opcode::SwitchEnd),
        ],
    );
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Grade", &[l(2)]), s("low"));
    assert_eq!(value(&lp, "Grade", &[l(3)]), s("mid"));
    assert_eq!(value(&lp, "Grade", &[l(5)]), s("mid"));
    assert_eq!(value(&lp, "Grade", &[l(6)]), s("high"));
    assert_eq!(value(&lp, "Grade", &[l(0)]), s("high"));
}

#[test]
fn calls_recursion_byref_and_statement_calls() {
    let mut t = Tables::default();
    let n = t.name("n");
    let fact = t.name("Fact");
    let mut lines = vec![];
    // Fact = IIf-less recursion: If n <= 1 Then Fact = 1 Else Fact = n * Fact(n - 1)
    lines.extend(function(
        &mut t,
        "Fact",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .read(n)
                .i16(1)
                .op(Opcode::CompareLessEqual)
                .op(Opcode::IfBlockOpen),
            Line::new().i16(1).write(fact),
            Line::new().op(Opcode::BlockElseBranch),
            Line::new()
                .read(n)
                .read(n)
                .i16(1)
                .op(Opcode::NumSubtract)
                .call(fact, 1)
                .op(Opcode::NumMultiply)
                .write(fact),
            Line::new().op(Opcode::BlockIfClose),
        ],
    ));
    // Sub Bump(ByRef v As Long): v = v + 1
    let v = t.name("v");
    let bump = t.function("Bump", None, &[("v", T_LONG, false)]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, bump));
    lines.push(Line::new().read(v).i16(1).op(Opcode::NumAdd).write(v));
    lines.push(Line::new().op(Opcode::ProcEndSub));
    // Function UseBump(x As Long): Dim y As Long: y = x: Bump y: UseBump = y
    let x = t.name("x");
    let y = t.name("y");
    let vy = t.variable("y", Some(T_LONG));
    let ub = t.name("UseBump");
    let bump_name = t.name("Bump");
    lines.extend(function(
        &mut t,
        "UseBump",
        T_LONG,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vy),
            Line::new().read(x).write(y),
            Line::new()
                .read(y)
                .name_args(Opcode::InvokeSimple, 16, bump_name, 1),
            Line::new().read(y).write(ub),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Fact", &[l(5)]), l(120));
    assert_eq!(value(&lp, "Fact", &[l(1)]), l(1));
    assert_eq!(value(&lp, "UseBump", &[l(41)]), l(42));
}

#[test]
fn debug_print_and_library_functions() {
    let mut t = Tables::default();
    let sn = t.name("s");
    let mut lines = vec![];
    // Sub P: Debug.Print "hi"; 1 + 2
    let p = t.function("P", None, &[]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, p));
    lines.push(
        Line::new()
            .op(Opcode::DebugPrefix)
            .op(Opcode::OutTargetObject)
            .text("hi")
            .op(Opcode::OutItemThenSemicolon)
            .i16(1)
            .i16(2)
            .op(Opcode::NumAdd)
            .op(Opcode::OutItemLast),
    );
    lines.push(Line::new().op(Opcode::ProcEndSub));
    // Function L(s As String) As Long: L = Len(s)
    let ln = t.name("L");
    lines.extend(function(
        &mut t,
        "L",
        T_LONG,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .read(sn)
                .op(Opcode::IntrinsicLengthChars)
                .write(ln),
        ],
    ));
    // Function U(s As String) As String: U = UCase(s) & Left(s, 1)
    let un = t.name("U");
    let ucase = t.name("UCase");
    let left = t.name("Left");
    lines.extend(function(
        &mut t,
        "U",
        T_STRING,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .read(sn)
                .call(ucase, 1)
                .read(sn)
                .i16(1)
                .call(left, 2)
                .op(Opcode::TextJoin)
                .write(un),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    let (r, out) = run(&lp, "P", &[]);
    r.expect("runs");
    assert_eq!(out, "hi 3 \n");
    assert_eq!(value(&lp, "L", &[s("hello")]), l(5));
    assert_eq!(value(&lp, "U", &[s("ab")]), s("ABa"));
}

#[test]
fn remaining_verified_families() {
    let mut t = Tables::default();
    let x = t.name("x");
    let sn = t.name("s");
    let n = t.name("n");
    let mut lines = vec![];
    // Mix = Abs(-x) + Int(x / 2) + Sgn(0 - x)          (5 -> 5 + 2 - 1 = 6)
    let mix = t.name("Mix");
    lines.extend(function(
        &mut t,
        "Mix",
        T_DOUBLE,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .read(x)
                .op(Opcode::NumNegate)
                .op(Opcode::IntrinsicAbsoluteValue)
                .read(x)
                .i16(2)
                .op(Opcode::NumDivide)
                .op(Opcode::IntrinsicFloor)
                .op(Opcode::NumAdd)
                .i16(0)
                .read(x)
                .op(Opcode::NumSubtract)
                .op(Opcode::IntrinsicSign)
                .op(Opcode::NumAdd)
                .write(mix),
        ],
    ));
    // Pos2 = InStr(s, "b"); Pos4 = InStr(1, s, "b", 0)         ("abc" -> 2)
    let p2 = t.name("Pos2");
    lines.extend(function(
        &mut t,
        "Pos2",
        T_LONG,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .read(sn)
                .text("b")
                .op(Opcode::IntrinsicSearchA)
                .write(p2),
        ],
    ));
    let p4 = t.name("Pos4");
    lines.extend(function(
        &mut t,
        "Pos4",
        T_LONG,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .i16(1)
                .read(sn)
                .text("b")
                .i16(0)
                .op(Opcode::IntrinsicSearchC)
                .write(p4),
        ],
    ));
    // Pat = s Like "a*"
    let pat = t.name("Pat");
    lines.extend(function(
        &mut t,
        "Pat",
        T_BOOLEAN,
        &[("s", T_STRING, true)],
        vec![
            Line::new()
                .read(sn)
                .text("a*")
                .op(Opcode::ComparePattern)
                .write(pat),
        ],
    ));
    // K = 2.5# + 100000 and T = True
    let k = t.name("K");
    lines.extend(function(
        &mut t,
        "K",
        T_DOUBLE,
        &[],
        vec![
            Line::new()
                .f64(2.5)
                .i32(100_000)
                .op(Opcode::NumAdd)
                .write(k),
        ],
    ));
    let tt = t.name("T");
    lines.extend(function(
        &mut t,
        "T",
        T_BOOLEAN,
        &[],
        vec![Line::new().opm(Opcode::ConstKeywordValue, 1).write(tt)],
    ));
    // FirstOver: For i = 1 To 100 : If i * i > n Then : FirstOver = i : Exit Function
    //            End If : Next : FirstOver = -1
    let i = t.name("i");
    let vi = t.variable("i", Some(T_LONG));
    let fo = t.name("FirstOver");
    let for_var = |i: u16| {
        Line::new()
            .op(Opcode::ForVarExprOpen)
            .read(i)
            .op(Opcode::ForVarExprClose)
    };
    lines.extend(function(
        &mut t,
        "FirstOver",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vi),
            Line(for_var(i).0).i16(1).i16(100).op(Opcode::ForCounted),
            Line::new()
                .read(i)
                .read(i)
                .op(Opcode::NumMultiply)
                .read(n)
                .op(Opcode::CompareGreater)
                .op(Opcode::IfBlockOpen),
            Line::new().read(i).write(fo),
            Line::new().op(Opcode::LeaveFunction),
            Line::new().op(Opcode::BlockIfClose),
            Line(for_var(i).0).op(Opcode::ForAdvanceNamed),
            Line::new().i16(1).op(Opcode::NumNegate).write(fo),
        ],
    ));
    // UntilLoop: Do Until i >= n : i = i + 1 : Loop        (n = 3 -> 3)
    let vi2 = t.variable("i", Some(T_LONG));
    let ul = t.name("UntilLoop");
    lines.extend(function(
        &mut t,
        "UntilLoop",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vi2),
            Line::new()
                .read(i)
                .read(n)
                .op(Opcode::CompareGreaterEqual)
                .op(Opcode::DoOpenUntil),
            Line::new().read(i).i16(1).op(Opcode::NumAdd).write(i),
            Line::new().op(Opcode::DoClose),
            Line::new().read(i).write(ul),
        ],
    ));
    // WhileLoop: Do : i = i + 1 : Loop While i < n          (n = 5 -> 5, n = 0 -> 1)
    let vi3 = t.variable("i", Some(T_LONG));
    let wl = t.name("WhileLoop");
    lines.extend(function(
        &mut t,
        "WhileLoop",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vi3),
            Line::new().op(Opcode::DoOpen),
            Line::new().read(i).i16(1).op(Opcode::NumAdd).write(i),
            Line::new()
                .read(i)
                .read(n)
                .op(Opcode::CompareLess)
                .op(Opcode::DoCloseWhile),
            Line::new().read(i).write(wl),
        ],
    ));
    // Sub Bump(ByRef v As Long) and a caller that uses the Call keyword.
    let v = t.name("v");
    let bump = t.function("Bump", None, &[("v", T_LONG, false)]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, bump));
    lines.push(Line::new().read(v).i16(1).op(Opcode::NumAdd).write(v));
    lines.push(Line::new().op(Opcode::ProcEndSub));
    let y = t.name("y");
    let vy = t.variable("y", Some(T_LONG));
    let ub = t.name("UseCall");
    let bn = t.name("Bump");
    lines.extend(function(
        &mut t,
        "UseCall",
        T_LONG,
        &[("x", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vy),
            Line::new().read(x).write(y),
            // Call Bump(y)
            Line::new()
                .read(y)
                .name_args(Opcode::InvokeSimple, 0, bn, 1),
            Line::new().read(y).write(ub),
        ],
    ));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 11, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Mix", &[l(5)]), Variant::Double(6.0));
    assert_eq!(value(&lp, "Pos2", &[s("abc")]), l(2));
    assert_eq!(value(&lp, "Pos4", &[s("abc")]), l(2));
    assert_eq!(value(&lp, "Pat", &[s("abc")]), Variant::Boolean(true));
    assert_eq!(value(&lp, "Pat", &[s("bca")]), Variant::Boolean(false));
    assert_eq!(value(&lp, "K", &[]), Variant::Double(100_002.5));
    assert_eq!(value(&lp, "T", &[]), Variant::Boolean(true));
    assert_eq!(value(&lp, "FirstOver", &[l(10)]), l(4));
    assert_eq!(value(&lp, "FirstOver", &[l(100_000)]), l(-1));
    assert_eq!(value(&lp, "UntilLoop", &[l(3)]), l(3));
    assert_eq!(value(&lp, "WhileLoop", &[l(5)]), l(5));
    assert_eq!(value(&lp, "WhileLoop", &[l(0)]), l(1));
    assert_eq!(value(&lp, "UseCall", &[l(9)]), l(10));
}

#[test]
fn typed_integer_arithmetic_overflows_like_the_language() {
    let mut t = Tables::default();
    let a = t.name("a");
    let r = t.name("Sq");
    let lines = function(
        &mut t,
        "Sq",
        T_INTEGER,
        &[("a", T_INTEGER, true)],
        vec![Line::new().read(a).read(a).op(Opcode::NumMultiply).write(r)],
    );
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 1);
    assert_eq!(
        value(&lp, "Sq", &[Variant::Integer(12)]),
        Variant::Integer(144)
    );
    let (res, _) = run(&lp, "Sq", &[Variant::Integer(300)]);
    let err = res.expect_err("Integer * Integer overflows");
    assert!(format!("{err:?}").contains('6'), "{err:?}");
}

#[test]
fn unsupported_constructs_are_reported_with_opcode_procedure_and_line() {
    let mut t = Tables::default();
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Sub E: On Error GoTo <label>   (only the Resume Next form is lowered)
    let e = t.function("E", None, &[]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, e));
    lines.push(on_error(0));
    lines.push(Line::new().op(Opcode::ProcEndSub));
    // Sub Caller calls E and therefore cannot be lowered either.
    let caller = t.function("Caller", None, &[]);
    let en = t.name("E");
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, caller));
    lines.push(Line::new().name_args(Opcode::InvokeSimple, 16, en, 0));
    lines.push(Line::new().op(Opcode::ProcEndSub));
    // Sub Fine does lower.
    let fine = t.function("Fine", None, &[]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 5, fine));
    lines.push(Line::new().op(Opcode::ProcEndSub));
    let lp = lower(&t, &lines);
    let rep = lp.report("Calc", "E").expect("report");
    match &rep.outcome {
        Outcome::Unsupported(u) => {
            assert_eq!(u.opcode, Opcode::ErrHandlerSet);
            assert_eq!(u.procedure, "E");
            assert_eq!(u.line, 2);
        }
        o => panic!("expected unsupported, got {o:?}"),
    }
    assert!(matches!(
        lp.report("Calc", "Caller").map(|r| &r.outcome),
        Some(Outcome::Unsupported(_))
    ));
    assert!(
        lp.report("Calc", "Fine")
            .is_some_and(|r| r.outcome.is_lowered())
    );
}

#[test]
fn optional_parameters_with_and_without_defaults() {
    let mut t = Tables::default();
    let (a, b, v, sn) = (t.name("a"), t.name("b"), t.name("v"), t.name("s"));
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Function Opt(a As Long, Optional b As Long = 7) As Long: Opt = a + b
    let opt = t.name("Opt");
    let f = t.function(
        "Opt",
        Some(T_LONG),
        &[("a", T_LONG, true), ("b", T_LONG, true)],
    );
    t.mark_optional(f, 1, true);
    lines.push(
        Line::new()
            .op(Opcode::ConstExprWithCall)
            .i16(7)
            .rec(Opcode::ProcHeader, 6, f),
    );
    lines.push(Line::new().read(a).read(b).op(Opcode::NumAdd).write(opt));
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    // Function Greet(Optional s As String = "x") As String: Greet = "<" & s
    let gr = t.name("Greet");
    let f = t.function("Greet", Some(T_STRING), &[("s", T_STRING, true)]);
    t.mark_optional(f, 0, true);
    lines.push(
        Line::new()
            .op(Opcode::ConstExprWithCall)
            .text("x")
            .rec(Opcode::ProcHeader, 6, f),
    );
    lines.push(
        Line::new()
            .text("<")
            .read(sn)
            .op(Opcode::TextJoin)
            .write(gr),
    );
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    // Function Miss(Optional v As Variant) As Boolean: Miss = IsMissing(v)
    let mi = t.name("Miss");
    let ism = t.name("IsMissing");
    let f = t.function("Miss", Some(T_BOOLEAN), &[("v", T_VARIANT, true)]);
    t.mark_optional(f, 0, false);
    lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
    lines.push(Line::new().read(v).call(ism, 1).write(mi));
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    // Function Use() As Long: Use = Opt(1)     (b omitted, so 1 + 7)
    let us = t.name("Use");
    let opn = t.name("Opt");
    let f = t.function("Use", Some(T_LONG), &[]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
    lines.push(Line::new().i16(1).call(opn, 1).write(us));
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 4, "{:?}", lp.procedures);
    let run_args = |entry: &str, args: Vec<Arg>| {
        let mut vm = Vm::new(
            Rc::new(lp.ir.clone()),
            Rc::new(HeadlessHost::new()),
            Box::new(NoServices),
            Box::new(vba_stdlib::FixedHost::default()),
            Box::new(BufferSink::new()),
            VmConfig::default(),
        )
        .expect("vm");
        let p = vm.find_procedure(entry).expect("entry");
        vm.run_procedure(p, args).expect("runs").value
    };
    assert_eq!(
        run_args("Calc.Opt", vec![Arg::Value(l(1)), Arg::Missing]),
        l(8)
    );
    assert_eq!(
        run_args("Calc.Opt", vec![Arg::Value(l(1)), Arg::Value(l(2))]),
        l(3)
    );
    assert_eq!(run_args("Calc.Greet", vec![Arg::Missing]), s("<x"));
    assert_eq!(run_args("Calc.Greet", vec![Arg::Value(s("y"))]), s("<y"));
    assert_eq!(
        run_args("Calc.Miss", vec![Arg::Missing]),
        Variant::Boolean(true)
    );
    assert_eq!(
        run_args("Calc.Miss", vec![Arg::Value(l(3))]),
        Variant::Boolean(false)
    );
    assert_eq!(run_args("Calc.Use", vec![]), l(8));
}

/// `On Error ...` with the given modifier (1 = Resume Next) and no target.
fn on_error(modifier: u16) -> Line {
    let mut l = Line::new().opm(Opcode::ErrHandlerSet, modifier);
    l.0.extend_from_slice(&0u16.to_le_bytes());
    l
}

#[test]
fn on_error_resume_next_continues_after_the_failing_statement() {
    let mut t = Tables::default();
    let a = t.name("a");
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Function R(a): On Error Resume Next: R = 5: R = a \ 0: R = a + 1
    let r = t.name("R");
    let f = t.function("R", Some(T_LONG), &[("a", T_LONG, true)]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
    lines.push(on_error(1));
    lines.push(Line::new().i16(5).write(r));
    lines.push(Line::new().read(a).i16(0).op(Opcode::NumIntDivide).write(r));
    lines.push(Line::new().read(a).i16(1).op(Opcode::NumAdd).write(r));
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    // Function Q(a): On Error Resume Next
    //   If 1 \ a > 0 Then Q = 1 Else Q = 2     (the failing condition
    //   continues in the Then branch, as the source lowering does)
    let q = t.name("Q");
    let f = t.function("Q", Some(T_LONG), &[("a", T_LONG, true)]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
    lines.push(on_error(1));
    lines.push(
        Line::new()
            .i16(1)
            .read(a)
            .op(Opcode::NumIntDivide)
            .i16(0)
            .op(Opcode::CompareGreater)
            .op(Opcode::IfBlockOpen),
    );
    lines.push(Line::new().i16(1).write(q));
    lines.push(Line::new().op(Opcode::BlockElseBranch));
    lines.push(Line::new().i16(2).write(q));
    lines.push(Line::new().op(Opcode::BlockIfClose));
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    // Function P(a) without a handler: the error propagates.
    let pn = t.name("P");
    let f = t.function("P", Some(T_LONG), &[("a", T_LONG, true)]);
    lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
    lines.push(
        Line::new()
            .read(a)
            .i16(0)
            .op(Opcode::NumIntDivide)
            .write(pn),
    );
    lines.push(Line::new().op(Opcode::ProcEndFunction));
    let lp = lower(&t, &lines);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    assert_eq!(value(&lp, "Calc.R", &[l(9)]), l(10));
    assert_eq!(value(&lp, "Calc.Q", &[l(0)]), l(1));
    assert_eq!(value(&lp, "Calc.Q", &[l(1)]), l(1));
    assert_eq!(value(&lp, "Calc.Q", &[l(-1)]), l(2));
    let (res, _) = run(&lp, "Calc.P", &[l(3)]);
    assert!(res.is_err(), "no handler: division by zero propagates");
}

#[test]
fn hostile_caches_never_panic() {
    use proptest::prelude::*;
    let mut t = Tables::default();
    let x = t.name("x");
    let r = t.name("F");
    let lines = function(
        &mut t,
        "F",
        T_LONG,
        &[("x", T_LONG, true)],
        vec![Line::new().read(x).i16(1).op(Opcode::NumAdd).write(r)],
    );
    let project = build_project_stream(&t.idents);
    let cache = build_cache(&lines, &t.ind);
    let mut runner = proptest::test_runner::TestRunner::default();
    runner
        .run(
            &(
                proptest::collection::vec((0..cache.len(), any::<u8>()), 0..12),
                0..cache.len() + 8,
            ),
            |(edits, cut)| {
                let mut c = cache.clone();
                for (at, b) in edits {
                    c[at] = b;
                }
                c.truncate(cut.max(1));
                let _ = lower_project(
                    &project,
                    1,
                    &[ModuleInput {
                        name: "Calc",
                        class_like: false,
                        cache: &c,
                    }],
                );
                Ok(())
            },
        )
        .expect("no panic");
}
