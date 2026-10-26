//! Lowering and execution of hand-assembled caches of class-like modules and
//! of module-level declarations (no local project needed).
//!
//! Every test assembles instruction streams from the documented layouts,
//! lowers them with `vba-pcode-ir`, runs the result on `vba-vm` and compares
//! with an expectation worked out by hand from the VBA semantics.
#![allow(missing_docs)]

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::host::HostMock;
use common::synth::*;
use vba_host::HeadlessHost;
use vba_pcode::Opcode;
use vba_pcode_ir::{
    LowerOptions, LoweredProject, ModuleInput, Outcome, lower_project_with_options,
};
use vba_project::ModuleKind;
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, Vm, VmConfig};

/// The name word of the implicit `Me`.
const ME: u16 = 0xFFFF;

fn lower_kind(
    t: &Tables,
    kind: ModuleKind,
    lines: &[Line],
    host_members: &[&str],
) -> LoweredProject {
    let project = build_project_stream(&t.idents);
    let cache = build_cache(lines, &t.ind);
    let options = LowerOptions {
        host_members: host_members.iter().map(|s| (*s).to_owned()).collect(),
    };
    let lp = lower_project_with_options(
        &project,
        1,
        &[ModuleInput {
            name: "Cls",
            kind,
            cache: &cache,
        }],
        &options,
    )
    .expect("lowers");
    vba_ir::verify(&lp.ir).expect("verified");
    lp
}

fn new_vm(lp: &LoweredProject) -> (Vm, BufferSink) {
    let sink = BufferSink::new();
    let vm = Vm::new(
        Rc::new(lp.ir.clone()),
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    (vm, sink)
}

/// The procedure of the module `Cls` called `name` (the `k`-th in order of
/// the same name: property procedures share one).
fn proc_id(lp: &LoweredProject, name: &str, k: usize) -> vba_sema::ProcId {
    lp.procedures
        .iter()
        .filter(|p| p.procedure == name)
        .nth(k)
        .and_then(|p| p.id)
        .expect("procedure")
}

fn l(v: i32) -> Variant {
    Variant::Long(v)
}

fn s(v: &str) -> Variant {
    Variant::String(v.into())
}

fn reasons(lp: &LoweredProject, name: &str) -> String {
    match &lp.report("Cls", name).expect("report").outcome {
        Outcome::Unsupported(u) => u.reason.clone(),
        Outcome::Lowered => String::new(),
    }
}

/// A procedure: header, body lines, end instruction.
fn block(rec: u32, header_mod: u16, end: Opcode, body: Vec<Line>) -> Vec<Line> {
    let mut v = vec![Line::new().rec(Opcode::ProcHeader, header_mod, rec)];
    v.extend(body);
    v.push(Line::new().op(end));
    v
}

#[test]
fn module_level_variables_and_constants_of_a_standard_module() {
    let mut t = Tables::default();
    let (g, k, n, bump) = (t.name("g"), t.name("K"), t.name("n"), t.name("Bump"));
    let gv = t.variable("g", Some(T_LONG));
    let kv = t.variable("K", None);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        // Private g As Long
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, gv),
        // Private Const K = 5
        Line::new()
            .opm(Opcode::DimMarker, 17)
            .i16(5)
            .rec(Opcode::VarDecl, 2, kv),
    ];
    // Function Bump(n As Long) As Long: g = g + n + K: Bump = g
    let f = t.function("Bump", Some(T_LONG), &[("n", T_LONG, true)]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .read(g)
                .read(n)
                .op(Opcode::NumAdd)
                .read(k)
                .op(Opcode::NumAdd)
                .write(g),
            Line::new().read(g).write(bump),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    assert_eq!(lp.declarations.len(), 2);
    assert!(lp.declarations.iter().all(|d| d.unsupported.is_none()));
    let (mut vm, _) = new_vm(&lp);
    let p = vm.find_procedure("Cls.Bump").expect("entry");
    // The variable keeps its value between calls: 0+1+5, then 6+1+5.
    for want in [6, 12] {
        let r = vm
            .run_procedure(p, vec![Arg::Value(l(1))])
            .expect("runs")
            .value;
        assert_eq!(r, l(want));
    }
}

