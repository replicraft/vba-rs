//! Hand-assembled caches for the constructs of the second round of local
//! material: local arrays (element access, `UBound`/`LBound`, `ReDim`),
//! implicit variables, `Resume` forms, string literals in the project code
//! page, file statements and `Declare`. Each result is worked out by hand
//! from the language rules.
#![allow(missing_docs)]

mod common;

use std::rc::Rc;

use common::synth::*;
use vba_host::HeadlessHost;
use vba_pcode::Opcode;
use vba_pcode_ir::{LowerOptions, LoweredProject, ModuleInput, lower_project_with_options};
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, RunOutput, Vm, VmConfig, VmError};

fn lower_with(t: &Tables, lines: &[Line], options: &LowerOptions) -> LoweredProject {
    let project = build_project_stream(&t.idents);
    let cache = build_cache_t(lines, t);
    let lp = lower_project_with_options(
        &project,
        1,
        &[ModuleInput {
            name: "Calc",
            kind: vba_project::ModuleKind::Standard,
            cache: &cache,
        }],
        options,
    )
    .expect("lowers");
    vba_ir::verify(&lp.ir).expect("verified");
    lp
}

fn lower(t: &Tables, lines: &[Line]) -> LoweredProject {
    lower_with(t, lines, &LowerOptions::default())
}

fn run(lp: &LoweredProject, entry: &str, args: &[Variant]) -> Result<RunOutput, VmError> {
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
    vm.run_procedure(p, args.iter().cloned().map(Arg::Value).collect())
}

fn value(lp: &LoweredProject, entry: &str, args: &[Variant]) -> Variant {
    run(lp, entry, args).expect("runs").value
}

fn l(v: i32) -> Variant {
    Variant::Long(v)
}

/// `Function name(params) As ret`, the body lines and `End Function`.
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
fn fixed_and_dynamic_local_arrays() {
    let mut t = Tables::default();
    let (fixed, dynamic) = (
        t.array_variable("fx", T_LONG),
        t.array_variable("dy", T_LONG),
    );
    let (fx, dy) = (t.name("fx"), t.name("dy"));
    let calc = t.name("Calc");
    let lines = function(
        &mut t,
        "Calc",
        T_LONG,
        &[],
        vec![
            // Dim fx(1 To 3) As Long
            Line::new()
                .op(Opcode::DimMarker)
                .i16(1)
                .i16(3)
                .rec(Opcode::VarDecl, 1, fixed),
            // Dim dy() As Long
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, dynamic),
            // fx(2) = 5
            Line::new()
                .i16(5)
                .i16(2)
                .name_args(Opcode::WriteSimpleParams, 0, fx, 1),
            // ReDim dy(2): the lower bound is the Option Base one.
            Line::new()
                .op(Opcode::OptionBaseMarker)
                .i16(2)
                .name_args_rec(Opcode::RedimSimple, 0, dy, 1, 0),
            // dy(0) = 7
            Line::new()
                .i16(7)
                .i16(0)
                .name_args(Opcode::WriteSimpleParams, 0, dy, 1),
            // ReDim Preserve dy(4)
            Line::new()
                .op(Opcode::OptionBaseMarker)
                .i16(4)
                .name_args_rec(Opcode::RedimSimple, 16, dy, 1, 0),
            // Calc = fx(2) + UBound(dy) * 100 + dy(0) * 10000 + LBound(fx) * 1000000
            Line::new()
                .i16(2)
                .call(fx, 1)
                .read(dy)
                .opw(Opcode::IntrinsicUpperBound, 0, 0)
                .i16(100)
                .op(Opcode::NumMultiply)
                .op(Opcode::NumAdd)
                .i16(0)
                .call(dy, 1)
                .i32(10000)
                .op(Opcode::NumMultiply)
                .op(Opcode::NumAdd)
                .read(fx)
                .opw(Opcode::IntrinsicLowerBound, 0, 0)
                .i32(1_000_000)
                .op(Opcode::NumMultiply)
                .op(Opcode::NumAdd)
                .write(calc),
        ],
    );
    let lp = lower(&t, &lines);
    assert!(lp.report("Calc", "Calc").unwrap().outcome.is_lowered());
    // 5 + 4 * 100 + 7 * 10000 + 1 * 1000000
    assert_eq!(value(&lp, "Calc", &[]), l(1_070_405));
}

