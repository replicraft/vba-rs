//! Cancellation, budgets and the state of an instance after a fatal fault.

mod common;

use std::rc::Rc;
use std::time::Duration;

use common::*;
use vba::runtime::Error as RunError;
use vba::{CallLimits, CallTarget, Engine, Error, ModuleKind, Project, Variant};

const FORM: &str = "\
Public Counter As Long
Private Sub Class_Terminate()
  Debug.Print \"terminated\"
End Sub
Public Sub Spin()
  Do
    Counter = Counter + 1
  Loop
End Sub
Public Sub Work()
  Dim i As Long
  For i = 1 To 200
    Counter = Counter + 1
  Next
End Sub
Public Sub Quick()
  Counter = Counter + 1
End Sub
";

fn engine_project() -> Project {
    let mut p = Project::new();
    p.add_module("Form_Work", ModuleKind::Document, FORM)
        .unwrap();
    p.add_module(
        "Module1",
        ModuleKind::Standard,
        "Public Total As Long\nPublic Sub Bump()\n  Total = Total + 1\nEnd Sub\nPublic Sub Handled()\n  On Error Resume Next\n  Dim x As Long\n  x = 1 \\ 0\nEnd Sub\nPublic Sub Burn()\n  Dim i As Long\n  For i = 1 To 100000\n  Next\nEnd Sub\n",
    )
    .unwrap();
    p
}

fn number(e: &Error) -> i32 {
    match e {
        Error::Run(RunError::Vba(r) | RunError::Host(r)) => r.number,
        other => panic!("not a VBA error: {other:?}"),
    }
}

#[test]
fn a_token_cancels_a_tight_loop_from_another_thread() {
    let engine = Engine::new();
    let token = engine.cancel_token();
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    let form = inst.create_object("Form_Work", None).unwrap();
    assert!(inst.is_usable());
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        token.cancel();
    });
    // No DoEvents anywhere in the loop: the interpreter polls the token.
    let err = inst.call_on(form, "Spin", vec![]).unwrap_err();
    canceller.join().unwrap();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert_eq!(err.kind(), "cancelled");
    // The token is still set: nothing runs until the host resets it.
    assert!(!inst.is_usable());
    let again = inst.call_on(form, "Quick", vec![]).unwrap_err();
    assert!(
        matches!(again, Error::Run(RunError::Cancelled(_))),
        "{again:?}"
    );
    engine.cancel_token().reset();
    assert!(inst.is_usable());
    inst.call_on(form, "Quick", vec![]).unwrap();
    // The loop ran for a while before it was stopped, and the state it
    // changed is still there.
    let n = inst.get_member(form, "Counter").unwrap();
    assert!(matches!(n, Variant::Long(n) if n > 1), "{n:?}");
}

#[test]
fn a_cancelled_token_stops_a_call_before_it_starts() {
    let engine = Engine::new();
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    engine.cancel_token().cancel();
    let err = inst.call(CallTarget::Project, "Bump", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    engine.cancel_token().reset();
    inst.call(CallTarget::Project, "Bump", vec![]).unwrap();
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(1));
}

#[test]
fn the_step_allowance_belongs_to_the_instance_and_spans_its_calls() {
    let engine = Engine::new().max_steps(3_000);
    let mut a = engine.load(&engine_project()).unwrap();
    let mut b = engine.load(&engine_project()).unwrap();
    a.trust();
    b.trust();
    let form = a.create_object("Form_Work", None).unwrap();
    // Each Work call spends a few hundred steps: the allowance runs out over
    // several calls, not within one.
    let mut calls = 0;
    let err = loop {
        match a.call_on(form, "Work", vec![]) {
            Ok(_) => calls += 1,
            Err(e) => break e,
        }
        assert!(calls < 100, "never ran out");
    };
    assert!(calls >= 1, "the first call fits");
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    assert!(!a.is_usable());
    // Another instance of the same engine has its own allowance.
    assert!(b.is_usable());
    b.call(CallTarget::Project, "Bump", vec![]).unwrap();
    // A fresh allowance for the exhausted one, state kept.
    a.reset_step_counter();
    assert!(a.is_usable());
    let kept = a.get_member(form, "Counter").unwrap();
    assert!(matches!(kept, Variant::Long(n) if n > 0), "{kept:?}");
    a.call_on(form, "Quick", vec![]).unwrap();
}

#[test]
fn a_limit_in_one_handler_leaves_the_instance_consistent() {
    let engine = Engine::new().max_steps(5_000);
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    let err = inst.call(CallTarget::Project, "Burn", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    assert!(!inst.is_usable());
    inst.reset_step_counter();
    assert!(inst.is_usable());
    inst.call(CallTarget::Project, "Bump", vec![]).unwrap();
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(1));
}

