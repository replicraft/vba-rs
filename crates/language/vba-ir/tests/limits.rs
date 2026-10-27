//! The instruction budget of lowering: a hostile program is cut off with a
//! typed error instead of being lowered without bound.

mod common;

use common::*;
use vba_ir::{LowerError, LowerLimits, lower_with_limits};
use vba_project::ModuleKind;

fn std_module(src: &str) -> vba_sema::SourceModule {
    module("M", ModuleKind::Standard, src)
}

fn instructions(ir: &vba_ir::IrProgram) -> u64 {
    ir.functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .map(|b| b.insts.len() as u64)
        .sum()
}

#[test]
fn a_program_within_the_budget_lowers_identically() {
    let a = analyze(&[std_module("Sub A()\nDim x As Long\nx = 1 + 2\nEnd Sub\n")]);
    assert!(!a.has_errors());
    let plain = vba_ir::lower(&a).unwrap();
    let n = instructions(&plain);
    let exact = lower_with_limits(&a, &LowerLimits::unlimited().with_max_instructions(Some(n)))
        .unwrap();
    assert_eq!(
        vba_ir::dump_program(&plain, vba_ir::DumpOptions::default()),
        vba_ir::dump_program(&exact, vba_ir::DumpOptions::default())
    );
}

#[test]
fn one_instruction_less_than_needed_is_refused() {
    let a = analyze(&[std_module(
        "Sub A()\nDim x As Long\nx = 1 + 2\nx = x * 3\nEnd Sub\n",
    )]);
    let n = instructions(&vba_ir::lower(&a).unwrap());
    assert!(n > 4);
    let r = lower_with_limits(
        &a,
        &LowerLimits::unlimited().with_max_instructions(Some(n - 1)),
    );
    assert!(matches!(
        r,
        Err(LowerError::TooLarge { limit }) if limit == n - 1
    ));
}

#[test]
fn a_huge_body_is_cut_off() {
    let body = "x = x + 1\n".repeat(50_000);
    let a = analyze(&[std_module(&format!(
        "Sub A()\nDim x As Long\n{body}End Sub\n"
    ))]);
    assert!(!a.has_errors());
    let r = lower_with_limits(
        &a,
        &LowerLimits::unlimited().with_max_instructions(Some(1_000)),
    );
    assert!(matches!(r, Err(LowerError::TooLarge { limit: 1_000 })), "{r:?}");
}

#[test]
fn the_budget_is_shared_by_all_procedures() {
    let procs: String = (0..500)
        .map(|i| format!("Sub P{i}()\nDim x As Long\nx = {i}\nEnd Sub\n"))
        .collect();
    let a = analyze(&[std_module(&procs)]);
    assert!(!a.has_errors());
    let n = instructions(&vba_ir::lower(&a).unwrap());
    // Each procedure alone is far below the budget; together they are not.
    let r = lower_with_limits(
        &a,
        &LowerLimits::unlimited().with_max_instructions(Some(n / 2)),
    );
    assert!(matches!(r, Err(LowerError::TooLarge { .. })));
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
fn a_long_expression_is_cut_off() {
    on_big_stack(|| {
        let expr = vec!["1"; 300].join(" + ");
        let a = analyze(&[std_module(&format!(
            "Function F() As Long\nF = {expr}\nEnd Function\n"
        ))]);
        assert!(!a.has_errors());
        let r = lower_with_limits(
            &a,
            &LowerLimits::unlimited().with_max_instructions(Some(100)),
        );
        assert!(matches!(r, Err(LowerError::TooLarge { .. })));
    });
}
