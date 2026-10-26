//! The step budget bounds CPU time and not only the number of statements:
//! built-ins that scan text and code that builds large values pay for the
//! work in steps.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{LimitKind, NoServices, VmError};

fn steps(max: u64) -> ResourceLimits {
    ResourceLimits {
        max_steps: Some(max),
        ..ResourceLimits::default()
    }
}

/// Runs `src` and returns the outcome and the steps it used.
fn counted(src: &str, max: u64) -> (Outcome, u64) {
    let host = Rc::new(HeadlessHost::new().with_limits(steps(max)));
    let o = run_with(
        &[module("M", ModuleKind::Standard, src)],
        "Main",
        vec![],
        host.clone(),
        Box::new(NoServices),
    );
    let used = host.budget().steps_used();
    (o, used)
}

/// Runs `src` and asserts the step limit is what stops it. The limit is the
/// bound on the work done, so no wall-clock time is asserted.
fn stops_with_steps(src: &str, max: u64) {
    let o = run_limited(src, steps(max));
    assert_eq!(
        o.result.as_ref().err(),
        Some(&VmError::Limit(LimitKind::Steps)),
        "output {:?}",
        o.out
    );
}

#[test]
fn a_pathological_like_pattern_runs_into_the_step_limit() {
    // 200k units of "a" against "*" + 2000 "a" + "b": 4e8 comparisons.
    stops_with_steps(
        "Sub Main()\nDim t As String, p As String, r As Boolean\nt = String$(200000, \"a\")\np = \"*\" & String$(2000, \"a\") & \"b\"\nr = t Like p\nDebug.Print r\nEnd Sub\n",
        200_000,
    );
}

#[test]
fn like_costs_steps_in_proportion_to_the_work() {
    let (o, small) = counted(
        "Sub Main()\nDim t As String, r As Boolean\nt = String$(1000, \"a\")\nr = t Like \"*b\"\nEnd Sub\n",
        10_000_000,
    );
    assert!(o.result.is_ok());
    let (o, big) = counted(
        "Sub Main()\nDim t As String, r As Boolean\nt = String$(100000, \"a\")\nr = t Like \"*b\"\nEnd Sub\n",
        10_000_000,
    );
    assert!(o.result.is_ok());
    assert!(big > small + 1_000, "small {small}, big {big}");
}

#[test]
fn instr_with_a_long_needle_is_linear_and_charged() {
    let src = "Sub Main()\nDim h As String, n As String\nh = String$(2000000, \"a\")\nn = String$(20000, \"a\") & \"b\"\nDebug.Print InStr(h, n)\nDebug.Print InStrRev(h, n)\nEnd Sub\n";
    let (o, used) = counted(src, 100_000_000);
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert_eq!(o.out, " 0 \n 0 \n");
    // 2M units scanned twice, 64 units to a step.
    assert!(used > 50_000, "used {used}");
}

#[test]
fn replace_split_and_join_on_large_strings_cost_steps() {
    for body in [
        "x = Replace(s, \"a\", \"b\")",
        "x = Split(s, \"a\")(0)",
        "x = Join(Split(s, \"a\"), \",\")",
        "x = StrComp(s, s & \"b\")",
        "x = Filter(Split(s, \"a\"), \"b\")",
    ] {
        let src = format!(
            "Sub Main()\nDim s As String, x As Variant\ns = String$(300000, \"a\")\n{body}\nEnd Sub\n"
        );
        stops_with_steps(&src, 3_000);
    }
}

#[test]
fn copying_large_strings_costs_steps() {
    // 1 MB copied ten thousand times is 10 GB of work in a few statements.
    stops_with_steps(
        "Sub Main()\nDim s As String, t As String, i As Long\ns = String$(500000, \"x\")\nFor i = 1 To 100000\nt = s\nNext\nEnd Sub\n",
        2_000_000,
    );
}

#[test]
fn copying_large_arrays_costs_steps() {
    stops_with_steps(
        "Sub Main()\nDim a() As Variant, b() As Variant, i As Long\nReDim a(1 To 200000)\nFor i = 1 To 100000\nb = a\nNext\nEnd Sub\n",
        2_000_000,
    );
}

#[test]
fn arrays_nested_absurdly_deep_are_a_limit_not_a_stack_overflow() {
    // Each Array(v) wraps the one before; cloning, comparing and dropping
    // such a chain recurse, so its depth is bounded.
    let o = run_limited(
        "Sub Main()\nDim v As Variant, i As Long\nv = 1\nFor i = 1 To 20000\nv = Array(v)\nNext\nDebug.Print \"built\"\nEnd Sub\n",
        ResourceLimits::unlimited(),
    );
    assert_eq!(
        o.result.as_ref().err(),
        Some(&VmError::Limit(LimitKind::CallDepth)),
        "{:?}",
        o.out
    );
    // Reasonable nesting still works.
    let o = run(
        "Sub Main()\nDim v As Variant, i As Long\nv = 1\nFor i = 1 To 50\nv = Array(v)\nNext\nDebug.Print UBound(v)\nEnd Sub\n",
    );
    assert_eq!(o.out, " 0 \n", "{:?}", o.result);
}

#[test]
fn small_inputs_cost_next_to_nothing() {
    let (o, used) = counted(
        "Sub Main()\nDim i As Long, n As Long\nFor i = 1 To 1000\nn = n + InStr(\"hello world\", \"wor\") + Len(Replace(\"abc\", \"b\", \"xx\"))\nNext\nEnd Sub\n",
        10_000_000,
    );
    assert!(o.result.is_ok());
    // A few instructions per iteration, no surcharge worth mentioning.
    assert!(used < 40_000, "used {used}");
}
