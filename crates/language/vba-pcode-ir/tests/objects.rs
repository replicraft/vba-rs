//! Types, objects and members in hand-assembled caches: enums, user-defined
//! types, class instances, `Set`, `New`, `Nothing`, named arguments, `With`,
//! `For Each` over a `Collection`, and library-qualified types.
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
use vba_pcode_ir::{
    LowerOptions, LoweredProject, ModuleInput, Outcome, lower_project_with_options,
};
use vba_project::ModuleKind;
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, Vm, VmConfig};

fn lower(t: &Tables, kind: ModuleKind, lines: &[Line], project: Option<&str>) -> LoweredProject {
    let project_stream = build_project_stream(&t.idents);
    let cache = build_cache_t(lines, t);
    let options = LowerOptions {
        host_members: Vec::new(),
        project_name: project.map(str::to_owned),
    };
    let lp = lower_project_with_options(
        &project_stream,
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

fn new_vm(lp: &LoweredProject) -> Vm {
    Vm::new(
        Rc::new(lp.ir.clone()),
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(BufferSink::new()),
        VmConfig::default(),
    )
    .expect("vm")
}

fn reason(lp: &LoweredProject, name: &str) -> String {
    match &lp.report("Cls", name).expect("report").outcome {
        Outcome::Unsupported(u) => u.reason.clone(),
        Outcome::Lowered => String::new(),
    }
}

fn block(rec: u32, header_mod: u16, end: Opcode, body: Vec<Line>) -> Vec<Line> {
    let mut v = vec![Line::new().rec(Opcode::ProcHeader, header_mod, rec)];
    v.extend(body);
    v.push(Line::new().op(end));
    v
}

/// Runs a function of a class on a fresh instance.
fn run_method(lp: &LoweredProject, name: &str, args: Vec<Arg>) -> Variant {
    let mut vm = new_vm(lp);
    let module = vm.find_module("Cls").expect("module");
    let o = vm.create_instance(module, None).expect("instance");
    let p = lp
        .procedures
        .iter()
        .find(|p| p.procedure == name)
        .and_then(|p| p.id)
        .expect("procedure");
    vm.run_method(o, p, args).expect("runs").value
}

/// A function of a class module carries the extra result record.
fn class_function(t: &mut Tables, name: &str, ret: u8, args: &[(&str, u8, bool)]) -> u32 {
    t.procedure(name, KIND_SUB_OR_FUNCTION, Some(ret), args, Some(ret))
}

#[test]
fn a_user_type_in_a_module_variable_is_read_and_written_through_its_fields() {
    let mut t = Tables::default();
    let (this, n, bump, count) = (t.name("This"), t.name("n"), t.name("Bump"), t.name("Count"));
    let e = t.object("TState");
    let d = t.named_type(e);
    let udt = t.udt("TState", false);
    let fld = t.variable("n", Some(T_LONG));
    let this_v = t.variable_of("This", d);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        // Private Type TState / n As Long / End Type
        Line::new().rec(Opcode::TypeDecl, 1, udt),
        Line::new()
            .op(Opcode::DimMarkerImplicit)
            .rec(Opcode::VarDecl, 1, fld),
        Line::new().op(Opcode::TypeDeclEnd),
        // Private This As TState
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, this_v),
    ];
    // Public Sub Bump(): This.n = This.n + 1
    let f = t.function("Bump", None, &[]);
    lines.extend(block(
        f,
        5,
        Opcode::ProcEndSub,
        vec![
            Line::new()
                .read(this)
                .name(Opcode::ReadDot, n)
                .i16(1)
                .op(Opcode::NumAdd)
                .read(this)
                .name(Opcode::WriteDot, n),
        ],
    ));
    // Public Function Count() As Long: Count = This.n
    let g = class_function(&mut t, "Count", T_LONG, &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(this).name(Opcode::ReadDot, n).write(count)],
    ));
    let _ = bump;
    let lp = lower(&t, ModuleKind::Class, &lines, None);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    assert!(lp.declarations.iter().all(|d| d.unsupported.is_none()));
    let mut vm = new_vm(&lp);
    let module = vm.find_module("Cls").expect("module");
    let o = vm.create_instance(module, None).expect("instance");
    let id = |name: &str| {
        lp.procedures
            .iter()
            .find(|p| p.procedure == name)
            .and_then(|p| p.id)
            .expect("procedure")
    };
    for _ in 0..3 {
        vm.run_method(o, id("Bump"), vec![]).expect("runs");
    }
    let r = vm.run_method(o, id("Count"), vec![]).expect("runs").value;
    assert_eq!(r, Variant::Long(3));
}

