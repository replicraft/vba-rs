//! Size budgets of the analysis: hostile projects are refused or cut off
//! with a typed limit, never analysed without bound.

mod common;

use common::*;
use vba_sema::{AnalysisLimit, AnalysisLimits, Options, analyze_sources, codes};

fn limited(mods: &[vba_sema::SourceModule], limits: AnalysisLimits) -> vba_sema::Analysis {
    let opts = Options {
        limits,
        ..Options::default()
    };
    analyze_sources(mods, &profile64(), &test_library(), &opts)
}

fn limit_errors(a: &vba_sema::Analysis) -> usize {
    a.diagnostics
        .iter()
        .filter(|d| d.code == codes::LIMIT_EXCEEDED)
        .count()
}

#[test]
fn unlimited_by_default() {
    let src = "Sub A()\nDim x As Long\nx = 1\nEnd Sub\n";
    let a = analyze_sources(
        &[bas("M", src)],
        &profile64(),
        &test_library(),
        &Options::default(),
    );
    assert!(a.limit_exceeded.is_none());
    assert!(!a.has_errors());
}

#[test]
fn too_much_source_is_refused_before_parsing() {
    let big = format!("Sub A()\n{}End Sub\n", "x = 1\n".repeat(10_000));
    let a = limited(
        &[bas("M", &big)],
        AnalysisLimits::unlimited().with_max_source_bytes(Some(1000)),
    );
    assert!(matches!(
        a.limit_exceeded,
        Some(AnalysisLimit::SourceBytes { limit: 1000, actual }) if actual == big.len() as u64
    ));
    assert_eq!(limit_errors(&a), 1);
    assert!(a.program.modules.is_empty(), "nothing was parsed or bound");
    // The diagnostic renders without panicking.
    assert!(render(&a).contains("SEM098"));
}

#[test]
fn source_budget_counts_all_modules() {
    let mods: Vec<_> = (0..10)
        .map(|i| bas(&format!("M{i}"), "Sub A()\nEnd Sub\n"))
        .collect();
    let total: u64 = mods.iter().map(|m| m.source.len() as u64).sum();
    let ok = limited(
        &mods,
        AnalysisLimits::unlimited().with_max_source_bytes(Some(total)),
    );
    assert!(ok.limit_exceeded.is_none());
    let over = limited(
        &mods,
        AnalysisLimits::unlimited().with_max_source_bytes(Some(total - 1)),
    );
    assert!(matches!(
        over.limit_exceeded,
        Some(AnalysisLimit::SourceBytes { .. })
    ));
}

#[test]
fn too_many_modules_are_refused() {
    let mods: Vec<_> = (0..50).map(|i| bas(&format!("M{i}"), "")).collect();
    let a = limited(&mods, AnalysisLimits::unlimited().with_max_modules(Some(49)));
    assert_eq!(
        a.limit_exceeded,
        Some(AnalysisLimit::Modules {
            limit: 49,
            actual: 50
        })
    );
    let ok = limited(&mods, AnalysisLimits::unlimited().with_max_modules(Some(50)));
    assert!(ok.limit_exceeded.is_none());
}

#[test]
fn a_huge_body_stops_binding_at_the_node_budget() {
    let body = "x = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8\n".repeat(20_000);
    let src = format!("Sub A()\nDim x As Long\n{body}End Sub\n");
    let a = limited(
        &[bas("M", &src)],
        AnalysisLimits::unlimited().with_max_nodes(Some(5_000)),
    );
    assert_eq!(
        a.limit_exceeded,
        Some(AnalysisLimit::Nodes { limit: 5_000 })
    );
    assert_eq!(limit_errors(&a), 1);
    assert!(a.has_errors());
    // Binding really stopped: far fewer statements were bound than written.
    let bound = a.program.procs[0].body.len();
    assert!(bound < 5_000, "{bound} statements bound");
}

/// Analysis and lowering recurse in proportion to the depth of an
/// expression; a debug build needs more than a test thread's 2 MiB for a few
/// hundred operators (the documented advice is a large stack for untrusted
/// source).
fn on_big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn a_long_expression_stops_at_the_node_budget() {
    on_big_stack(|| {
        let expr = vec!["1"; 300].join(" + ");
        let src = format!("Function F() As Long\nF = {expr}\nEnd Function\n");
        let a = limited(
            &[bas("M", &src)],
            AnalysisLimits::unlimited().with_max_nodes(Some(100)),
        );
        assert!(matches!(
            a.limit_exceeded,
            Some(AnalysisLimit::Nodes { .. })
        ));
    });
}

#[test]
fn the_node_budget_stops_later_procedures_too() {
    let procs: String = (0..2000)
        .map(|i| format!("Sub P{i}()\nDim x As Long\nx = {i}\nEnd Sub\n"))
        .collect();
    let a = limited(
        &[bas("M", &procs)],
        AnalysisLimits::unlimited().with_max_nodes(Some(300)),
    );
    assert!(a.limit_exceeded.is_some());
    // The ones after the cut have no body.
    assert!(a.program.procs.last().unwrap().body.is_empty());
}

#[test]
fn a_program_within_the_budget_is_untouched() {
    let src = "Sub A()\nDim x As Long\nx = 1 + 2\nEnd Sub\n";
    let a = limited(
        &[bas("M", src)],
        AnalysisLimits::unlimited()
            .with_max_nodes(Some(1000))
            .with_max_source_bytes(Some(1000))
            .with_max_modules(Some(1)),
    );
    assert!(a.limit_exceeded.is_none());
    assert!(!a.has_errors());
}