#[test]
fn array_declared_with_bounds_that_are_not_constants_is_refused() {
    let mut t = Tables::default();
    let n = t.name("n");
    let a = t.array_variable("a", T_LONG);
    let lines = function(
        &mut t,
        "Calc",
        T_LONG,
        &[("n", T_LONG, true)],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .op(Opcode::OptionBaseMarker)
                .read(n)
                .op(Opcode::NumNegate)
                .rec(Opcode::VarDecl, 1, a),
        ],
    );
    let lp = lower(&t, &lines);
    // The bound is `-n`: neither a literal nor a constant name.
    assert!(!lp.report("Calc", "Calc").unwrap().outcome.is_lowered());
}

#[test]
fn an_assignment_declares_an_implicit_variable() {
    let mut t = Tables::default();
    let (x, calc) = (t.name("x"), t.name("Calc"));
    let lines = function(
        &mut t,
        "Calc",
        T_LONG,
        &[],
        vec![
            Line::new().i16(41).write(x),
            Line::new().read(x).i16(1).op(Opcode::NumAdd).write(calc),
        ],
    );
    let lp = lower(&t, &lines);
    assert!(lp.report("Calc", "Calc").unwrap().outcome.is_lowered());
    assert_eq!(value(&lp, "Calc", &[]), l(42));
    // Under `Option Explicit` the name is not declared by the assignment.
    let mut t2 = Tables::default();
    let (x2, calc2) = (t2.name("x"), t2.name("Calc"));
    let mut lines2 = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    lines2.extend(function(
        &mut t2,
        "Calc",
        T_LONG,
        &[],
        vec![
            Line::new().i16(41).write(x2),
            Line::new().read(x2).write(calc2),
        ],
    ));
    let lp2 = lower(&t2, &lines2);
    assert!(!lp2.report("Calc", "Calc").unwrap().outcome.is_lowered());
}

/// `Resume` (modifier 8) or `Resume Next` (1): the name operand is 0.
fn resume_form(modifier: u16) -> Line {
    Line::new().opw(Opcode::ErrResume, modifier, 0)
}

#[test]
fn resume_and_resume_next() {
    let mut t = Tables::default();
    let (z, calc, h) = (t.name("z"), t.name("Calc"), t.name("H"));
    let mut lines = Vec::new();
    // Function Calc(z) : On Error GoTo H : Calc = 100 \ z ... handler: z = 4 : Resume
    lines.extend(function(
        &mut t,
        "Calc",
        T_LONG,
        &[("z", T_LONG, false)],
        vec![
            Line::new().name(Opcode::ErrHandlerSet, h),
            Line::new()
                .i16(100)
                .read(z)
                .op(Opcode::NumIntDivide)
                .write(calc),
            Line::new().op(Opcode::LeaveFunction),
            Line::new().name(Opcode::LabelNamed, h),
            Line::new().i16(4).write(z),
            resume_form(8),
        ],
    ));
    // Function Skip(z): On Error GoTo H : Skip = 100 \ z : Skip = Skip + 1 ...
    let (skip, h2) = (t.name("Skip"), t.name("H2"));
    lines.extend(function(
        &mut t,
        "Skip",
        T_LONG,
        &[("z", T_LONG, true)],
        vec![
            Line::new().name(Opcode::ErrHandlerSet, h2),
            Line::new()
                .i16(100)
                .read(z)
                .op(Opcode::NumIntDivide)
                .write(skip),
            Line::new().i16(7).write(skip),
            Line::new().op(Opcode::LeaveFunction),
            Line::new().name(Opcode::LabelNamed, h2),
            resume_form(1),
        ],
    ));
    let lp = lower(&t, &lines);
    let rep = lp.report("Calc", "Calc").unwrap();
    assert!(rep.outcome.is_lowered(), "{:?}", rep.outcome);
    let rep = lp.report("Calc", "Skip").unwrap();
    assert!(rep.outcome.is_lowered(), "{:?}", rep.outcome);
    // Division by zero, the handler sets z to 4 and the statement runs again.
    assert_eq!(value(&lp, "Calc", &[l(0)]), l(25));
    assert_eq!(value(&lp, "Calc", &[l(5)]), l(20));
    // `Resume Next` skips the failing statement.
    assert_eq!(value(&lp, "Skip", &[l(0)]), l(7));
}

