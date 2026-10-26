//! The interpreter nests itself on the native stack for initializers, terminators, event
//! handlers, default members and re-entrant host calls. All of those draw on one budget, so a
//! hostile program ends in a limit error on a small thread stack, whatever the call-depth limit.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{HeadlessHost, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{BufferSink, LimitKind, NoServices, Vm, VmConfig, VmError};

type Mods = Vec<(&'static str, ModuleKind, &'static str)>;

/// A thread stack that holds the default budget (1 MiB of nesting) with room for the
/// frames below the first run.
const ROOMY_STACK: usize = 2 * 1024 * 1024;

fn run_on_stack(mods: Mods, stack: usize, budget: Option<usize>) -> Result<(), VmError> {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let mods: Vec<_> = mods.iter().map(|(n, k, s)| module(n, *k, s)).collect();
            // Lift the count so that only the byte budget is under test.
            let default = VmConfig::default();
            let cfg = VmConfig {
                max_native_depth: u32::MAX,
                max_native_stack_bytes: budget.unwrap_or(default.max_native_stack_bytes),
                ..default
            };
            let host = Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
                max_steps: Some(50_000_000),
                max_call_depth: 1_000_000,
                ..ResourceLimits::default()
            }));
            let mut vm = Vm::new(
                compile(&mods),
                host,
                Box::new(NoServices),
                Box::new(vba_stdlib::FixedHost::default()),
                Box::new(BufferSink::new()),
                cfg,
            )
            .expect("vm");
            let p = vm.find_procedure("Main").expect("Main");
            vm.run_procedure(p, vec![]).map(|_| ())
        })
        .unwrap()
        .join()
        .expect("the run must not overflow the native stack")
}

/// The run ends because of the depth: either as the untrappable limit or as
/// VBA error 28, "Out of stack space".
fn expect_depth_limit(mods: Mods) {
    // The default budget on a stack that holds it, then a small budget on a small stack.
    for (stack, budget) in [(ROOMY_STACK, None), (512 * 1024, Some(96 * 1024))] {
        expect_depth_limit_on(mods.clone(), stack, budget);
    }
}

fn expect_depth_limit_on(mods: Mods, stack: usize, budget: Option<usize>) {
    match run_on_stack(mods, stack, budget) {
        Err(VmError::Limit(LimitKind::CallDepth)) => {}
        Err(VmError::Runtime(e)) if e.error.number == 28 => {}
        other => panic!("not a depth limit: {other:?}"),
    }
}

#[test]
fn initializer_recursion_stays_inside_a_small_stack() {
    expect_depth_limit(vec![
        (
            "Rec",
            ModuleKind::Class,
            "Public Child As Rec\nPrivate Sub Class_Initialize()\nSet Child = New Rec\nEnd Sub\n",
        ),
        (
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim r As Rec\nSet r = New Rec\nEnd Sub\n",
        ),
    ]);
}

#[test]
fn terminator_recursion_stays_inside_a_small_stack() {
    expect_depth_limit(vec![
        (
            "Rec",
            ModuleKind::Class,
            "Private Sub Class_Terminate()\nDim r As Rec\nSet r = New Rec\nSet r = Nothing\nEnd Sub\n",
        ),
        (
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim r As Rec\nSet r = New Rec\nSet r = Nothing\nEnd Sub\n",
        ),
    ]);
}

#[test]
fn default_member_recursion_stays_inside_a_small_stack() {
    expect_depth_limit(vec![
        (
            "Rec",
            ModuleKind::Class,
            "Public Property Get Value() As Variant\nAttribute Value.VB_UserMemId = 0\nDim r As New Rec\nSet Value = r\nEnd Property\n",
        ),
        (
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim r As New Rec\nDebug.Print r & \"\"\nEnd Sub\n",
        ),
    ]);
}

#[test]
fn late_bound_and_event_recursion_share_the_budget() {
    expect_depth_limit(vec![
        (
            "Src",
            ModuleKind::Class,
            "Public Event Ping(ByVal n As Long)\nPublic Sub Fire(n As Long)\nRaiseEvent Ping(n)\nEnd Sub\n",
        ),
        (
            "Sink",
            ModuleKind::Class,
            "Public WithEvents S As Src\nPrivate Sub S_Ping(ByVal n As Long)\nDim o As Object\nSet o = Me\nCallByName o, \"Again\", VbMethod, n\nEnd Sub\nPublic Sub Again(ByVal n As Long)\nS.Fire n + 1\nEnd Sub\n",
        ),
        (
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim s As New Src, k As New Sink\nSet k.S = s\ns.Fire 0\nEnd Sub\n",
        ),
    ]);
}