#[test]
fn an_enum_is_declared_with_its_values_and_types_a_parameter() {
    let mut t = Tables::default();
    let (low, high, up, m) = (t.name("Low"), t.name("High"), t.name("Up"), t.name("m"));
    let e = t.object("Mode");
    let d = t.named_type(e);
    let udt = t.udt("Mode", true);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        // Public Enum Mode / Low / High = 5 / End Enum
        Line::new().rec(Opcode::TypeDecl, 3, udt),
        Line::new().name_args(Opcode::InvokeSimple, 16, low, 0),
        Line::new().i16(5).write(high),
        Line::new().op(Opcode::EnumDeclEnd),
    ];
    // Public Function Up(m As Mode) As Long: Up = m + High
    let f = t.function("Up", Some(T_LONG), &[("m", T_LONG, true)]);
    t.set_arg_type(f, 0, d);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(m).read(high).op(Opcode::NumAdd).write(up)],
    ));
    let lp = lower(&t, ModuleKind::Standard, &lines, None);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    let mut vm = new_vm(&lp);
    let p = vm.find_procedure("Cls.Up").expect("entry");
    let r = vm
        .run_procedure(p, vec![Arg::Value(Variant::Long(2))])
        .expect("runs")
        .value;
    assert_eq!(r, Variant::Long(7));
}

/// `Public v As Long`, and a function that builds an object of its own
/// class, sets a field through the variable and reads it back.
fn self_instancing_class() -> (Tables, Vec<Line>) {
    let mut t = Tables::default();
    let (v, o, twice, gone) = (t.name("v"), t.name("o"), t.name("Twice"), t.name("Gone"));
    let e = t.object("Cls");
    let d = t.named_type(e);
    let vv = t.variable("v", Some(T_LONG));
    let ov = t.variable_of("o", d);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new()
            .opm(Opcode::DimMarker, 8)
            .rec(Opcode::VarDecl, 1, vv),
    ];
    // Public Function Twice() As Long
    //   Dim o As Cls: Set o = New Cls: o.v = 21: Twice = o.v * 2
    let f = class_function(&mut t, "Twice", T_LONG, &[]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, ov),
            Line::new()
                .op(Opcode::SetKeywordMarker)
                .obj(Opcode::NewObject, e)
                .name(Opcode::WriteRefSimple, o),
            Line::new().i16(21).read(o).name(Opcode::WriteDot, v),
            Line::new()
                .read(o)
                .name(Opcode::ReadDot, v)
                .i16(2)
                .op(Opcode::NumMultiply)
                .write(twice),
        ],
    ));
    // Public Function Gone() As Boolean
    //   Dim o As Cls: Set o = New Cls: Set o = Nothing: Gone = o Is Nothing
    let g = class_function(&mut t, "Gone", T_BOOLEAN, &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, ov),
            Line::new()
                .op(Opcode::SetKeywordMarker)
                .obj(Opcode::NewObject, e)
                .name(Opcode::WriteRefSimple, o),
            Line::new()
                .op(Opcode::SetKeywordMarker)
                .op(Opcode::ConstNullObject)
                .name(Opcode::WriteRefSimple, o),
            Line::new()
                .read(o)
                .op(Opcode::ConstNullObject)
                .op(Opcode::CompareObjectIdentity)
                .write(gone),
        ],
    ));
    (t, lines)
}

