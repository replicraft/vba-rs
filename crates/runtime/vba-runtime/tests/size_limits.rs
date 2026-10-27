//! Hostile source is refused by the session builder with a typed limit,
//! before anything runs, under `ResourceLimits::untrusted()`.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{HeadlessHost, ResourceLimits};
use vba_runtime::*;
use vba_vm::LimitKind;

fn build_with(limits: ResourceLimits, mods: &[vba_sema::SourceModule]) -> Result<Session, Error> {
    Session::builder()
        .host(Rc::new(HeadlessHost::new().with_limits(limits)))
        .build_sources(mods)
}

fn small() -> Vec<vba_sema::SourceModule> {
    vec![std_module(
        "Public Sub Main()\nDim x As Long\nx = 1\nEnd Sub\n",
    )]
}

#[test]
fn untrusted_limits_carry_finite_size_budgets() {
    let u = ResourceLimits::untrusted();
    assert!(u.max_source_bytes.is_some());
    assert!(u.max_modules.is_some());
    assert!(u.max_analysis_nodes.is_some());
    assert!(u.max_ir_instructions.is_some());
    let none = ResourceLimits::unlimited();
    assert!(none.max_source_bytes.is_none() && none.max_ir_instructions.is_none());
    // An ordinary program passes them.
    assert!(build_with(u, &small()).is_ok());
}

#[test]
fn hostile_source_is_refused_with_a_typed_limit() {
    let base = ResourceLimits::unlimited();

    let big = format!(
        "Public Sub Main()\n{}End Sub\n",
        "Debug.Print 1\n".repeat(5000)
    );
    let r = build_with(base.with_max_source_bytes(Some(1000)), &[std_module(&big)]);
    assert!(matches!(r, Err(Error::Limit(LimitKind::SourceSize))));

    let many: Vec<_> = (0..20)
        .map(|i| module(&format!("M{i}"), vba_project::ModuleKind::Standard, ""))
        .collect();
    let r = build_with(base.with_max_modules(Some(10)), &many);
    assert!(matches!(r, Err(Error::Limit(LimitKind::ModuleCount))));

    let r = build_with(base.with_max_analysis_nodes(Some(100)), &[std_module(&big)]);
    assert!(matches!(r, Err(Error::Limit(LimitKind::AnalysisSize))));

    let r = build_with(
        base.with_max_ir_instructions(Some(100)),
        &[std_module(&big)],
    );
    assert!(matches!(r, Err(Error::Limit(LimitKind::ProgramSize))));

    // The same program loads once the budgets allow it.
    assert!(build_with(base, &[std_module(&big)]).is_ok());
}

#[test]
fn the_untrusted_source_budget_applies_without_other_configuration() {
    let n = vba_host::UNTRUSTED_MAX_SOURCE_BYTES as usize;
    let filler = format!("Public Sub Main()\n'{}\nEnd Sub\n", "x".repeat(n));
    let r = build_with(ResourceLimits::untrusted(), &[std_module(&filler)]);
    assert!(matches!(r, Err(Error::Limit(LimitKind::SourceSize))));
}

#[test]
fn a_refused_project_ran_nothing() {
    let sink = vba_vm::BufferSink::new();
    let big = format!(
        "Public Sub Auto_Open()\nDebug.Print \"ran\"\nEnd Sub\n{}",
        "' padding\n".repeat(1000)
    );
    let r = Session::builder()
        .debug_sink(sink.clone())
        .host(Rc::new(HeadlessHost::new().with_limits(
            ResourceLimits::unlimited().with_max_source_bytes(Some(100)),
        )))
        .build_sources(&[std_module(&big)]);
    assert!(r.is_err());
    assert_eq!(sink.contents(), "");
}