#[test]
fn constants_keep_their_literal_types_and_unsupported_declarations_are_left_out() {
    let mut t = Tables::default();
    let (c1, c2, c3, c4) = (t.name("C1"), t.name("C2"), t.name("C3"), t.name("C4"));
    let (r1, r2, r3, r4) = (t.name("R1"), t.name("R2"), t.name("R3"), t.name("R4"));
    let v1 = t.variable("C1", None);
    let v2 = t.variable("C2", None);
    let v3 = t.variable("C3", Some(T_DOUBLE));
    // A constant whose value is not a single literal is left out.
    let v4 = t.variable("C4", None);
    let mut lines = vec![
        // Const C1 = &H7FFF (Integer), C2 = 5& (Long), C3 As Double = 2, C4 = 1 + 2
        Line::new()
            .opm(Opcode::DimMarker, 1)
            .hex16(0x7FFF)
            .rec(Opcode::VarDecl, 2, v1),
        Line::new()
            .opm(Opcode::DimMarker, 1)
            .i32(5)
            .rec(Opcode::VarDecl, 2, v2),
        Line::new()
            .opm(Opcode::DimMarker, 1)
            .i16(2)
            .rec(Opcode::VarDecl, 2, v3),
        Line::new()
            .opm(Opcode::DimMarker, 1)
            .i16(1)
            .i16(2)
            .op(Opcode::NumAdd)
            .rec(Opcode::VarDecl, 2, v4),
    ];
    for (name, c, ret) in [
        ("R1", c1, T_VARIANT),
        ("R2", c2, T_VARIANT),
        ("R3", c3, T_VARIANT),
        ("R4", c4, T_VARIANT),
    ] {
        let target = match name {
            "R1" => r1,
            "R2" => r2,
            "R3" => r3,
            _ => r4,
        };
        let f = t.function(name, Some(ret), &[]);
        lines.extend(block(
            f,
            6,
            Opcode::ProcEndFunction,
            vec![Line::new().read(c).write(target)],
        ));
    }
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    let unsupported: Vec<_> = lp
        .declarations
        .iter()
        .filter(|d| d.unsupported.is_some())
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(unsupported, ["C4"]);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    assert!(reasons(&lp, "R4").contains("not a local"));
    let (mut vm, _) = new_vm(&lp);
    let mut val = |name: &str| {
        let p = vm.find_procedure(&format!("Cls.{name}")).expect("entry");
        vm.run_procedure(p, vec![]).expect("runs").value
    };
    assert_eq!(val("R1"), Variant::Integer(0x7FFF));
    assert_eq!(val("R2"), l(5));
    assert_eq!(val("R3"), Variant::Double(2.0));
}

/// A class with a field, `Class_Initialize`, a property pair, a method that
/// calls them on `Me` and reads a field through `Me`.
fn counter_class() -> (Tables, Vec<Line>) {
    let mut t = Tables::default();
    let (m, a, v, add, x, init) = (
        t.name("m_a"),
        t.name("A"),
        t.name("v"),
        t.name("Add"),
        t.name("x"),
        t.name("Class_Initialize"),
    );
    let mv = t.variable("m_a", Some(T_LONG));
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, mv),
    ];
    // Private Sub Class_Initialize(): m_a = 10
    let f = t.function("Class_Initialize", None, &[]);
    lines.extend(block(
        f,
        1,
        Opcode::ProcEndSub,
        vec![Line::new().i16(10).write(m)],
    ));
    let _ = init;
    // Public Property Get A() As Long: A = m_a
    let g = t.procedure("A", KIND_PROPERTY_GET, Some(T_LONG), &[], Some(T_LONG));
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndProperty,
        vec![Line::new().read(m).write(a)],
    ));
    // Public Property Let A(v As Long): m_a = v
    let p = t.procedure("A", KIND_PROPERTY_LET, None, &[("v", T_LONG, false)], None);
    lines.extend(block(
        p,
        6,
        Opcode::ProcEndProperty,
        vec![Line::new().read(v).write(m)],
    ));
    // Public Function Add(x As Long) As Long
    //   A = A + x            (property get, property let)
    //   Add = Me.m_a         (field through Me)
    let f = t.procedure(
        "Add",
        KIND_SUB_OR_FUNCTION,
        Some(T_LONG),
        &[("x", T_LONG, false)],
        Some(T_LONG),
    );
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new().read(a).read(x).op(Opcode::NumAdd).write(a),
            Line::new()
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, m)
                .write(add),
        ],
    ));
    (t, lines)
}