#[test]
fn set_new_nothing_and_fields_of_an_object() {
    let (t, lines) = self_instancing_class();
    let lp = lower(&t, ModuleKind::Class, &lines, None);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    assert_eq!(run_method(&lp, "Twice", vec![]), Variant::Long(42));
    assert_eq!(run_method(&lp, "Gone", vec![]), Variant::Boolean(true));
}

#[test]
fn named_arguments_are_put_in_parameter_order() {
    let mut t = Tables::default();
    let (a, b, diff, calls) = (t.name("a"), t.name("b"), t.name("Diff"), t.name("Calls"));
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Function Diff(a As Long, b As Long) As Long: Diff = a - b
    let f = t.function(
        "Diff",
        Some(T_LONG),
        &[("a", T_LONG, true), ("b", T_LONG, true)],
    );
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .read(a)
                .read(b)
                .op(Opcode::NumSubtract)
                .write(diff),
        ],
    ));
    // Function Calls() As Long: Calls = Diff(b:=1, a:=10)
    let g = t.function("Calls", Some(T_LONG), &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .i16(1)
                .name(Opcode::ArgNamed, b)
                .i16(10)
                .name(Opcode::ArgNamed, a)
                .call(diff, 2)
                .write(calls),
        ],
    ));
    let lp = lower(&t, ModuleKind::Standard, &lines, None);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    let mut vm = new_vm(&lp);
    let p = vm.find_procedure("Cls.Calls").expect("entry");
    assert_eq!(
        vm.run_procedure(p, vec![]).expect("runs").value,
        Variant::Long(9)
    );
}

#[test]
fn for_each_and_with_over_a_collection() {
    let mut t = Tables::default();
    let (c, x, total, sum, add, count) = (
        t.name("c"),
        t.name("x"),
        t.name("total"),
        t.name("Sum"),
        t.name("Add"),
        t.name("Count"),
    );
    let e = t.object("Collection");
    let d = t.named_type(e);
    let cv = t.variable_of("c", d);
    let xv = t.variable("x", Some(T_VARIANT));
    let tv = t.variable("total", Some(T_LONG));
    let f = class_function(&mut t, "Sum", T_LONG, &[]);
    let add_item = |n: i16| {
        Line::new()
            .i16(n)
            .read(c)
            .name_args(Opcode::InvokeDot, 16, add, 1)
    };
    let body = vec![
        Line::new()
            .op(Opcode::DimMarker)
            .rec(Opcode::VarDecl, 1, cv),
        Line::new()
            .op(Opcode::DimMarker)
            .rec(Opcode::VarDecl, 1, xv),
        Line::new()
            .op(Opcode::DimMarker)
            .rec(Opcode::VarDecl, 1, tv),
        // Set c = New Collection
        Line::new()
            .op(Opcode::SetKeywordMarker)
            .obj(Opcode::NewObject, e)
            .name(Opcode::WriteRefSimple, c),
        add_item(1),
        add_item(2),
        add_item(3),
        // For Each x In c: total = total + x: Next
        Line::new()
            .op(Opcode::ForVarExprOpen)
            .read(x)
            .op(Opcode::ForVarExprClose)
            .read(c)
            .op(Opcode::ForCollection),
        Line::new()
            .read(total)
            .read(x)
            .op(Opcode::NumAdd)
            .write(total),
        Line::new()
            .op(Opcode::ForVarExprOpen)
            .op(Opcode::ForAdvance),
        // With c: .Add 4: End With
        Line::new()
            .op(Opcode::WithExprOpen)
            .read(c)
            .op(Opcode::WithOpen),
        Line::new()
            .i16(4)
            .name_args(Opcode::InvokeWithDot, 16, add, 1),
        Line::new().op(Opcode::WithClose),
        // Sum = total + c.Count
        Line::new()
            .read(total)
            .read(c)
            .name(Opcode::ReadDot, count)
            .op(Opcode::NumAdd)
            .write(sum),
    ];
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    lines.extend(block(f, 6, Opcode::ProcEndFunction, body));
    let lp = lower(&t, ModuleKind::Class, &lines, None);
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    // total = 1 + 2 + 3, then four items are in the collection.
    assert_eq!(run_method(&lp, "Sum", vec![]), Variant::Long(10));
}