#[test]
fn text_literals_use_the_project_code_page() {
    let mut t = Tables::default();
    let calc = t.name("Calc");
    // The byte 0xE9 is "e acute" in code page 1252 and in no ASCII.
    let lines = function(
        &mut t,
        "Calc",
        T_STRING,
        &[],
        vec![Line::new().text_bytes(b"caf\xE9").write(calc)],
    );
    let none = lower(&t, &lines);
    assert!(!none.report("Calc", "Calc").unwrap().outcome.is_lowered());
    let options = LowerOptions {
        code_page: Some(1252),
        ..LowerOptions::default()
    };
    let lp = lower_with(&t, &lines, &options);
    assert!(lp.report("Calc", "Calc").unwrap().outcome.is_lowered());
    assert_eq!(value(&lp, "Calc", &[]), Variant::String("caf\u{e9}".into()));
    // A byte that code page 1250 maps to another letter.
    let options = LowerOptions {
        code_page: Some(1250),
        ..LowerOptions::default()
    };
    let lp = lower_with(&t, &lines, &options);
    assert_eq!(value(&lp, "Calc", &[]), Variant::String("caf\u{e9}".into()));
}

#[test]
fn declare_statements_and_calls_to_them() {
    let mut t = Tables::default();
    let (calc, beep) = (t.name("Calc"), t.name("Twice"));
    let d = t.declare(
        "Twice",
        "mylib",
        "TwiceA",
        Some(T_LONG),
        &[("n", T_LONG, true)],
    );
    let mut lines = vec![Line::new().rec(Opcode::ProcHeader, 6, d)];
    lines.extend(function(
        &mut t,
        "Calc",
        T_LONG,
        &[],
        vec![Line::new().i16(3).call(beep, 1).write(calc)],
    ));
    let lp = lower(&t, &lines);
    let rep = lp.report("Calc", "Twice").expect("declare is reported");
    assert!(rep.is_declare && rep.outcome.is_lowered(), "{rep:?}");
    assert!(lp.report("Calc", "Calc").unwrap().outcome.is_lowered());
    // The library is not available to the headless run: an error, not a
    // value, and not a panic.
    assert!(run(&lp, "Calc", &[]).is_err());
}

#[test]
fn open_print_and_close_lower_to_file_statements() {
    let mut t = Tables::default();
    let (f, calc) = (t.name("f"), t.name("Calc"));
    let vf = t.variable("f", Some(T_INTEGER));
    let lines = function(
        &mut t,
        "Calc",
        T_LONG,
        &[],
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, vf),
            // f = 1 : Open "x.txt" For Output As #f
            Line::new().i16(1).write(f),
            Line::new()
                .text("x.txt")
                .read(f)
                .op(Opcode::ExprFileNumberMark)
                .op(Opcode::ArgDefaultPlaceholder)
                .opw(Opcode::FileOpen, 0, 2),
            // Print #f, "hi"
            Line::new()
                .read(f)
                .op(Opcode::ExprFileNumberMark)
                .op(Opcode::OutChannel)
                .text("hi")
                .op(Opcode::OutItemLast),
            // Close #f
            Line::new()
                .read(f)
                .op(Opcode::ExprFileNumberMark)
                .opw(Opcode::FileClose, 0, 1),
            Line::new().i16(1).write(calc),
        ],
    );
    let lp = lower(&t, &lines);
    let rep = lp.report("Calc", "Calc").unwrap();
    assert!(rep.outcome.is_lowered(), "{:?}", rep.outcome);
    // The headless run has no file service: the statement fails.
    assert!(run(&lp, "Calc", &[]).is_err());
}
