//! Per-call CPU, memory and time allowances for an embedding application
//! that runs many short handlers on one long-lived instance.

use std::time::Duration;

use vba::runtime::Error as RunError;
use vba::{CallLimits, CallTarget, Engine, Error, Project, Variant};

const CODE: &str = "\
Public Sub Spin()
  Do
  Loop
End Sub
Public Function Work(ByVal n As Long) As Long
  Dim i As Long, t As Long
  For i = 1 To n
    t = t + i
  Next
  Work = t
End Function
Public Function Big(ByVal n As Long) As Long
  Dim s As String
  s = String$(n, \"x\")
  Big = Len(s)
End Function
";

fn instance(engine: &Engine) -> vba::Instance {
    let mut inst = engine.load(&Project::from_source("Module1", CODE)).unwrap();
    inst.trust();
    inst
}

#[test]
fn step_allowance_applies_to_each_call_and_leaves_the_instance_usable() {
    let mut inst = instance(&Engine::new());
    let limits = CallLimits::new().max_steps(5_000);
    // Each call gets the whole allowance, so many calls add up to more.
    for _ in 0..20 {
        let r = inst.with_call_limits(limits, |i| i.run("Work", vec![Variant::Long(100)]));
        assert_eq!(r.unwrap().value, Variant::Long(5050));
    }
    let err = inst
        .with_call_limits(limits, |i| i.run("Spin", vec![]))
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    // The allowance was the call's, not the instance's: no reset needed.
    assert!(inst.is_usable());
    assert_eq!(
        inst.run("Work", vec![Variant::Long(10)]).unwrap().value,
        Variant::Long(55)
    );
}

#[test]
fn the_engine_limit_still_caps_a_generous_call_allowance() {
    let mut inst = instance(&Engine::new().max_steps(2_000));
    let err = inst
        .with_call_limits(CallLimits::new().max_steps(1_000_000), |i| {
            i.run("Spin", vec![])
        })
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
}

#[test]
fn memory_headroom_is_per_call() {
    let mut inst = instance(&Engine::new());
    let tight = CallLimits::new().max_memory(64 * 1024);
    let ok = inst.with_call_limits(tight, |i| i.run("Big", vec![Variant::Long(1_000)]));
    assert_eq!(ok.unwrap().value, Variant::Long(1_000));
    let err = inst
        .with_call_limits(tight, |i| i.run("Big", vec![Variant::Long(5_000_000)]))
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    // Without the call limit the same request is within the engine's budget.
    let r = inst.run("Big", vec![Variant::Long(5_000_000)]).unwrap();
    assert_eq!(r.value, Variant::Long(5_000_000));
}

#[test]
fn a_timeout_stops_a_loop_without_touching_the_token() {
    let mut inst = instance(&Engine::new());
    let err = inst
        .with_call_limits(CallLimits::new().timeout(Duration::from_millis(20)), |i| {
            i.run("Spin", vec![])
        })
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert_eq!(err.kind(), "cancelled");
    assert!(inst.is_usable());
    assert_eq!(
        inst.run("Work", vec![Variant::Long(3)]).unwrap().value,
        Variant::Long(6)
    );
}

#[test]
fn limits_nest_and_the_inner_scope_restores_the_outer_one() {
    let mut inst = instance(&Engine::new());
    let outer = CallLimits::new().max_steps(100_000);
    let inner = CallLimits::new().max_steps(500);
    let r = inst.with_call_limits(outer, |i| {
        let spun = i.with_call_limits(inner, |i| {
            i.call(CallTarget::Project, "Spin", vec![]).is_err()
        });
        (spun, i.run("Work", vec![Variant::Long(100)]).is_ok())
    });
    assert_eq!(r, (true, true));
}

#[test]
fn the_instance_hands_out_its_cancel_token() {
    let engine = Engine::new();
    let inst = instance(&engine);
    inst.cancel_token().cancel();
    assert!(engine.cancel_token().is_cancelled());
    assert!(!inst.is_usable());
}
