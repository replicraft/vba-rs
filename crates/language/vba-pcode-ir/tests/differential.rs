//! Differential execution on a local project (ignored; set
//! `VBA_PCODE_PROJECT`): the lowered cache program and the program compiled
//! from the original source run the same procedures with the same arguments
//! on `vba-vm`; results, `Debug.Print` output, errors and everything done to
//! the host base object of document-like modules must agree.
#![allow(missing_docs)]

mod common;

use common::host::HostMock;
use std::cell::RefCell;
use std::rc::Rc;

use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_ir::IrProgram;
use vba_pcode_ir::{LowerOptions, LoweredProject, ModuleInput, lower_project_with_options};
use vba_project::ModuleKind;
use vba_sema::{SourceModule, Type};
use vba_value::Variant;
use vba_vm::{Arg, BufferSink, NoServices, Vm, VmConfig};

/// An argument as a recipe, so that every VM builds its own objects.
#[derive(Debug, Clone, PartialEq)]
enum Sym {
    Val(Variant),
    /// `Nothing`.
    Nothing,
    /// A new instance of a class-like module of the project.
    Inst(String),
    /// The host mock object.
    Host,
}

fn pool(t: &Type, program: &vba_sema::Program) -> Vec<Sym> {
    let v = |x: Vec<Variant>| x.into_iter().map(Sym::Val).collect::<Vec<_>>();
    match t {
        Type::Object => vec![Sym::Nothing, Sym::Host, Sym::Nothing, Sym::Host],
        Type::Class(m) => vec![
            Sym::Nothing,
            Sym::Inst(program.module(*m).name.to_string()),
            Sym::Nothing,
        ],
        Type::Enum(_) => v([0, 1, 2, 3, -1].map(Variant::Long).to_vec()),
        Type::Udt(_) => vec![Sym::Nothing],
        t => v(pool_scalar(t)),
    }
}