#[test]
fn class_members_fields_properties_and_me() {
    let (t, lines) = counter_class();
    let lp = lower_kind(&t, ModuleKind::Class, &lines, &[]);
    assert_eq!(lp.lowered_count(), 4, "{:?}", lp.procedures);
    let (mut vm, _) = new_vm(&lp);
    let module = vm.find_module("Cls").expect("module");
    // Instances are independent and start with Class_Initialize's value.
    let o1 = vm.create_instance(module, None).expect("instance");
    let o2 = vm.create_instance(module, None).expect("instance");
    let get = proc_id(&lp, "A", 0);
    let let_ = proc_id(&lp, "A", 1);
    let add = proc_id(&lp, "Add", 0);
    let run = |vm: &mut Vm, o: &vba_value::ObjectRef, p, args: Vec<Arg>| {
        vm.run_method(*o, p, args).expect("runs").value
    };
    assert_eq!(run(&mut vm, &o1, get, vec![]), l(10));
    assert_eq!(run(&mut vm, &o1, add, vec![Arg::Value(l(5))]), l(15));
    run(&mut vm, &o2, let_, vec![Arg::Value(l(-3))]);
    assert_eq!(run(&mut vm, &o2, add, vec![Arg::Value(l(1))]), l(-2));
    assert_eq!(run(&mut vm, &o1, get, vec![]), l(15));
}

#[test]
fn an_unlowered_initializer_keeps_the_module_from_being_offered() {
    let mut t = Tables::default();
    let f = t.function("Class_Initialize", None, &[]);
    let g = t.function("Work", None, &[]);
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // An opcode that was never observed.
    lines.extend(block(
        f,
        1,
        Opcode::ProcEndSub,
        vec![Line::new().op(Opcode::FileClose)],
    ));
    lines.extend(block(g, 5, Opcode::ProcEndSub, vec![]));
    let lp = lower_kind(&t, ModuleKind::Class, &lines, &[]);
    assert_eq!(lp.lowered_count(), 0);
    assert!(reasons(&lp, "Work").contains("Class_Initialize"));
}

#[test]
fn the_result_record_of_a_class_function_is_not_a_parameter() {
    // The chain of `Function F(x As Long) As Long` has two records in a
    // class-like module; the second is the result.
    let mut t = Tables::default();
    let (x, f) = (t.name("x"), t.name("F"));
    let rec = t.procedure(
        "F",
        KIND_SUB_OR_FUNCTION,
        Some(T_LONG),
        &[("x", T_LONG, true)],
        Some(T_LONG),
    );
    let mut lines = vec![];
    lines.extend(block(
        rec,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(x).i16(1).op(Opcode::NumAdd).write(f)],
    ));
    let lp = lower_kind(&t, ModuleKind::Class, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    let info = lp.ir.program.proc(proc_id(&lp, "F", 0));
    assert_eq!(info.params.len(), 1);
    let (mut vm, _) = new_vm(&lp);
    let module = vm.find_module("Cls").expect("module");
    let o = vm.create_instance(module, None).expect("instance");
    let r = vm
        .run_method(o, proc_id(&lp, "F", 0), vec![Arg::Value(l(41))])
        .expect("runs");
    assert_eq!(r.value, l(42));
}