#[test]
fn a_type_qualified_with_the_project_name_is_the_project_type() {
    let mut t = Tables::default();
    let (v, o, q) = (t.name("v"), t.name("o"), t.name("Q"));
    let e = t.qualified("Proj", "Cls");
    let d = t.named_type(e);
    let vv = t.variable("v", Some(T_LONG));
    let ov = t.variable_of("o", d);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new()
            .opm(Opcode::DimMarker, 8)
            .rec(Opcode::VarDecl, 1, vv),
    ];
    // Function Q() As Long: Dim o As Proj.Cls: Set o = New Proj.Cls: o.v = 5: Q = o.v
    let f = class_function(&mut t, "Q", T_LONG, &[]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, ov),
            Line::new()
                .op(Opcode::SetKeywordMarker)
                .obj(Opcode::NewObject, e)
                .name(Opcode::WriteRefSimple, o),
            Line::new().i16(5).read(o).name(Opcode::WriteDot, v),
            Line::new().read(o).name(Opcode::ReadDot, v).write(q),
        ],
    ));
    // The qualifier names the project: the type is the class `Cls`.
    let lp = lower(&t, ModuleKind::Class, &lines, Some("Proj"));
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    assert_eq!(run_method(&lp, "Q", vec![]), Variant::Long(5));
    // Any other qualifier is a library whose types the cache does not
    // describe: nothing is guessed.
    let other = lower(&t, ModuleKind::Class, &lines, Some("Other"));
    assert_eq!(other.lowered_count(), 0);
    assert!(
        reason(&other, "Q").contains("library-qualified type `Cls`"),
        "{}",
        reason(&other, "Q")
    );
    let none = lower(&t, ModuleKind::Class, &lines, None);
    assert_eq!(none.lowered_count(), 0);
}

#[test]
fn a_variable_of_another_type_passed_byref_to_a_variant_is_copied_back() {
    let mut t = Tables::default();
    let (v, n, inc, calls) = (t.name("v"), t.name("n"), t.name("Inc"), t.name("Calls"));
    let nv = t.variable("n", Some(T_LONG));
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Sub Inc(ByRef v As Variant): v = v + 1
    let f = t.function("Inc", None, &[("v", T_VARIANT, false)]);
    lines.extend(block(
        f,
        5,
        Opcode::ProcEndSub,
        vec![Line::new().read(v).i16(1).op(Opcode::NumAdd).write(v)],
    ));
    // Function Calls() As Long: Dim n As Long: n = 5: Inc n: Calls = n
    let g = t.function("Calls", Some(T_LONG), &[]);
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, nv),
            Line::new().i16(5).write(n),
            Line::new()
                .read(n)
                .name_args(Opcode::InvokeSimple, 16, inc, 1),
            Line::new().read(n).write(calls),
        ],
    ));
    let lp = lower(&t, ModuleKind::Standard, &lines, None);
    assert_eq!(lp.lowered_count(), 2, "{:?}", lp.procedures);
    let mut vm = new_vm(&lp);
    let p = vm.find_procedure("Cls.Calls").expect("entry");
    // The callee works on a `Variant` copy that is converted back to `Long`.
    assert_eq!(
        vm.run_procedure(p, vec![]).expect("runs").value,
        Variant::Long(6)
    );
}

