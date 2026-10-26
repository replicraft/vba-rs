//! Differential execution on a local project (ignored; set
//! `VBA_PCODE_PROJECT`): the lowered cache program and the program compiled
//! from the original source run the same procedures with the same arguments
//! on `vba-vm`; results, `Debug.Print` output and errors must agree.
#![allow(missing_docs)]

mod common;

use std::rc::Rc;

use vba_host::{HeadlessHost, ResourceLimits};
use vba_ir::IrProgram;
use vba_pcode_ir::{LoweredProject, ModuleInput, Outcome, lower_project};
use vba_project::ModuleKind;
use vba_sema::{SourceModule, Type};
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, Vm, VmConfig};

fn pool(t: &Type) -> Vec<Variant> {
    match t {
        Type::Integer => [0, 1, 7, -3, 12].map(Variant::Integer).to_vec(),
        Type::Long => [0, 1, 42, -7, 100_000].map(Variant::Long).to_vec(),
        Type::Single => [0.0, 1.0, 0.5, -2.25, 100.0].map(Variant::Single).to_vec(),
        Type::Double | Type::Currency | Type::Date => {
            [0.0, 1.0, 0.5, -2.25, 100.0].map(Variant::Double).to_vec()
        }
        Type::Byte => [0, 1, 200].map(Variant::Byte).to_vec(),
        Type::Boolean => vec![Variant::Boolean(false), Variant::Boolean(true)],
        Type::String => ["", "a", "Hello", "abc def", "42"]
            .map(|s| Variant::String(s.into()))
            .to_vec(),
        _ => vec![
            Variant::Integer(5),
            Variant::String("x".into()),
            Variant::Double(2.5),
            Variant::Empty,
        ],
    }
}

/// Argument sets; `None` is an omitted argument. Sets with the trailing
/// `Optional` parameters omitted (each cut point) are added.
fn arg_sets(params: &[(Type, bool)]) -> Vec<Vec<Option<Variant>>> {
    let pools: Vec<Vec<Variant>> = params.iter().map(|p| pool(&p.0)).collect();
    let mut sets: Vec<Vec<Option<Variant>>> = Vec::new();
    for k in 0..6 {
        let set: Vec<Option<Variant>> = pools
            .iter()
            .enumerate()
            .map(|(i, p)| Some(p[(k + i) % p.len()].clone()))
            .collect();
        for cut in 0..=params.len() {
            if params[cut..].iter().all(|p| p.1) {
                let mut c = set.clone();
                for slot in &mut c[cut..] {
                    *slot = None;
                }
                if !sets.contains(&c) {
                    sets.push(c);
                }
            }
        }
    }
    sets
}

fn run_one(ir: &Rc<IrProgram>, name: &str, args: &[Option<Variant>]) -> String {
    let sink = BufferSink::new();
    let host = HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(300_000),
        ..ResourceLimits::default()
    });
    let mut vm = match Vm::new(
        ir.clone(),
        Rc::new(host),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    ) {
        Ok(v) => v,
        Err(e) => return format!("vm: {e}"),
    };
    let Some(p) = vm.find_procedure(name) else {
        return "no such procedure".into();
    };
    let r = vm.run_procedure(
        p,
        args.iter()
            .cloned()
            .map(|a| a.map_or(Arg::Missing, Arg::Value))
            .collect(),
    );
    format!("{r:?} | out={:?}", sink.contents())
}

/// Source program with only the lowered procedures of the project.
fn source_program(p: &common::Project, lp: &LoweredProject) -> Rc<IrProgram> {
    let mut mods = Vec::new();
    for m in &p.modules {
        if m.kind != ModuleKind::Standard {
            continue;
        }
        let lines = common::logical_lines(&m.source);
        let mut text = String::new();
        for l in &lines {
            let t = String::from_utf8_lossy(l);
            if t.trim_start()
                .to_ascii_lowercase()
                .starts_with("option compare")
            {
                text.push_str(&t);
                text.push('\n');
            }
        }
        for r in lp
            .procedures
            .iter()
            .filter(|r| r.module == m.name && r.outcome.is_lowered())
        {
            for l in &lines[r.first_line..=r.last_line] {
                text.push_str(&String::from_utf8_lossy(l));
                text.push('\n');
            }
        }
        mods.push(SourceModule {
            name: m.name.clone(),
            kind: ModuleKind::Standard,
            source: text.into_bytes(),
        });
    }
    let a = vba_sema::analyze_sources(
        &mods,
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    let errs: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .map(|d| vba_diagnostics::render_plain(d, &a.sources))
        .collect();
    assert!(
        errs.is_empty(),
        "source side does not compile:\n{}",
        errs.join("\n")
    );
    let ir = vba_ir::lower(&a).expect("source lowering");
    vba_ir::verify(&ir).expect("source IR verifies");
    Rc::new(ir)
}

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn lowered_procedures_agree_with_source() {
    let Some(p) = common::load() else { return };
    let inputs: Vec<ModuleInput<'_>> = p
        .modules
        .iter()
        .map(|m| ModuleInput {
            name: &m.name,
            class_like: m.kind != ModuleKind::Standard,
            cache: &m.cache,
        })
        .collect();
    let lp = lower_project(&p.project_stream, p.sys_kind, &inputs).expect("lowers");
    let total = lp.procedures.len();
    let lowered = lp.lowered_count();
    let src = source_program(&p, &lp);
    let cache_ir = Rc::new(lp.ir.clone());
    let (mut ran, mut runs, mut mismatches, mut errors_both) = (0usize, 0usize, 0usize, 0usize);
    for r in lp.procedures.iter().filter(|r| r.outcome.is_lowered()) {
        let Some(id) = r.id else { continue };
        let params: Vec<(Type, bool)> = cache_ir
            .program
            .proc(id)
            .params
            .iter()
            .map(|q| (q.ty.clone(), q.optional))
            .collect();
        let full = format!("{}.{}", r.module, r.procedure);
        let mut any = false;
        for args in arg_sets(&params) {
            let a = run_one(&src, &full, &args);
            let b = run_one(&cache_ir, &full, &args);
            runs += 1;
            any = true;
            if a.starts_with("Err") {
                errors_both += 1;
            }
            if a != b {
                mismatches += 1;
                println!("MISMATCH {full} {args:?}\n  source: {a}\n  cache : {b}");
            }
        }
        ran += any as usize;
    }
    println!(
        "procedures {total}, lowered {lowered}, executed {ran}, runs {runs} (source-side errors {errors_both}), mismatches {mismatches}"
    );
    assert_eq!(mismatches, 0);
    let _ = Outcome::Lowered;
}