#[test]
fn a_parameter_outside_the_identifier_table_keeps_its_identity() {
    // The name word is a reference the table cannot resolve; the parameter is
    // still the same name wherever the body uses it.
    let mut t = Tables::default();
    let r = t.name("F");
    let rec = t.procedure(
        "F",
        KIND_SUB_OR_FUNCTION,
        Some(T_LONG),
        &[("#FDFC", T_LONG, true)],
        None,
    );
    let mut lines = vec![];
    lines.extend(block(
        rec,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .read(0xFDFC)
                .i16(2)
                .op(Opcode::NumMultiply)
                .write(r),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    let (mut vm, _) = new_vm(&lp);
    let p = vm.find_procedure("Cls.F").expect("entry");
    let r = vm
        .run_procedure(p, vec![Arg::Value(l(21))])
        .expect("runs")
        .value;
    assert_eq!(r, l(42));
}

/// Runs `name` on a new instance of the document-like module `Cls` whose
/// host base is the recording mock; returns the value and the journal.
fn run_on_document(
    lp: &LoweredProject,
    name: &str,
    args: Vec<Arg>,
) -> (Variant, Vec<String>, String) {
    let (mut vm, sink) = new_vm(lp);
    let module = vm.find_module("Cls").expect("module");
    let journal = Rc::new(RefCell::new(Vec::new()));
    let o = vm
        .create_instance(module, Some(Rc::new(HostMock::new(journal.clone()))))
        .expect("instance");
    let r = vm
        .run_method(o, proc_id(lp, name, 0), args)
        .expect("runs")
        .value;
    let j = journal.borrow().clone();
    (r, j, sink.contents())
}

#[test]
fn members_of_the_host_base_are_late_bound_on_me() {
    let mut t = Tables::default();
    let (text, ctl, cap, refresh, s_, go, shout) = (
        t.name("txtName"),
        t.name("lblTitle"),
        t.name("Caption"),
        t.name("Refresh"),
        t.name("s"),
        t.name("Go"),
        t.name("Shout"),
    );
    let _ = go;
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Sub Go(s As String)
    //   Me.txtName = s & "!"
    //   Me.lblTitle.Caption = "T"
    //   Me.Refresh
    //   Debug.Print Me.lblTitle.Caption
    let f = t.function("Go", None, &[("s", T_STRING, true)]);
    lines.extend(block(
        f,
        1,
        Opcode::ProcEndSub,
        vec![
            Line::new()
                .read(s_)
                .text("!")
                .op(Opcode::TextJoin)
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::WriteDot, text),
            Line::new()
                .text("T")
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, ctl)
                .name(Opcode::WriteDot, cap),
            Line::new()
                .name(Opcode::ReadSimple, ME)
                .name_args(Opcode::InvokeDot, 16, refresh, 0),
            Line::new()
                .op(Opcode::DebugPrefix)
                .op(Opcode::OutTargetObject)
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, ctl)
                .name(Opcode::ReadDot, cap)
                .op(Opcode::OutItemLast),
        ],
    ));
    // Function Shout() As String: Shout = Me.txtName
    let g = t.function("Shout", Some(T_STRING), &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, text)
                .write(shout),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Document, &lines, &[]);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    let (_, journal, out) = run_on_document(&lp, "Go", vec![Arg::Value(s("hi"))]);
    // The value is evaluated before the receiver of an assignment; `Me.lblTitle`
    // is fetched, then `Caption` is set on it.
    assert_eq!(
        journal,
        [
            "put txtname[] = String(VbaString(\"hi!\"))",
            "get lbltitle[]",
            "put lbltitle.caption[] = String(VbaString(\"T\"))",
            "get refresh[]",
            "get lbltitle[]",
            "get lbltitle.caption[]",
        ]
    );
    assert_eq!(out, "T\n");
    let (v, _, _) = run_on_document(&lp, "Shout", vec![]);
    // The control is an object; reading it as a String takes its default member.
    assert_eq!(v, s("<(default)>"));
    // The same members in a class module (no host base) do not lower.
    let lp = lower_kind(&t, ModuleKind::Class, &lines, &[]);
    assert_eq!(lp.lowered_count(), 0);
    assert!(reasons(&lp, "Shout").contains("not available"));
}