#[test]
fn a_property_with_a_parameter_is_assigned_with_the_value_last() {
    let mut t = Tables::default();
    let (m, i, v, item, o, probe) = (
        t.name("m"),
        t.name("i"),
        t.name("v"),
        t.name("Item"),
        t.name("o"),
        t.name("Probe"),
    );
    let e = t.object("Cls");
    let d = t.named_type(e);
    let mv = t.variable("m", Some(T_LONG));
    let ov = t.variable_of("o", d);
    let mut lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new()
            .opm(Opcode::DimMarker, 16)
            .rec(Opcode::VarDecl, 1, mv),
    ];
    // Property Get Item(i As Long) As Long: Item = m + i
    let g = t.procedure(
        "Item",
        KIND_PROPERTY_GET,
        Some(T_LONG),
        &[("i", T_LONG, true)],
        Some(T_LONG),
    );
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndProperty,
        vec![Line::new().read(m).read(i).op(Opcode::NumAdd).write(item)],
    ));
    // Property Let Item(i As Long, v As Long): m = v * 10 + i
    let p = t.procedure(
        "Item",
        KIND_PROPERTY_LET,
        None,
        &[("i", T_LONG, true), ("v", T_LONG, true)],
        None,
    );
    lines.extend(block(
        p,
        6,
        Opcode::ProcEndProperty,
        vec![
            Line::new()
                .read(v)
                .i16(10)
                .op(Opcode::NumMultiply)
                .read(i)
                .op(Opcode::NumAdd)
                .write(m),
        ],
    ));
    // Function Probe() As Long
    //   Dim o As Cls: Set o = New Cls: o.Item(2) = 7: Probe = o.Item(1)
    let f = class_function(&mut t, "Probe", T_LONG, &[]);
    lines.extend(block(
        f,
        6,
        Opcode::ProcEndFunction,
        vec![
            Line::new()
                .op(Opcode::DimMarker)
                .rec(Opcode::VarDecl, 1, ov),
            Line::new()
                .op(Opcode::SetKeywordMarker)
                .obj(Opcode::NewObject, e)
                .name(Opcode::WriteRefSimple, o),
            // the value, the index, the receiver
            Line::new()
                .i16(7)
                .i16(2)
                .read(o)
                .name_args(Opcode::WriteDotParams, 0, item, 1),
            Line::new()
                .i16(1)
                .read(o)
                .name_args(Opcode::ReadDotParams, 0, item, 1)
                .write(probe),
        ],
    ));
    let lp = lower(&t, ModuleKind::Class, &lines, None);
    assert_eq!(lp.lowered_count(), 3, "{:?}", lp.procedures);
    // m = 7 * 10 + 2 = 72, then Item(1) = 72 + 1.
    assert_eq!(run_method(&lp, "Probe", vec![]), Variant::Long(73));
}

#[test]
fn a_library_enum_qualifies_its_members_even_when_the_host_lists_the_name() {
    let mut t = Tables::default();
    let (vt, vs, f) = (t.name("VbVarType"), t.name("vbString"), t.name("F"));
    let g = class_function(&mut t, "F", T_LONG, &[]);
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    // Function F() As Long: F = VbVarType.vbString
    lines.extend(block(
        g,
        6,
        Opcode::ProcEndFunction,
        vec![Line::new().read(vt).name(Opcode::ReadDot, vs).write(f)],
    ));
    let project = build_project_stream(&t.idents);
    let cache = build_cache_t(&lines, &t);
    let options = LowerOptions {
        host_members: vec!["VbVarType".to_owned()],
        project_name: None,
    };
    let lp = lower_project_with_options(
        &project,
        1,
        &[ModuleInput {
            name: "Cls",
            kind: ModuleKind::Document,
            cache: &cache,
        }],
        &options,
    )
    .expect("lowers");
    vba_ir::verify(&lp.ir).expect("verified");
    assert_eq!(lp.lowered_count(), 1, "{:?}", lp.procedures);
    // `vbString` is 8: a constant of the library, not a member of the host.
    assert_eq!(run_method(&lp, "F", vec![]), Variant::Long(8));
}
