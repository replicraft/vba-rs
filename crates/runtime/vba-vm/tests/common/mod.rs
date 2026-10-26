//! Shared helpers: compile source to IR and run it in a VM.

#![allow(dead_code)]

use std::rc::Rc;

use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_ir::IrProgram;
use vba_project::ModuleKind;
use vba_sema::SourceModule;
use vba_vm::{Arg, BufferSink, NoServices, RunOutput, Services, Vm, VmConfig, VmError};

pub fn module(name: &str, kind: ModuleKind, src: &str) -> SourceModule {
    SourceModule {
        name: name.to_owned(),
        kind,
        source: src.as_bytes().to_vec(),
    }
}

pub fn compile(mods: &[SourceModule]) -> Rc<IrProgram> {
    compile_with(mods, &vba_sema::NoLibraries)
}

pub fn compile_with(mods: &[SourceModule], provider: &dyn vba_sema::TypeProvider) -> Rc<IrProgram> {
    let a = vba_sema::analyze_sources(
        mods,
        vba_profile::default_profile(),
        provider,
        &vba_sema::Options::default(),
    );
    let errors: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .map(|d| vba_diagnostics::render_plain(d, &a.sources))
        .collect();
    assert!(errors.is_empty(), "compile errors:\n{}", errors.join("\n"));
    let ir = vba_ir::lower(&a).expect("lowering");
    vba_ir::verify(&ir).expect("verified IR");
    Rc::new(ir)
}

pub struct Outcome {
    pub out: String,
    pub result: Result<RunOutput, VmError>,
    pub vm: Vm,
}

pub fn run_with(
    mods: &[SourceModule],
    entry: &str,
    args: Vec<Arg>,
    host: Rc<dyn Host>,
    services: Box<dyn Services>,
) -> Outcome {
    run_full(mods, entry, args, host, services, &vba_sema::NoLibraries)
}

pub fn run_full(
    mods: &[SourceModule],
    entry: &str,
    args: Vec<Arg>,
    host: Rc<dyn Host>,
    services: Box<dyn Services>,
    provider: &dyn vba_sema::TypeProvider,
) -> Outcome {
    let ir = compile_with(mods, provider);
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        ir,
        host,
        services,
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    let p = vm.find_procedure(entry).expect("entry procedure");
    let result = vm.run_procedure(p, args);
    Outcome {
        out: sink.contents(),
        result,
        vm,
    }
}

/// Runs `Main` of a single standard module.
pub fn run(src: &str) -> Outcome {
    run_limited(
        src,
        ResourceLimits {
            max_steps: Some(2_000_000),
            ..ResourceLimits::default()
        },
    )
}

pub fn run_limited(src: &str, limits: ResourceLimits) -> Outcome {
    run_with(
        &[module("M", ModuleKind::Standard, src)],
        "Main",
        vec![],
        Rc::new(HeadlessHost::new().with_limits(limits)),
        Box::new(NoServices),
    )
}

/// The output of a program that must complete.
pub fn output(src: &str) -> String {
    let o = run(src);
    if let Err(e) = &o.result {
        panic!("program failed: {e}\noutput so far:\n{}", o.out);
    }
    o.out
}

/// A program and the text it must print.
pub struct Case {
    pub name: &'static str,
    pub src: &'static str,
    pub out: &'static str,
}

pub const fn case(name: &'static str, src: &'static str, out: &'static str) -> Case {
    Case { name, src, out }
}

/// Runs every case (a `Main` in a standard module `M`) and compares the
/// `Debug.Print` text; reports all failures together.
pub fn check(cases: &[Case]) {
    let mut failures = Vec::new();
    for c in cases {
        let o = match std::panic::catch_unwind(|| run(c.src)) {
            Ok(o) => o,
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                failures.push(format!("{}: {msg}", c.name));
                continue;
            }
        };
        match &o.result {
            Ok(_) if o.out == c.out => {}
            Ok(_) => failures.push(format!("{}: expected {:?}, got {:?}", c.name, c.out, o.out)),
            Err(e) => failures.push(format!("{}: failed with {e}; output {:?}", c.name, o.out)),
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// A multi-module program: `(name, is_class, source)` triples.
pub struct MCase {
    pub name: &'static str,
    pub modules: &'static [(&'static str, bool, &'static str)],
    pub out: &'static str,
}

pub const fn mcase(
    name: &'static str,
    modules: &'static [(&'static str, bool, &'static str)],
    out: &'static str,
) -> MCase {
    MCase { name, modules, out }
}

/// Runs `Main` of the module named `Main` (or the first standard module).
pub fn check_modules(cases: &[MCase]) {
    let mut failures = Vec::new();
    for c in cases {
        let r = std::panic::catch_unwind(|| {
            let mods: Vec<SourceModule> = c
                .modules
                .iter()
                .map(|(n, class, src)| {
                    module(
                        n,
                        if *class {
                            ModuleKind::Class
                        } else {
                            ModuleKind::Standard
                        },
                        src,
                    )
                })
                .collect();
            run_with(
                &mods,
                "Main",
                vec![],
                Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
                    max_steps: Some(2_000_000),
                    ..ResourceLimits::default()
                })),
                Box::new(NoServices),
            )
        });
        match r {
            Ok(o) => match &o.result {
                Ok(_) if o.out == c.out => {}
                Ok(_) => {
                    failures.push(format!("{}: expected {:?}, got {:?}", c.name, c.out, o.out))
                }
                Err(e) => failures.push(format!("{}: failed with {e}; output {:?}", c.name, o.out)),
            },
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                failures.push(format!("{}: {msg}", c.name));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