#[test]
fn unresolved_names_are_host_members_only_where_the_source_binding_says_so() {
    let mut t = Tables::default();
    let (ctl, cap, r) = (t.name("lblStatus"), t.name("Caption"), t.name("Look"));
    let (r2, call) = (t.name("Peek"), t.name("Beep2"));
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Function Look() As String: Look = lblStatus.Caption   (receiver: always host)
    let f = t.function("Look", Some(T_STRING), &[]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(ctl).name(Opcode::ReadDot, cap).write(r)],
    ));
    // Function Peek() As String: Peek = lblStatus          (bare read: needs the list)
    let g = t.function("Peek", Some(T_STRING), &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(ctl).write(r2)],
    ));
    // Sub Beep2(): lblStatus 3                              (statement: host call)
    let h = t.function("Beep2", None, &[]);
    lines.extend(block(
        h,
        5,
        Opcode::ProcEndSub,
        vec![
            Line::new()
                .i16(3)
                .name_args(Opcode::InvokeSimple, 16, ctl, 1),
        ],
    ));
    let _ = call;
    let lp = lower_kind(&t, ModuleKind::Document, &lines, &[]);
    assert!(lp.report("Cls", "Look").unwrap().outcome.is_lowered());
    assert!(lp.report("Cls", "Beep2").unwrap().outcome.is_lowered());
    assert!(!lp.report("Cls", "Peek").unwrap().outcome.is_lowered());
    let (v, journal, _) = run_on_document(&lp, "Look", vec![]);
    assert_eq!(v, s("<caption>"));
    assert_eq!(journal, ["get lblstatus[]", "get lblstatus.caption[]"]);
    let (_, journal, _) = run_on_document(&lp, "Beep2", vec![]);
    assert_eq!(journal, ["get lblstatus[Integer(3)]"]);
    // Listed by the embedding application, the bare read is late bound too.
    let lp = lower_kind(&t, ModuleKind::Document, &lines, &["lblStatus"]);
    assert!(lp.report("Cls", "Peek").unwrap().outcome.is_lowered());
    let (v, journal, _) = run_on_document(&lp, "Peek", vec![]);
    assert_eq!(v, s("<(default)>"));
    assert_eq!(journal, ["get lblstatus[]", "get lblstatus.(default)[]"]);
}

/// Runs a function of the module `Cls` (any kind) as a procedure of a
/// standard module.
fn call_std(lp: &LoweredProject, name: &str, args: Vec<Arg>) -> Variant {
    let (mut vm, _) = new_vm(lp);
    let p = vm.find_procedure(&format!("Cls.{name}")).expect("entry");
    vm.run_procedure(p, args).expect("runs").value
}

#[test]
fn goto_jumps_to_a_label() {
    let mut t = Tables::default();
    let (x, r, done, f) = (t.name("x"), t.name("r"), t.name("Done"), t.name("F"));
    let rv = t.variable("r", Some(T_LONG));
    let rec = t.function("F", Some(T_LONG), &[("x", T_LONG, true)]);
    let mut lines = vec![];
    // Function F(x As Long) As Long
    //   Dim r As Long: r = 1: GoTo Done: r = 2
    // Done:
    //   F = r + x
    lines.extend(block(
        rec,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, rv),
            Line::new().i16(1).write(r),
            Line::new().name(Opcode::BranchGoto, done),
            Line::new().i16(2).write(r),
            Line::new().name(Opcode::LabelNamed, done),
            Line::new().read(r).read(x).op(Opcode::NumAdd).write(f),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    assert_eq!(call_std(&lp, "F", vec![Arg::Value(l(10))]), l(11));
}

#[test]
fn omitted_arguments_become_missing() {
    let mut t = Tables::default();
    let (a, b, h, use_, bad) = (
        t.name("a"),
        t.name("b"),
        t.name("H"),
        t.name("Use"),
        t.name("Bad"),
    );
    // Function H(a As Long, Optional b As Long) As Long: H = a * 100 + b
    let hr = t.function(
        "H",
        Some(T_LONG),
        &[("a", T_LONG, true), ("b", T_LONG, true)],
    );
    t.mark_optional(hr, 1, false);
    let mut lines = block(
        hr,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .read(a)
                .i16(100)
                .op(Opcode::NumMultiply)
                .read(b)
                .op(Opcode::NumAdd)
                .write(h),
        ],
    );
    // Function Use() As Long: Use = H(1, )
    let u = t.function("Use", Some(T_LONG), &[]);
    lines.extend(block(
        u,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .i16(1)
                .op(Opcode::ArgMissing)
                .call(h, 2)
                .write(use_),
        ],
    ));
    // Function Bad() As Long: Bad = H(, 1)   (the first parameter is required)
    let bd = t.function("Bad", Some(T_LONG), &[]);
    lines.extend(block(
        bd,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::ArgMissing)
                .i16(1)
                .call(h, 2)
                .write(bad),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert!(lp.report("Cls", "Use").unwrap().outcome.is_lowered());
    assert!(reasons(&lp, "Bad").contains("omitted argument"));
    assert_eq!(call_std(&lp, "Use", vec![]), l(100));
}