fn pool_scalar(t: &Type) -> Vec<Variant> {
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
fn arg_sets(params: &[(Type, bool)], program: &vba_sema::Program) -> Vec<Vec<Option<Sym>>> {
    let pools: Vec<Vec<Sym>> = params.iter().map(|p| pool(&p.0, program)).collect();
    let mut sets: Vec<Vec<Option<Sym>>> = Vec::new();
    for k in 0..6 {
        let set: Vec<Option<Sym>> = pools
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

/// Which procedure of a module: name and kind (property procedures share a
/// name).
#[derive(Clone, Copy)]
struct ProcSel<'a> {
    name: &'a str,
    kind: vba_ast::ProcKind,
}

fn find_proc(ir: &IrProgram, module: &str, sel: &ProcSel<'_>) -> Option<vba_sema::ProcId> {
    let m = ir.program.module_by_name(module)?;
    m.procs
        .iter()
        .map(|id| ir.program.proc(*id))
        .find(|p| p.name.to_string().eq_ignore_ascii_case(sel.name) && p.kind == sel.kind)
        .map(|p| p.id)
}

/// Builds the arguments of a call on `vm`.
fn build_args(
    vm: &mut Vm,
    host: &HeadlessHost,
    args: &[Option<Sym>],
    journal: &Rc<RefCell<Vec<String>>>,
) -> Result<Vec<Arg>, String> {
    let mut out = Vec::new();
    for a in args {
        out.push(match a {
            None => Arg::Missing,
            Some(Sym::Val(v)) => Arg::Value(v.clone()),
            Some(Sym::Nothing) => Arg::Value(Variant::Object(vba_value::ObjectRef::Nothing)),
            Some(Sym::Host) => {
                let h = host.objects().insert_obj(HostMock::new(journal.clone()));
                Arg::Value(Variant::Object(vba_value::ObjectRef::Handle(h)))
            }
            Some(Sym::Inst(name)) => {
                let Some(mid) = vm.find_module(name) else {
                    return Err("no such module".into());
                };
                let o = vm
                    .create_instance(mid, Some(Rc::new(HostMock::new(journal.clone()))))
                    .map_err(|e| format!("create arg: {e:?}"))?;
                Arg::Value(Variant::Object(o))
            }
        });
    }
    Ok(out)
}

/// Runs a procedure: a standard-module one directly, a member of a
/// class-like module on a fresh instance whose host base is the mock.
fn run_one(
    ir: &Rc<IrProgram>,
    module: &str,
    class_like: bool,
    sel: &ProcSel<'_>,
    args: &[Option<Sym>],
) -> String {
    let sink = BufferSink::new();
    let host = Rc::new(
        HeadlessHost::new().with_limits(ResourceLimits::default().with_max_steps(Some(300_000))),
    );
    let mut vm = match Vm::new(
        ir.clone(),
        host.clone(),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    ) {
        Ok(v) => v,
        Err(e) => return format!("vm: {e}"),
    };
    let Some(p) = find_proc(ir, module, sel) else {
        return "no such procedure".into();
    };
    let journal = Rc::new(RefCell::new(Vec::new()));
    let args = match build_args(&mut vm, &host, args, &journal) {
        Ok(a) => a,
        Err(e) => return e,
    };
    let r = if class_like {
        let Some(mid) = vm.find_module(module) else {
            return "no such module".into();
        };
        let mock = HostMock::new(journal.clone());
        let obj = match vm.create_instance(mid, Some(Rc::new(mock))) {
            Ok(o) => o,
            Err(e) => return format!("create: {e:?} | out={:?}", sink.contents()),
        };
        let r = vm.run_method(obj, p, args);
        let d = vm.destroy_instance(&obj);
        match (r, d) {
            (Ok(o), Ok(())) => Ok(o),
            (Ok(_), Err(e)) | (Err(e), _) => Err(e),
        }
    } else {
        vm.run_procedure(p, args)
    };
    format!(
        "{r:?} | out={:?} | host={:?}",
        sink.contents(),
        journal.borrow()
    )
}

/// Runs several procedures one after the other on the same VM (and, for
/// a class-like module, the same instance), so that module-level state and
/// fields written by one step are read by the next.
fn run_session(
    ir: &Rc<IrProgram>,
    module: &str,
    class_like: bool,
    steps: &[(ProcSel<'_>, Vec<Option<Sym>>)],
) -> String {
    let sink = BufferSink::new();
    let host = Rc::new(
        HeadlessHost::new().with_limits(ResourceLimits::default().with_max_steps(Some(300_000))),
    );
    let mut vm = match Vm::new(
        ir.clone(),
        host.clone(),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    ) {
        Ok(v) => v,
        Err(e) => return format!("vm: {e}"),
    };
    let journal = Rc::new(RefCell::new(Vec::new()));
    let obj = if class_like {
        let Some(mid) = vm.find_module(module) else {
            return "no such module".into();
        };
        match vm.create_instance(mid, Some(Rc::new(HostMock::new(journal.clone())))) {
            Ok(o) => Some(o),
            Err(e) => return format!("create: {e:?} | out={:?}", sink.contents()),
        }
    } else {
        None
    };
    let mut out = String::new();
    for (sel, args) in steps {
        let Some(p) = find_proc(ir, module, sel) else {
            return "no such procedure".into();
        };
        let args = match build_args(&mut vm, &host, args, &journal) {
            Ok(a) => a,
            Err(e) => return e,
        };
        let r = match obj {
            Some(o) => vm.run_method(o, p, args),
            None => vm.run_procedure(p, args),
        };
        out.push_str(&format!("{} = {r:?}\n", sel.name));
    }
    if let Some(o) = obj {
        out.push_str(&format!("destroy {:?}\n", vm.destroy_instance(&o)));
    }
    format!(
        "{out}out={:?} | host={:?}",
        sink.contents(),
        journal.borrow()
    )
}

/// Removes the project name used as a qualifier (`Project.Type`) from source
/// text: the compiled code does not keep the qualifier of a type of the
/// project itself, and the source analysis does not resolve it for an enum
/// declared in a class module.
fn strip_project_qualifier(text: &str, project: &str) -> String {
    if project.is_empty() {
        return text.to_owned();
    }
    let lower = text.to_ascii_lowercase();
    let needle = format!("{}.", project.to_ascii_lowercase());
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let at_boundary = i == 0 || {
            let c = bytes[i - 1];
            !(c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
        };
        if at_boundary && lower[i..].starts_with(&needle) {
            i += needle.len();
        } else {
            let ch = text[i..].chars().next().unwrap_or(' ');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Source program with only the lowered procedures of the project, and the
/// module-level declarations that the cache side generated.
fn source_program(
    p: &common::Project,
    lp: &LoweredProject,
    host_members: &[String],
) -> Result<Rc<IrProgram>, String> {
    let mut mods = Vec::new();
    for m in &p.modules {
        let lines = common::logical_lines(&m.source);
        let mut text = String::new();
        for l in &lines {
            let t = String::from_utf8_lossy(l);
            let low = t.trim_start().to_ascii_lowercase();
            if low.starts_with("option compare") || low.starts_with("option explicit") {
                text.push_str(&t);
                text.push('\n');
            }
        }
        let mut decl_lines: Vec<usize> = lp
            .declarations
            .iter()
            .filter(|d| d.module == m.name && d.unsupported.is_none())
            .flat_map(|d| d.line..=d.last_line)
            .collect();
        decl_lines.sort_unstable();
        decl_lines.dedup();
        for l in decl_lines {
            text.push_str(&String::from_utf8_lossy(&lines[l]));
            text.push('\n');
        }
        for r in lp
            .procedures
            .iter()
            .filter(|r| r.module == m.name && r.id.is_some())
        {
            if r.outcome.is_lowered() {
                for l in &lines[r.first_line..=r.last_line] {
                    text.push_str(&String::from_utf8_lossy(l));
                    text.push('\n');
                }
            } else {
                // In the shell but not lowered (so never run): header and
                // end only, so that the module has the same members and
                // `Implements` can be satisfied.
                for l in [&lines[r.first_line], &lines[r.last_line]] {
                    text.push_str(&String::from_utf8_lossy(l));
                    text.push('\n');
                }
            }
        }
        mods.push(SourceModule {
            name: m.name.clone(),
            kind: common::role(m),
            source: strip_project_qualifier(&text, &p.name).into_bytes(),
        });
    }
    let a = vba_sema::analyze_sources(
        &mods,
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options {
            host_members: host_members.to_vec(),
            project_name: p.name.clone(),
            ..vba_sema::Options::default()
        },
    );
    let errs: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .map(|d| vba_diagnostics::render_plain(d, &a.sources))
        .collect();
    if !errs.is_empty() && std::env::var_os("DUMP_SRC").is_some() {
        for m in &mods {
            println!("--- {}\n{}", m.name, String::from_utf8_lossy(&m.source));
        }
    }
    if !errs.is_empty() {
        return Err(format!(
            "source side does not compile:\n{}",
            errs.join("\n")
        ));
    }
    let ir = vba_ir::lower(&a).map_err(|e| format!("source lowering: {e:?}"))?;
    vba_ir::verify(&ir).map_err(|e| format!("source IR does not verify: {e:?}"))?;
    Ok(Rc::new(ir))
}

/// What one differential run covered.
#[derive(Debug, Default, Clone, Copy)]
struct Summary {
    procedures: usize,
    lowered: usize,
    /// `[standard, class-like]`
    executed: [usize; 2],
    runs: [usize; 2],
    errors_both: usize,
    sessions: usize,
    mismatches: usize,
}

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn lowered_procedures_agree_with_source() {
    let Some(p) = common::load() else { return };
    let s = differential(&p).expect("differential run");
    println!(
        "procedures {}, lowered {}, executed {} + {}, runs {} + {} (source-side errors {}), sessions {}, mismatches {}",
        s.procedures,
        s.lowered,
        s.executed[0],
        s.executed[1],
        s.runs[0],
        s.runs[1],
        s.errors_both,
        s.sessions,
        s.mismatches
    );
    assert_eq!(s.mismatches, 0);
}

/// The differential run over every project file below the directory named by
/// `VBA_PCODE_CORPUS`: one line per file with what was lowered, executed and
/// compared. A file whose source side cannot be compiled by the source
/// analysis is reported and skipped.
#[test]
#[ignore = "needs a local corpus (VBA_PCODE_CORPUS)"]
fn corpus_agrees_with_source() {
    let mut total = Summary::default();
    let mut per_layout: std::collections::BTreeMap<String, (usize, Summary)> =
        std::collections::BTreeMap::new();
    let (mut files, mut skipped) = (0, 0);
    for (n, path) in common::corpus_files().iter().enumerate() {
        let Some(p) = common::load_path(path) else {
            continue;
        };
        let v = vba_pcode::version_word(&p.project_stream).unwrap_or(0);
        match differential(&p) {
            Ok(s) => {
                files += 1;
                println!(
                    "#{n} v={v:#x} sys={} procedures {} lowered {} executed {} + {} runs {} + {} sessions {} mismatches {}",
                    p.sys_kind,
                    s.procedures,
                    s.lowered,
                    s.executed[0],
                    s.executed[1],
                    s.runs[0],
                    s.runs[1],
                    s.sessions,
                    s.mismatches
                );
                let e = per_layout
                    .entry(format!("version {v:#x}, platform kind {}", p.sys_kind))
                    .or_default();
                e.0 += 1;
                for t in [&mut e.1, &mut total] {
                    t.procedures += s.procedures;
                    t.lowered += s.lowered;
                    for i in 0..2 {
                        t.executed[i] += s.executed[i];
                        t.runs[i] += s.runs[i];
                    }
                    t.errors_both += s.errors_both;
                    t.sessions += s.sessions;
                    t.mismatches += s.mismatches;
                }
            }
            Err(e) => {
                skipped += 1;
                println!(
                    "#{n} v={v:#x} sys={} skipped: {}",
                    p.sys_kind,
                    e.lines().take(4).collect::<Vec<_>>().join(" / ")
                );
            }
        }
    }
    for (k, (n, t)) in &per_layout {
        println!("{k}: {n} files, {t:?}");
    }
    println!("files compared {files}, skipped {skipped}, total {total:?}");
    assert_eq!(total.mismatches, 0);
}

fn differential(p: &common::Project) -> Result<Summary, String> {
    let inputs: Vec<ModuleInput<'_>> = p
        .modules
        .iter()
        .map(|m| ModuleInput {
            name: &m.name,
            kind: common::role(m),
            cache: &m.cache,
        })
        .collect();
    let options = LowerOptions {
        host_members: common::host_member_names(p),
        project_name: Some(p.name.clone()),
    };
    let lp = lower_project_with_options(&p.project_stream, p.sys_kind, &inputs, &options)
        .map_err(|e| format!("lowering: {e}"))?;
    let total = lp.procedures.len();
    let lowered = lp.lowered_count();
    let src = source_program(p, &lp, &options.host_members)?;
    let cache_ir = Rc::new(lp.ir.clone());
    let class_like_of = |name: &str| {
        p.modules
            .iter()
            .any(|m| m.name == name && m.kind != ModuleKind::Standard)
    };
    // `[std, class-like]`
    let (mut ran, mut runs, mut mismatches, mut errors_both) = ([0usize; 2], [0usize; 2], 0, 0);
    let mut lowered_by = [0usize; 2];
    for r in lp.procedures.iter().filter(|r| r.outcome.is_lowered()) {
        let Some(id) = r.id else { continue };
        let cl = usize::from(class_like_of(&r.module));
        lowered_by[cl] += 1;
        let info = cache_ir.program.proc(id);
        let params: Vec<(Type, bool)> = info
            .params
            .iter()
            .map(|q| (q.ty.clone(), q.optional))
            .collect();
        let sel = ProcSel {
            name: &r.procedure,
            kind: info.kind,
        };
        let full = format!("{}.{}", r.module, r.procedure);
        let mut any = false;
        for args in arg_sets(&params, &cache_ir.program) {
            let a = run_one(&src, &r.module, cl == 1, &sel, &args);
            let b = run_one(&cache_ir, &r.module, cl == 1, &sel, &args);
            runs[cl] += 1;
            any = true;
            if a.starts_with("Err") {
                errors_both += 1;
            }
            if a != b {
                mismatches += 1;
                println!("MISMATCH {full} {args:?}\n  source: {a}\n  cache : {b}");
                if std::env::var_os("DUMP_IR").is_some() {
                    for (label, ir) in [("source", &src), ("cache", &cache_ir)] {
                        if let Some(pid) = find_proc(ir, &r.module, &sel) {
                            let f = ir.function_of(pid);
                            println!(
                                "--- {label} IR\n{}",
                                vba_ir::dump_function(ir, f, vba_ir::DumpOptions::default())
                            );
                        }
                    }
                }
            }
        }
        ran[cl] += usize::from(any);
    }
    // Sessions: every lowered procedure of a module, one after the other on
    // one VM and instance, so that state written by one is read by the next.
    let mut sessions = 0usize;
    for m in &p.modules {
        let cl = m.kind != ModuleKind::Standard;
        let procs: Vec<_> = lp
            .procedures
            .iter()
            .filter(|r| r.module == m.name && r.outcome.is_lowered() && r.id.is_some())
            .collect();
        if procs.is_empty() {
            continue;
        }
        for round in 0..4usize {
            let mut steps = Vec::new();
            for (n, r) in procs.iter().enumerate() {
                let info = cache_ir.program.proc(r.id.expect("id"));
                let params: Vec<(Type, bool)> = info
                    .params
                    .iter()
                    .map(|q| (q.ty.clone(), q.optional))
                    .collect();
                let sets = arg_sets(&params, &cache_ir.program);
                let args = sets[(round + n) % sets.len()].clone();
                steps.push((
                    ProcSel {
                        name: &r.procedure,
                        kind: info.kind,
                    },
                    args,
                ));
            }
            if round % 2 == 1 {
                steps.reverse();
            }
            let a = run_session(&src, &m.name, cl, &steps);
            let b = run_session(&cache_ir, &m.name, cl, &steps);
            sessions += 1;
            if a != b {
                mismatches += 1;
                println!(
                    "SESSION MISMATCH {} round {round}\n  source: {a}\n  cache : {b}",
                    m.name
                );
            }
        }
    }
    Ok(Summary {
        procedures: total,
        lowered,
        executed: ran,
        runs,
        errors_both,
        sessions,
        mismatches,
    })
}