#[test]
fn reset_discards_state_objects_and_the_budget() {
    let st = FormState::new(&[("Caption", Variant::from("x"))]);
    let engine = Engine::new().max_steps(3_000);
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    inst.call(CallTarget::Project, "Bump", vec![]).unwrap();
    let form = inst
        .create_object("Form_Work", Some(FormHost::new(&st)))
        .unwrap();
    inst.call_on(form, "Work", vec![]).unwrap();
    inst.call(CallTarget::Project, "Handled", vec![]).unwrap();
    assert!(inst.last_error().is_some());
    let live = inst.session().host().objects().live_count();
    assert!(live >= 1);
    // Run into the limit.
    let _ = inst.call(CallTarget::Project, "Burn", vec![]).unwrap_err();
    assert!(!inst.is_usable());

    inst.reset().unwrap();
    assert!(inst.is_usable());
    assert!(inst.last_error().is_none(), "Err starts over");
    // Module variables are back to their initial values...
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(0));
    // ...instances are gone, without Class_Terminate...
    let err = inst.call_on(form, "Quick", vec![]).unwrap_err();
    assert_eq!(number(&err), 91);
    assert!(!inst.output().contains("terminated"), "{}", inst.output());
    // ...the host object the instance held was released...
    assert!(inst.session().host().objects().live_count() < live);
    // ...and the budget starts over: new instances work.
    let form = inst.create_object("Form_Work", None).unwrap();
    inst.call_on(form, "Work", vec![]).unwrap();
    inst.destroy_object(form).unwrap();
    assert!(inst.output().ends_with("terminated\n"));
}

#[test]
fn a_fatal_fault_in_nested_code_leaves_the_instance_usable_after_a_new_allowance() {
    let app = Rc::new(AppState::default());
    let engine = app_engine(&app).max_steps(50_000);
    let mut inst = load_app(
        &engine,
        &[
            (
                "Form_Spin",
                ModuleKind::Document,
                "Private Sub Form_Load()\n  Do\n  Loop\nEnd Sub\n",
            ),
            (
                "Module1",
                ModuleKind::Standard,
                "Public Total As Long\nPublic Sub Go()\n  On Error Resume Next\n  DoCmd.OpenForm \"Spin\"\n  Total = 99\nEnd Sub\nPublic Sub Fine()\n  Total = 7\nEnd Sub\n",
            ),
        ],
    );
    let err = inst.call(CallTarget::Project, "Go", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    // The statements after the failed one never ran.
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(0));
    inst.reset_step_counter();
    inst.call(CallTarget::Project, "Fine", vec![]).unwrap();
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(7));
}

#[test]
fn engines_allow_a_minute_per_call_unless_unlimited() {
    assert_eq!(
        Engine::new().configured_call_timeout(),
        Some(Duration::from_secs(60))
    );
    assert_eq!(Engine::new().unlimited().configured_call_timeout(), None);
    assert_eq!(
        Engine::new().call_timeout(None).configured_call_timeout(),
        None
    );
}

#[test]
fn the_default_time_allowance_ends_a_loop_that_the_step_limit_would_let_run() {
    // Plenty of steps: only the clock can stop it in good time.
    let engine = Engine::new()
        .max_steps(u64::MAX / 2)
        .call_timeout(Some(Duration::from_millis(100)));
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    let form = inst.create_object("Form_Work", None).unwrap();
    let started = std::time::Instant::now();
    let err = inst.call_on(form, "Spin", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(30));
    // The allowance belongs to the call: the token is untouched and the next
    // call gets a fresh one.
    assert!(inst.is_usable());
    inst.call_on(form, "Quick", vec![]).unwrap();
}

#[test]
fn a_scope_timeout_replaces_the_default_allowance() {
    let engine = Engine::new()
        .max_steps(u64::MAX / 2)
        .call_timeout(Some(Duration::from_millis(1)));
    let mut inst = engine.load(&engine_project()).unwrap();
    inst.trust();
    let form = inst.create_object("Form_Work", None).unwrap();
    // 200 iterations fit easily in a generous scope even though the default
    // would be spent in a millisecond.
    inst.with_call_limits(CallLimits::new().timeout(Duration::from_secs(60)), |i| {
        i.call_on(form, "Work", vec![])
    })
    .unwrap();
}

#[test]
fn a_long_library_call_is_cancelled_by_the_time_allowance() {
    // Each Replace scans 4 million units; the allowance ends the run long
    // before the 100000 repetitions are done.
    let project = Project::from_source(
        "Module1",
        "Public Sub Main()\n  Dim s As String, x As String, i As Long\n  s = String$(4000000, \"a\")\n  For i = 1 To 100000\n    x = Replace(s, \"a\", \"b\")\n  Next\nEnd Sub\n",
    );
    let engine = Engine::new()
        .max_steps(u64::MAX / 2)
        .call_timeout(Some(Duration::from_millis(200)));
    let mut inst = engine.load(&project).unwrap();
    inst.trust();
    let started = std::time::Instant::now();
    let err = inst.run("Main", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn from_session_has_the_default_allowance() {
    // A hand-built session on a host without any limits: the instance still
    // bounds every call in time, and opting out is explicit.
    let engine = Engine::new().unlimited();
    let (builder, sink) = engine.session_builder();
    let session = builder.build_project(engine_project().model()).unwrap();
    let mut inst = vba::Instance::from_session(session, sink, engine.cancel_token());
    assert_eq!(inst.call_timeout(), Some(Duration::from_secs(60)));
    inst.default_call_timeout(None);
    assert_eq!(inst.call_timeout(), None);
    inst.default_call_timeout(Some(Duration::from_millis(100)));
    inst.trust();
    let form = inst.create_object("Form_Work", None).unwrap();
    let started = std::time::Instant::now();
    let err = inst.call_on(form, "Spin", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(30));
}

#[test]
fn from_session_with_the_default_allowance_runs_ordinary_calls() {
    let engine = Engine::new().unlimited();
    let (builder, sink) = engine.session_builder();
    let session = builder.build_project(engine_project().model()).unwrap();
    let mut inst = vba::Instance::from_session(session, sink, engine.cancel_token());
    inst.trust();
    inst.run("Bump", vec![]).unwrap();
    assert!(inst.is_usable());
}