#[test]
fn local_constants_fold_where_they_are_read() {
    let mut t = Tables::default();
    let (k, j, g) = (t.name("K"), t.name("J"), t.name("G"));
    let kv = t.variable("K", Some(T_LONG));
    let jv = t.variable("J", None);
    let rec = t.function("G", Some(T_LONG), &[]);
    // Function G() As Long
    //   Const K As Long = 40
    //   Const J = -2
    //   G = K + J
    let lines = block(
        rec,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .opm(Opcode::DimMarker, 1)
                .i16(40)
                .rec(Opcode::VarDecl, 2, kv),
            Line::new()
                .opm(Opcode::DimMarker, 1)
                .i16(2)
                .op(Opcode::NumNegate)
                .rec(Opcode::VarDecl, 2, jv),
            Line::new().read(k).read(j).op(Opcode::NumAdd).write(g),
        ],
    );
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    assert_eq!(call_std(&lp, "G", vec![]), l(38));
}

#[test]
fn the_end_statement_ends_the_run() {
    let mut t = Tables::default();
    let (g, n) = (t.name("g"), t.name("n"));
    let gv = t.variable("g", Some(T_LONG));
    let mut lines = vec![
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, gv),
    ];
    // Sub Quit(n As Long): g = n: End: g = 99
    let rec = t.function("Quit", None, &[("n", T_LONG, true)]);
    lines.extend(block(
        rec,
        5,
        Opcode::ProcEndSub,
        vec![
            Line::new().read(n).write(g),
            Line::new().op(Opcode::HaltEnd),
            Line::new().i16(99).write(g),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Standard, &lines, &[]);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    let (mut vm, _) = new_vm(&lp);
    let p = vm.find_procedure("Cls.Quit").expect("entry");
    let r = vm.run_procedure(p, vec![Arg::Value(l(7))]).expect("runs");
    assert_eq!(r.status, vba_vm::RunStatus::Ended);
}

#[test]
fn me_reaches_the_field_even_when_a_parameter_has_the_same_name() {
    let mut t = Tables::default();
    let (m, got) = (t.name("m"), t.name("Got"));
    let mv = t.variable("m", Some(T_LONG));
    let mut lines = vec![
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, mv),
    ];
    // Public Sub Store(m As Long): Me.m = m * 2
    let p = t.function("Store", None, &[("m", T_LONG, true)]);
    lines.extend(block(
        p,
        5,
        Opcode::ProcEndSub,
        vec![
            Line::new()
                .read(m)
                .i16(2)
                .op(Opcode::NumMultiply)
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::WriteDot, m),
        ],
    ));
    // Public Function Got() As Long: Got = Me.m
    let g = t.function("Got", Some(T_LONG), &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, m)
                .write(got),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Class, &lines, &[]);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    let (mut vm, _) = new_vm(&lp);
    let module = vm.find_module("Cls").expect("module");
    let o = vm.create_instance(module, None).expect("instance");
    vm.run_method(o, proc_id(&lp, "Store", 0), vec![Arg::Value(l(21))])
        .expect("runs");
    let r = vm
        .run_method(o, proc_id(&lp, "Got", 0), vec![])
        .expect("runs");
    assert_eq!(r.value, l(42));
}

#[test]
fn a_member_the_cache_declares_is_never_taken_for_a_host_member() {
    let mut t = Tables::default();
    let (x, r) = (t.name("x"), t.name("Look"));
    // Private x As Object   (not a plain scalar: left out of the program)
    let xv = t.variable("x", Some(9));
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, xv),
    ];
    // Function Look() As Variant: Look = Me.x
    let f = t.function("Look", Some(T_VARIANT), &[]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .name(Opcode::ReadSimple, ME)
                .name(Opcode::ReadDot, x)
                .write(r),
        ],
    ));
    let lp = lower_kind(&t, ModuleKind::Document, &lines, &[]);
    assert_eq!(lp.lowered_count(), 0);
    assert!(reasons(&lp, "Look").contains("not available"));
    assert_eq!(
        lp.declarations
            .iter()
            .filter(|d| d.unsupported.is_some())
            .count(),
        1
    );
}
