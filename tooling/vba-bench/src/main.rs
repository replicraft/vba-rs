//! Timing harness for the interpreter: compiles each program under
//! `programs/`, runs it repeatedly and reports steps, wall time and time per
//! step. The step count and an output checksum are printed so a change that
//! alters accounting or behaviour is visible next to the timings.
//!
//! `vba-bench [--runs N] [filter]`

use std::rc::Rc;
use std::time::Instant;

use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig};

const PROGRAMS: &[(&str, &str)] = &[
    ("int_loop", include_str!("../programs/int_loop.bas")),
    ("string_build", include_str!("../programs/string_build.bas")),
    ("array_sort", include_str!("../programs/array_sort.bas")),
    ("collection_calls", include_str!("../programs/collection_calls.bas")),
    ("recursion", include_str!("../programs/recursion.bas")),
    ("variant_math", include_str!("../programs/variant_math.bas")),
    ("format_val", include_str!("../programs/format_val.bas")),
];

fn compile(src: &str) -> Rc<vba_ir::IrProgram> {
    let a = vba_sema::analyze_sources(
        &[vba_sema::SourceModule {
            name: "M".to_owned(),
            kind: ModuleKind::Standard,
            source: src.as_bytes().to_vec(),
        }],
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    assert!(!a.has_errors(), "benchmark program does not compile");
    let ir = vba_ir::lower(&a).expect("lowering");
    vba_ir::verify(&ir).expect("verified IR");
    Rc::new(ir)
}

/// One execution: (steps, output).
fn run_once(ir: &Rc<vba_ir::IrProgram>) -> (u64, String) {
    let host = Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(500_000_000),
        ..ResourceLimits::default()
    }));
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        ir.clone(),
        host.clone(),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    let main = vm.find_procedure("Main").expect("Main");
    vm.run_procedure(main, vec![]).expect("benchmark runs");
    (host.budget().steps_used(), sink.contents())
}

fn checksum(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn main() {
    let mut runs = 7usize;
    let mut filter = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--runs" {
            runs = args.next().and_then(|n| n.parse().ok()).expect("--runs N");
        } else {
            filter = Some(a);
        }
    }
    println!(
        "{:<18} {:>11} {:>10} {:>10} {:>8}  {:<16}",
        "program", "steps", "min ms", "median ms", "ns/step", "output fnv"
    );
    for (name, src) in PROGRAMS {
        if filter.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        let ir = compile(src);
        let (steps, out) = run_once(&ir);
        let mut times = Vec::new();
        for _ in 0..runs {
            let t = Instant::now();
            let (s, o) = run_once(&ir);
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            assert_eq!((s, &o), (steps, &out), "{name} is not deterministic");
        }
        times.sort_by(f64::total_cmp);
        let (min, med) = (times[0], times[times.len() / 2]);
        println!(
            "{name:<18} {steps:>11} {min:>10.2} {med:>10.2} {:>8.1}  {:016x}",
            min * 1e6 / steps as f64,
            checksum(&out)
        );
    }
}
