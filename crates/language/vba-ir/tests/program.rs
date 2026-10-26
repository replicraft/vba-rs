//! Whole-program lowering, statement tables, class information and the
//! verifier's negative cases.

mod common;

use common::*;
use expect_test::expect;
use vba_ir::*;
use vba_project::ModuleKind;

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../vba-sema/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn corpus() -> Vec<vba_sema::SourceModule> {
    vec![
        module("Geometry", ModuleKind::Standard, &fixture("Geometry.bas")),
        module("IShape", ModuleKind::Class, &fixture("IShape.cls")),
        module("Disc", ModuleKind::Class, &fixture("Disc.cls")),
        module("Watcher", ModuleKind::Class, &fixture("Watcher.cls")),
        module("WinApi", ModuleKind::Standard, &fixture("WinApi.bas")),
        module("Tour", ModuleKind::Standard, &fixture("Tour.bas")),
        module("Text", ModuleKind::Standard, &fixture("Text.bas")),
        module("Bag", ModuleKind::Class, &fixture("Bag.cls")),
        module("BagUser", ModuleKind::Class, &fixture("BagUser.cls")),
    ]
}

#[test]
fn corpus_lowers_and_verifies() {
    let ir = lower_ok(&corpus());
    assert_eq!(ir.program.procs.len(), ir.functions.len() - init_count(&ir));
    // Dumping is deterministic.
    let a = dump_program(&ir, DumpOptions::default());
    let b = dump_program(&lower_ok(&corpus()), DumpOptions::default());
    assert_eq!(a, b);
}

fn init_count(ir: &IrProgram) -> usize {
    ir.module_init.iter().flatten().count()
}

#[test]
fn class_information() {
    let ir = lower_ok(&corpus());
    let disc = ir.program.module_by_name("Disc").unwrap().id;
    let info = ir.classes[disc.index()].as_ref().unwrap();
    assert!(info.initialize.is_some() && info.terminate.is_some());
    assert_eq!(info.implements.len(), 1);
    assert_eq!(info.events.len(), 1);
    assert!(info.fields.len() >= 2);
    let watcher = ir.program.module_by_name("Watcher").unwrap().id;
    let w = ir.classes[watcher.index()].as_ref().unwrap();
    assert_eq!(w.handlers.len(), 1);
    // Standard modules have no class information.
    let geo = ir.program.module_by_name("Geometry").unwrap().id;
    assert!(ir.classes[geo.index()].is_none());
}

#[test]
fn module_initialiser_allocates_fixed_arrays() {
    let ir = lower_ok(&corpus());
    let geo = ir.program.module_by_name("Geometry").unwrap().id;
    let init = ir.module_init[geo.index()].expect("Geometry has a fixed array");
    let text = dump_function(&ir, &ir.functions[init.0 as usize], DumpOptions::default());
    expect![[r#"
        func Geometry.$init
          b0:
            r0 = ref g0:Geometry.shapes
            dim.fixed r0 as IShape (1 To 16)
            return
    "#]]
    .assert_eq(&text);
}

#[test]
fn class_methods_take_receivers() {
    let ir = lower_ok(&corpus());
    let p = ir.program.find_proc("Disc", "IShape_Rescale").unwrap();
    let text = dump_function(&ir, ir.function_of(p.id), DumpOptions::default());
    expect![[r#"
        func Disc.IShape_Rescale
          slot s0 factor : Double (param 0)
          b0:
            r0 = me
            r1 = load.field r0.g3:Disc.Radius
            r2 = load.local s0:factor
            r3 = mul.typed:Double r1, r2
            r4 = me
            store.field r4.g3:Disc.Radius <- r3 [let]
            r5 = me
            r6 = load.field r5.g3:Disc.Radius
            raise.event Disc.Resized(r6)
            return
    "#]]
    .assert_eq(&text);
}

#[test]
fn interface_calls_name_the_interface_member() {
    let ir = lower_ok(&corpus());
    let p = ir.program.find_proc("Geometry", "TotalArea").unwrap();
    let text = dump_function(&ir, ir.function_of(p.id), DumpOptions::default());
    assert!(text.contains("call IShape.Area() on"), "{text}");
}

#[test]
fn statements_carry_spans_and_resume_points() {
    let src = "Sub P()\nOn Error Resume Next\nDebug.Print 1 / 0\nDebug.Print 2\nEnd Sub\n";
    let ir = lower_ok(&[module("M", ModuleKind::Standard, src)]);
    let p = ir.program.find_proc("M", "P").unwrap();
    let f = ir.function_of(p.id);
    // Every instruction belongs to a statement whose span covers it.
    for b in &f.blocks {
        for i in &b.insts {
            let s = &f.stmts[i.stmt.0 as usize];
            assert!(
                i.span.start >= s.span.start && i.span.end <= s.span.end,
                "{:?} not inside {:?}",
                i.span,
                s.span
            );
        }
    }
    // The division statement resumes right after its own instructions.
    let div = f
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .find(|i| matches!(i.kind, InstKind::Binary { .. }))
        .unwrap();
    let stmt = &f.stmts[div.stmt.0 as usize];
    let point = stmt.resume_next.expect("resume point");
    let block = &f.blocks[point.block.0 as usize];
    let last = block
        .insts
        .iter()
        .rposition(|i| i.stmt == div.stmt)
        .expect("statement instructions");
    assert_eq!(point.index as usize, last + 1);
    let text = dump_function(
        &ir,
        f,
        DumpOptions {
            spans: true,
            stmts: true,
        },
    );
    assert!(
        text.contains("; stmt#") && text.contains("resume.next b0:"),
        "{text}"
    );
}

#[test]
fn erroneous_programs_are_not_lowered() {
    let a = analyze(&[module(
        "M",
        ModuleKind::Standard,
        "Sub P()\nx = undefined(1)\nEnd Sub\n",
    )]);
    assert!(matches!(lower(&a), Err(LowerError::HasErrors(n)) if n >= 1));
}

// ---- verifier negative tests -------------------------------------------------

fn small() -> IrProgram {
    lower_ok(&[module(
        "M",
        ModuleKind::Standard,
        "Function F(ByVal n As Long) As Long\nIf n > 1 Then\nF = n\nEnd If\nEnd Function\n",
    )])
}

fn first_error(ir: &IrProgram) -> String {
    verify(ir).expect_err("verifier should fail")[0]
        .message
        .clone()
}

#[test]
fn verifier_accepts_valid_programs() {
    verify(&small()).unwrap();
}

#[test]
fn verifier_rejects_bad_jump_targets() {
    let mut ir = small();
    ir.functions[0].blocks[0].term = Terminator::Jump(BlockId(99));
    assert!(first_error(&ir).contains("missing block"));
}

#[test]
fn verifier_rejects_use_before_def() {
    let mut ir = small();
    let f = &mut ir.functions[0];
    let r = Reg(f.regs.len() as u32);
    let d = Reg(r.0 + 1);
    f.regs.push(vba_sema::Type::Long);
    f.regs.push(vba_sema::Type::Long);
    let stmt = f.blocks[0].insts[0].stmt;
    f.blocks[0].insts.insert(
        0,
        Inst {
            span: vba_source::Span::empty(0),
            stmt,
            kind: InstKind::Copy {
                dst: d,
                src: Operand::Reg(r),
            },
        },
    );
    let msg = verify(&ir).expect_err("must fail");
    assert!(
        msg.iter().any(|e| e.message.contains("used before")),
        "{msg:?}"
    );
}

#[test]
fn verifier_rejects_double_definition() {
    let mut ir = small();
    let f = &mut ir.functions[0];
    let first = f.blocks[0].insts[0].clone();
    f.blocks[0].insts.insert(1, first);
    let errs = verify(&ir).expect_err("must fail");
    assert!(
        errs.iter().any(|e| e.message.contains("defined 2 times")),
        "{errs:?}"
    );
}

#[test]
fn verifier_rejects_wrong_argument_counts() {
    let mut ir = lower_ok(&[module(
        "M",
        ModuleKind::Standard,
        "Sub Q(a As Long)\nEnd Sub\nSub P()\nQ 1\nEnd Sub\n",
    )]);
    let p = ir.program.find_proc("M", "P").unwrap().id;
    let f = &mut ir.functions[p.index()];
    for b in &mut f.blocks {
        for i in &mut b.insts {
            if let InstKind::Call { args, .. } = &mut i.kind {
                args.clear();
            }
        }
    }
    assert!(first_error(&ir).contains("argument"));
}

#[test]
fn verifier_rejects_non_boolean_conditions() {
    let mut ir = lower_ok(&[module(
        "M",
        ModuleKind::Standard,
        "Sub P(ByVal n As Long)\nIf n > 1 Then Debug.Print 1\nEnd Sub\n",
    )]);
    let p = ir.program.find_proc("M", "P").unwrap().id;
    let f = &mut ir.functions[p.index()];
    for r in &mut f.regs {
        if *r == vba_sema::Type::Boolean {
            *r = vba_sema::Type::Long;
        }
    }
    assert!(first_error(&ir).contains("Boolean or Variant"));
}

#[test]
fn verifier_rejects_resume_points_outside_blocks() {
    let mut ir = small();
    ir.functions[0].stmts[0].resume_next = Some(Point {
        block: BlockId(0),
        index: 1000,
    });
    assert!(first_error(&ir).contains("resume point"));
}

#[test]
fn verifier_rejects_missing_handler_blocks() {
    let mut ir = lower_ok(&[module(
        "M",
        ModuleKind::Standard,
        "Sub P()\nOn Error GoTo h\nExit Sub\nh:\nEnd Sub\n",
    )]);
    let p = ir.program.find_proc("M", "P").unwrap().id;
    for b in &mut ir.functions[p.index()].blocks {
        for i in &mut b.insts {
            if let InstKind::SetErrorMode { mode } = &mut i.kind {
                *mode = ErrMode::Handler(BlockId(77));
            }
        }
    }
    assert!(first_error(&ir).contains("handler block"));
}
