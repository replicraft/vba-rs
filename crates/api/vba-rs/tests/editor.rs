//! Editor services: the debugger, the immediate window (`eval`), hot edits
//! and the "Run Macro" list.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vba::runtime::Error as RunError;
use vba::{
    Capability, DebugContext, Engine, Error, EvalScope, Instance, ModuleEdit, ModuleKind,
    PauseReason, Project, ResumeAction, RunStatus, Variant,
};

const MAIN: &str = "\
Public Counter As Long
Private secret As String
Public Function Add(ByVal a As Long, ByVal b As Long) As Long
  Dim t As Long
  t = a + b
  Add = t
End Function
Public Sub Main()
  Dim r As Long
  secret = \"s\"
  r = Add(1, 2)
  Counter = r
End Sub
";

fn project(sources: &[(&str, ModuleKind, &str)]) -> Project {
    let mut p = Project::new();
    for (name, kind, src) in sources {
        p.add_module(name, *kind, &src.replace('\n', "\r\n"))
            .unwrap();
    }
    p
}

fn load(sources: &[(&str, ModuleKind, &str)]) -> Instance {
    let mut inst = Engine::new().load(&project(sources)).unwrap();
    inst.trust();
    inst
}

fn one(src: &str) -> Instance {
    load(&[("Module1", ModuleKind::Standard, src)])
}

type Log = Rc<RefCell<Vec<String>>>;

/// A hook that records `reason module.proc:line` and answers from a script
/// (`Continue` once the script is used up).
fn scripted(
    log: &Log,
    script: Vec<ResumeAction>,
) -> impl FnMut(&mut DebugContext<'_>) -> ResumeAction + use<> {
    let log = log.clone();
    let mut script = script.into_iter();
    move |ctx: &mut DebugContext<'_>| {
        let f = &ctx.frames()[0];
        log.borrow_mut().push(format!(
            "{:?} {}.{}:{}",
            ctx.reason(),
            f.module,
            f.procedure,
            f.line
        ));
        script.next().unwrap_or(ResumeAction::Continue)
    }
}

fn steps(inst: &Instance) -> u64 {
    inst.session().host().budget().steps_used()
}

// ---- debugger ------------------------------------------------------------------------

#[test]
fn a_breakpoint_pauses_with_the_stack_locals_and_module_variables() {
    let mut inst = one(MAIN);
    let set = inst.set_breakpoints(&[("module1", 5)]);
    assert_eq!(set[0].line, Some(5));
    let seen: Rc<RefCell<Vec<String>>> = Rc::default();
    let log = seen.clone();
    inst.attach_debugger(move |ctx: &mut DebugContext<'_>| {
        assert_eq!(ctx.reason(), PauseReason::Breakpoint);
        let frames = ctx.frames();
        let names: Vec<String> = frames
            .iter()
            .map(|f| format!("{}.{}:{}", f.module, f.procedure, f.line))
            .collect();
        log.borrow_mut().push(names.join(" < "));
        let locals = ctx.locals(0);
        let show: Vec<String> = locals
            .iter()
            .map(|v| format!("{} {} {:?} {:?}", v.name, v.type_name, v.value, v.kind))
            .collect();
        log.borrow_mut().push(show.join("; "));
        // The caller's frame.
        let caller: Vec<String> = ctx.locals(1).iter().map(|v| v.name.clone()).collect();
        log.borrow_mut().push(caller.join(","));
        let globals: Vec<String> = ctx
            .module_variables("Module1")
            .iter()
            .map(|v| format!("{} {} {:?}", v.name, v.type_name, v.value))
            .collect();
        log.borrow_mut().push(globals.join("; "));
        ResumeAction::Continue
    });
    inst.run("Main", vec![]).unwrap();
    let seen = seen.borrow();
    assert_eq!(seen[0], "Module1.Add:5 < Module1.Main:11");
    assert_eq!(
        seen[1],
        "a Long Long(1) Parameter; b Long Long(2) Parameter; Add Long Long(0) Result; t Long Long(0) Local"
    );
    assert_eq!(seen[2], "r");
    assert_eq!(
        seen[3],
        "Counter Long Long(0); secret String String(VbaString(\"s\"))"
    );
}

#[test]
fn a_breakpoint_on_a_line_without_code_moves_or_is_dropped() {
    let mut inst = one(MAIN);
    let set = inst.set_breakpoints(&[
        ("Module1", 4),
        ("Module1", 7),
        ("Module1", 1),
        ("Nope", 3),
        ("Module1", 99),
    ]);
    // `Dim t As Long` holds no statement: the next one is line 5. `End Function`
    // belongs to the procedure but nothing follows it.
    assert_eq!(set[0].line, Some(5));
    assert_eq!(set[1].line, None);
    assert_eq!(set[2].line, None); // a declaration outside any procedure
    assert_eq!(set[3].line, None);
    assert_eq!(set[4].line, None);
    assert_eq!(inst.breakable_lines("Module1"), vec![5, 6, 10, 11, 12]);
}

#[test]
fn stepping_into_over_and_out() {
    let mut inst = one(MAIN);
    let log: Log = Rc::default();
    inst.set_breakpoints(&[("Module1", 10)]);
    inst.attach_debugger(scripted(
        &log,
        vec![
            ResumeAction::StepInto, // line 10 -> 11
            ResumeAction::StepInto, // 11 -> Add:5
            ResumeAction::StepOver, // Add:5 -> Add:6
            ResumeAction::StepOut,  // Add:6 -> Main:12 (after the call)
            ResumeAction::Continue,
        ],
    ));
    inst.run("Main", vec![]).unwrap();
    assert_eq!(
        *log.borrow(),
        vec![
            "Breakpoint Module1.Main:10",
            "Step Module1.Main:11",
            "Step Module1.Add:5",
            "Step Module1.Add:6",
            "Step Module1.Main:12",
        ]
    );
}

#[test]
fn step_over_does_not_enter_calls() {
    let mut inst = one(MAIN);
    let log: Log = Rc::default();
    inst.set_breakpoints(&[("Module1", 11)]);
    inst.attach_debugger(scripted(
        &log,
        vec![ResumeAction::StepOver, ResumeAction::Continue],
    ));
    inst.run("Main", vec![]).unwrap();
    assert_eq!(
        *log.borrow(),
        vec!["Breakpoint Module1.Main:11", "Step Module1.Main:12"]
    );
}

#[test]
fn break_on_next_statement_steps_into_the_first_line_of_a_run() {
    let mut inst = one(MAIN);
    let log: Log = Rc::default();
    inst.attach_debugger(scripted(&log, vec![ResumeAction::Continue]));
    inst.break_on_next_statement();
    inst.run("Main", vec![]).unwrap();
    assert_eq!(*log.borrow(), vec!["Step Module1.Main:10"]);
    // The request was for one run only.
    inst.run("Main", vec![]).unwrap();
    assert_eq!(log.borrow().len(), 1);
}

#[test]
fn a_step_left_over_from_one_run_does_not_leak_into_the_next() {
    let mut inst = one(MAIN);
    let log: Log = Rc::default();
    inst.set_breakpoints(&[("Module1", 12)]);
    inst.attach_debugger(scripted(&log, vec![ResumeAction::StepOver]));
    inst.run("Main", vec![]).unwrap(); // pauses at 12, then steps over the last line
    assert_eq!(*log.borrow(), vec!["Breakpoint Module1.Main:12"]);
    inst.set_breakpoints(&[]);
    inst.run("Main", vec![]).unwrap();
    assert_eq!(
        log.borrow().len(),
        1,
        "stepping state must not survive the run"
    );
}

#[test]
fn several_statements_on_a_line_pause_once() {
    let mut inst =
        one("Public Sub P()\n  Dim a As Long\n  a = 1: a = a + 1: a = a + 1\n  a = 4\nEnd Sub\n");
    let log: Log = Rc::default();
    inst.set_breakpoints(&[("Module1", 3)]);
    inst.attach_debugger(scripted(
        &log,
        vec![ResumeAction::StepInto, ResumeAction::Continue],
    ));
    inst.run("P", vec![]).unwrap();
    assert_eq!(
        *log.borrow(),
        vec!["Breakpoint Module1.P:3", "Step Module1.P:4"]
    );
}

#[test]
fn recursion_is_stepped_per_frame() {
    let mut inst = one(
        "Public Function F(ByVal n As Long) As Long\n  If n = 0 Then\n    F = 0\n  Else\n    F = F(n - 1)\n  End If\nEnd Function\n",
    );
    let log: Log = Rc::default();
    inst.set_breakpoints(&[("Module1", 5)]);
    inst.attach_debugger({
        let mut inner = scripted(&log, vec![]);
        move |ctx: &mut DebugContext<'_>| {
            let depth = ctx.frames().len();
            let n = ctx
                .locals(0)
                .iter()
                .find(|v| v.name == "n")
                .unwrap()
                .value
                .clone();
            inner(ctx);
            assert_eq!(n, Variant::Long(2 - (depth as i32 - 1)));
            ResumeAction::Continue
        }
    });
    inst.run("F", vec![2.into()]).unwrap();
    assert_eq!(log.borrow().len(), 2, "n = 2 and n = 1 reach line 5");
}

#[test]
fn stop_and_a_failed_assert_halt_without_a_debugger() {
    let mut inst = one(
        "Public Sub A()\n  Debug.Print \"a\"\n  Stop\n  Debug.Print \"after\"\nEnd Sub\nPublic Sub B()\n  Debug.Assert 1 = 2\n  Debug.Print \"after\"\nEnd Sub\n",
    );
    let out = inst.run("A", vec![]).unwrap();
    assert_eq!(out.status, RunStatus::Stopped);
    assert_eq!(out.output, "a\n");
    let out = inst.run("B", vec![]).unwrap();
    assert_eq!(out.status, RunStatus::Stopped);
    assert_eq!(out.output, "");
    // A true assertion has no effect.
    let mut ok = one("Public Sub C()\n  Debug.Assert 1 = 1\n  Debug.Print \"ran\"\nEnd Sub\n");
    assert_eq!(ok.run("C", vec![]).unwrap().output, "ran\n");
}

#[test]
fn stop_and_a_failed_assert_pause_with_a_debugger_and_the_program_goes_on() {
    let mut inst = one(
        "Public Sub A()\n  Debug.Print \"a\"\n  Stop\n  Debug.Print \"after stop\"\n  Debug.Assert 1 = 2\n  Debug.Print \"after assert\"\nEnd Sub\n",
    );
    let log: Log = Rc::default();
    inst.attach_debugger(scripted(&log, vec![]));
    let out = inst.run("A", vec![]).unwrap();
    assert_eq!(out.status, RunStatus::Completed);
    assert_eq!(out.output, "a\nafter stop\nafter assert\n");
    assert_eq!(
        *log.borrow(),
        vec!["Stop Module1.A:3", "AssertFailed Module1.A:5"]
    );
}

#[test]
fn abort_ends_the_run_and_on_error_cannot_catch_it() {
    let mut inst = one(
        "Public Sub A()\n  On Error Resume Next\n  Debug.Print \"1\"\n  Debug.Print \"2\"\nEnd Sub\n",
    );
    inst.set_breakpoints(&[("Module1", 4)]);
    inst.attach_debugger(|_: &mut DebugContext<'_>| ResumeAction::Abort);
    let err = inst.run("A", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert_eq!(inst.output(), "1\n");
    // The instance is usable (the cancel token was not involved).
    assert!(inst.is_usable());
    inst.detach_debugger();
    inst.set_breakpoints(&[]);
    assert_eq!(inst.run("A", vec![]).unwrap().output, "1\n2\n");
}

#[test]
fn attaching_a_debugger_changes_neither_results_nor_steps() {
    let src = "\
Public Function Work(ByVal n As Long) As Long
  Dim i As Long, s As Long
  For i = 1 To n
    If i Mod 3 = 0 Then s = s + i Else s = s - 1
  Next i
  Work = s
End Function
";
    let mut plain = one(src);
    let a = plain.run("Work", vec![200.into()]).unwrap();
    let used_plain = steps(&plain);

    let mut dbg = one(src);
    let hits = Rc::new(RefCell::new(0u32));
    let h = hits.clone();
    dbg.set_breakpoints(&[("Module1", 4)]);
    dbg.attach_debugger(move |_: &mut DebugContext<'_>| {
        *h.borrow_mut() += 1;
        ResumeAction::StepInto // pause on every line
    });
    let b = dbg.run("Work", vec![200.into()]).unwrap();
    assert!(*hits.borrow() > 400);
    assert_eq!(a.value, b.value);
    assert_eq!(used_plain, steps(&dbg), "pausing must not charge steps");
}

#[test]
fn time_spent_paused_does_not_burn_the_call_timeout() {
    let mut inst = Engine::new()
        .call_timeout(Some(Duration::from_millis(300)))
        .load(&project(&[("Module1", ModuleKind::Standard, MAIN)]))
        .unwrap();
    inst.trust();
    inst.set_breakpoints(&[("Module1", 11)]);
    inst.attach_debugger(|_: &mut DebugContext<'_>| {
        std::thread::sleep(Duration::from_millis(700));
        ResumeAction::Continue
    });
    inst.run("Main", vec![])
        .expect("the wait at the breakpoint is not charged");
    // The timeout still works for running time.
    let mut spin = Engine::new()
        .unlimited()
        .call_timeout(Some(Duration::from_millis(100)))
        .load(&project(&[(
            "Module1",
            ModuleKind::Standard,
            "Public Sub S()\n  Do\n    DoEvents\n  Loop\nEnd Sub\n",
        )]))
        .unwrap();
    spin.trust();
    let err = spin.run("S", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
}

#[test]
fn me_and_fields_of_a_class_frame_are_visible() {
    let mut inst = load(&[
        (
            "Module1",
            ModuleKind::Standard,
            "Public Sub Main()\n  Dim c As Counter\n  Set c = New Counter\n  c.Bump 5\nEnd Sub\n",
        ),
        (
            "Counter",
            ModuleKind::Class,
            "Private total As Long\nPublic Name As String\nPublic Sub Bump(ByVal n As Long)\n  total = total + n\nEnd Sub\n",
        ),
    ]);
    inst.set_breakpoints(&[("Counter", 4)]);
    let seen: Rc<RefCell<Vec<String>>> = Rc::default();
    let log = seen.clone();
    inst.attach_debugger(move |ctx: &mut DebugContext<'_>| {
        let (me, fields) = ctx.me(0).expect("a method has Me");
        log.borrow_mut().push(ctx.describe(&me).unwrap());
        for f in fields {
            log.borrow_mut().push(format!(
                "{} {} {:?} {:?}",
                f.name, f.type_name, f.value, f.kind
            ));
        }
        assert!(ctx.me(1).is_none());
        // Evaluate against Me and a parameter.
        let r = ctx.eval(0, "? Me.Name & n").unwrap();
        log.borrow_mut().push(r.text);
        ResumeAction::Continue
    });
    inst.run("Main", vec![]).unwrap();
    let seen = seen.borrow();
    assert_eq!(seen[0], "Counter");
    assert_eq!(seen[1], "total Long Long(0) Field");
    assert_eq!(seen[2], "Name String String(VbaString(\"\")) Field");
    assert_eq!(seen[3], "5");
}

// ---- eval in a paused frame -----------------------------------------------------------

#[test]
fn eval_in_a_paused_frame_reads_and_assigns_locals() {
    let mut inst = one(MAIN);
    inst.set_breakpoints(&[("Module1", 6)]);
    let seen: Rc<RefCell<Vec<String>>> = Rc::default();
    let log = seen.clone();
    inst.attach_debugger(move |ctx: &mut DebugContext<'_>| {
        let r = ctx.eval(0, "? t * 10 + a").unwrap();
        log.borrow_mut().push(format!("{:?} {}", r.value, r.text));
        // Assignment to a local, and to a module variable of the frame's module.
        ctx.eval(0, "t = 100: Counter = 7").unwrap();
        let t = ctx.locals(0).into_iter().find(|v| v.name == "t").unwrap();
        log.borrow_mut().push(format!("{:?}", t.value));
        // The caller's frame.
        let r = ctx.eval(1, "? secret & \"!\"").unwrap();
        log.borrow_mut().push(r.text);
        // Errors do not end the run and leave Err alone.
        let e = ctx.eval(0, "? 1 / 0").unwrap_err();
        log.borrow_mut().push(e.to_string());
        let e = ctx.eval(0, "? nosuch + 1").unwrap_err();
        log.borrow_mut().push(format!("{e}"));
        // A write-back that does not fit the declared type is reported.
        let e = ctx.eval(0, "a = \"x\"").unwrap_err();
        log.borrow_mut().push(format!("writeback: {e}"));
        ResumeAction::Continue
    });
    let out = inst.run("Main", vec![]).unwrap();
    assert_eq!(out.status, RunStatus::Completed);
    let seen = seen.borrow();
    assert_eq!(seen[0], "Some(Long(31))  31 ");
    assert_eq!(seen[1], "Long(100)");
    assert_eq!(seen[2], "\"s!\"".trim_matches('"').to_owned() + "");
    assert!(seen[3].contains("Division by zero"), "{}", seen[3]);
    assert!(
        seen[4].to_lowercase().contains("not defined"),
        "{}",
        seen[4]
    );
    assert!(seen[5].starts_with("writeback:"), "{}", seen[5]);
    // `t = 100` took effect: Add returned 100 and Main stored it.
    assert_eq!(inst.get_global("Counter").unwrap(), Variant::Long(100));
    assert!(inst.last_error().is_none());
}

#[test]
fn eval_while_paused_obeys_the_step_budget() {
    let mut inst = Engine::new()
        .max_steps(5_000)
        .load(&project(&[("Module1", ModuleKind::Standard, MAIN)]))
        .unwrap();
    inst.trust();
    inst.set_breakpoints(&[("Module1", 6)]);
    inst.attach_debugger(|ctx: &mut DebugContext<'_>| {
        let _ = ctx.eval(0, "Do: Loop");
        ResumeAction::Continue
    });
    let err = inst.run("Main", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
}

// ---- eval from the facade -----------------------------------------------------------------

#[test]
fn eval_expressions_statements_and_scopes() {
    let mut inst = load(&[
        (
            "Module1",
            ModuleKind::Standard,
            "Private hidden As Long\nPublic shown As Long\nPrivate Const K = 4\nPrivate Type Pt\n  x As Long\nEnd Type\nPrivate Function Secret(ByVal n As Long) As Long\n  Secret = n + hidden\nEnd Function\nPublic Sub Show(ByVal s As String)\n  Debug.Print \"show \" & s\nEnd Sub\n",
        ),
        (
            "Module2",
            ModuleKind::Standard,
            "Private Const Mine = 1\nPublic Sub Other()\nEnd Sub\n",
        ),
    ]);
    let m1 = EvalScope::Module("Module1");
    inst.eval("hidden = 5", m1).unwrap();
    assert_eq!(
        inst.eval("? Secret(K)", m1).unwrap().value,
        Some(Variant::Long(9))
    );
    assert_eq!(inst.eval("? Secret(K)", m1).unwrap().text, " 9 ");
    let r = inst.eval("Show \"a\": Call Show(\"b\")", m1).unwrap();
    assert_eq!(r.output, "show a\nshow b\n");
    assert_eq!(r.display(), "show a\nshow b");
    assert_eq!(inst.eval("? \"x\" & 1 + 2", m1).unwrap().display(), "x3");
    // Public names are visible from anywhere, private ones are not.
    assert_eq!(
        inst.eval("? shown", EvalScope::Global).unwrap().value,
        Some(Variant::Long(0))
    );
    assert!(inst.eval("? hidden", EvalScope::Global).is_err());
    assert!(inst.eval("? hidden", EvalScope::Module("Module2")).is_err());
    assert!(inst.eval("? Mine", m1).is_err());
    assert!(inst.eval("? Mine", EvalScope::Module("Module2")).is_ok());
    // Private types of the module can be used in the text.
    inst.eval("Dim p As Pt: p.x = 3: Debug.Print p.x", m1)
        .unwrap();
    // Unknown module.
    assert!(inst.eval("? 1", EvalScope::Module("Nope")).is_err());
    // Module state persists: the statements ran on the live instance.
    assert_eq!(inst.eval("? hidden", m1).unwrap().text, " 5 ");
}

#[test]
fn eval_reports_errors_as_text_and_leaves_the_instance_usable() {
    let mut inst = one(MAIN);
    let m = EvalScope::Module("Module1");
    let e = inst.eval("? 1 / 0", m).unwrap_err();
    assert_eq!(
        e.immediate_message(),
        "Run-time error '11': Division by zero"
    );
    let e = inst.eval("? 1 +", m).unwrap_err();
    assert!(matches!(&e, Error::Run(RunError::Load(_))), "{e:?}");
    assert!(!e.immediate_message().is_empty());
    let e = inst.eval("x = ", m).unwrap_err();
    assert!(matches!(&e, Error::Run(RunError::Load(_))), "{e:?}");
    // Err was not touched, the instance still works.
    assert!(inst.last_error().is_none());
    assert_eq!(inst.eval("? Add(2, 3)", m).unwrap().text, " 5 ");
    assert!(inst.is_usable());
}

#[test]
fn eval_needs_trust_and_the_same_capabilities_as_the_program() {
    let p = project(&[("Module1", ModuleKind::Standard, MAIN)]);
    let mut inst = Engine::new().load(&p).unwrap();
    let e = inst.eval("? 1", EvalScope::Global).unwrap_err();
    assert!(matches!(e, Error::Run(RunError::NotTrusted)), "{e:?}");
    inst.trust();
    let e = inst
        .eval(
            "Dim o As Object: Set o = CreateObject(\"Scripting.Dictionary\")",
            EvalScope::Global,
        )
        .unwrap_err();
    assert!(
        matches!(e, Error::Run(RunError::PermissionDenied(_))),
        "{e:?}"
    );
    let mut allowed = Engine::new().grant(Capability::ComCreate).load(&p).unwrap();
    allowed.trust();
    allowed
        .eval(
            "Dim o As Object: Set o = CreateObject(\"Scripting.Dictionary\")",
            EvalScope::Global,
        )
        .unwrap();
}

#[test]
fn eval_takes_one_line_and_cannot_declare_anything() {
    let mut inst = one(MAIN);
    let m = EvalScope::Module("Module1");
    for bad in [
        "Counter = 1\nEnd Sub\nPublic Sub Evil()\nEnd Sub",
        "Counter = 1\r\nCounter = 2",
        "",
        "?",
        "   ",
    ] {
        assert!(inst.eval(bad, m).is_err(), "{bad:?}");
    }
    for sneaky in [
        "Counter = 1: End Sub: Public Sub Evil(): Counter = 2",
        "End Sub: Public Declare Function GetTickCount Lib \"kernel32\" () As Long",
        "Counter = 1: Public Zed As Long",
    ] {
        let before = inst.modules();
        let r = inst.eval(sneaky, m);
        assert!(
            r.is_err(),
            "{sneaky:?} must not compile into extra declarations"
        );
        assert_eq!(inst.modules(), before);
    }
    assert_eq!(
        inst.get_global("Counter").unwrap(),
        Variant::Long(0),
        "a refused text runs nothing"
    );
}

#[test]
fn eval_does_not_stop_at_breakpoints() {
    let mut inst = one(MAIN);
    inst.set_breakpoints(&[("Module1", 5)]);
    let hits = Rc::new(RefCell::new(0));
    let h = hits.clone();
    inst.attach_debugger(move |_: &mut DebugContext<'_>| {
        *h.borrow_mut() += 1;
        ResumeAction::Continue
    });
    assert_eq!(
        inst.eval("? Add(1, 1)", EvalScope::Module("Module1"))
            .unwrap()
            .text,
        " 2 "
    );
    assert_eq!(*hits.borrow(), 0);
    inst.run("Main", vec![]).unwrap();
    assert_eq!(*hits.borrow(), 1);
}

// ---- hot edit ---------------------------------------------------------------------------

const STATE: &str = "\
Public Total As Long
Public Label As String
Public Items As Collection
Public Function Bump() As Long
  Total = Total + 1
  Bump = Total
End Function
";

#[test]
fn a_body_edit_keeps_every_piece_of_state() {
    let mut inst = load(&[
        ("Module1", ModuleKind::Standard, STATE),
        (
            "Thing",
            ModuleKind::Class,
            "Public V As Long\nPublic Function Get2() As Long\n  Get2 = V * 2\nEnd Function\n",
        ),
    ]);
    inst.run("Bump", vec![]).unwrap();
    inst.run("Bump", vec![]).unwrap();
    let obj = inst.create_object("Thing", None).unwrap();
    inst.set_member(obj, "V", Variant::Long(21)).unwrap();
    let report = inst
        .apply_edits(&[ModuleEdit::Replace {
            name: "module1".into(),
            source: STATE.replace("Total + 1", "Total + 10"),
        }])
        .unwrap();
    assert_eq!(report.replaced, vec!["Module1"]);
    assert!(!report.state.as_ref().unwrap().layout_changed);
    assert_eq!(inst.run("Bump", vec![]).unwrap().value, Variant::Long(12));
    // The class instance survived.
    assert!(inst.session_mut().vm().is_live_instance(&obj));
    let r = inst
        .call(vba::CallTarget::Object(obj), "Get2", vec![])
        .unwrap();
    assert_eq!(r.value, Variant::Long(42));
    // The same text again is a no-op.
    let again = inst
        .apply_edits(&[ModuleEdit::Replace {
            name: "Module1".into(),
            source: STATE.replace("Total + 1", "Total + 10"),
        }])
        .unwrap();
    assert!(again.unchanged && again.state.is_none());
}

#[test]
fn a_declaration_change_keeps_unchanged_plain_variables_and_drops_objects() {
    let mut inst = load(&[
        ("Module1", ModuleKind::Standard, STATE),
        ("Thing", ModuleKind::Class, "Public V As Long\n"),
    ]);
    inst.run("Bump", vec![]).unwrap();
    inst.set_global("Label", Variant::from("keep me")).unwrap();
    let obj = inst.create_object("Thing", None).unwrap();
    // Label changes type, Total keeps its declaration; a new variable appears.
    let edited = "\
Public Total As Long
Public Label As Long
Public Extra As Long
Public Items As Collection
Public Function Bump() As Long
  Total = Total + 1
  Bump = Total
End Function
";
    let report = inst
        .apply_edits(&[ModuleEdit::Replace {
            name: "Module1".into(),
            source: edited.into(),
        }])
        .unwrap();
    let state = report.state.unwrap();
    assert!(state.layout_changed);
    assert!(
        state.kept.contains(&"Module1.Total".to_owned()),
        "{state:?}"
    );
    assert!(
        state.reset.contains(&"Module1.Label".to_owned()),
        "{state:?}"
    );
    assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(1));
    assert_eq!(inst.get_global("Label").unwrap(), Variant::Long(0));
    assert_eq!(inst.get_global("Extra").unwrap(), Variant::Long(0));
    assert_eq!(inst.run("Bump", vec![]).unwrap().value, Variant::Long(2));
    assert!(
        !inst.session_mut().vm().is_live_instance(&obj),
        "instances are dropped when the layout changes"
    );
    assert_eq!(state.dropped_objects, 1);
}

#[test]
fn a_failed_edit_changes_nothing() {
    let mut inst = one(STATE);
    inst.run("Bump", vec![]).unwrap();
    let err = inst
        .apply_edits(&[
            ModuleEdit::Add {
                name: "Fine".into(),
                kind: ModuleKind::Standard,
                source: "Public Sub X()\nEnd Sub\n".into(),
            },
            ModuleEdit::Replace {
                name: "Module1".into(),
                source: "Public Function Bump() As Long\n  Bump = 1 +\nEnd Function\n".into(),
            },
        ])
        .unwrap_err();
    let Error::Run(RunError::Load(l)) = &err else {
        panic!("{err:?}")
    };
    assert!(!l.diagnostics.is_empty());
    assert!(
        inst.callable_procedures()
            .iter()
            .all(|p| p.module != "Fine")
    );
    assert_eq!(inst.run("Bump", vec![]).unwrap().value, Variant::Long(2));
    // Bad requests are refused too.
    for bad in [
        ModuleEdit::Replace {
            name: "Nope".into(),
            source: String::new(),
        },
        ModuleEdit::Remove {
            name: "Nope".into(),
        },
        ModuleEdit::Add {
            name: "module1".into(),
            kind: ModuleKind::Standard,
            source: String::new(),
        },
        ModuleEdit::Add {
            name: "VbaRsImmediate".into(),
            kind: ModuleKind::Standard,
            source: String::new(),
        },
    ] {
        let e = inst.apply_edits(&[bad]).unwrap_err();
        assert!(matches!(e, Error::Run(RunError::BadEdit(_))), "{e:?}");
    }
}

#[test]
fn modules_can_be_added_and_removed() {
    let mut inst = one(STATE);
    inst.apply_edits(&[ModuleEdit::Add {
        name: "Helpers".into(),
        kind: ModuleKind::Standard,
        source: "Public Function Triple(ByVal n As Long) As Long\n  Triple = n * 3\nEnd Function\n"
            .into(),
    }])
    .unwrap();
    assert_eq!(
        inst.run("Helpers.Triple", vec![4.into()]).unwrap().value,
        Variant::Long(12)
    );
    // Removing a module another one needs is refused with diagnostics.
    inst.apply_edits(&[ModuleEdit::Replace {
        name: "Module1".into(),
        source: format!(
            "{STATE}Public Function Use3() As Long\n  Use3 = Triple(2)\nEnd Function\n"
        ),
    }])
    .unwrap();
    let e = inst
        .apply_edits(&[ModuleEdit::Remove {
            name: "Helpers".into(),
        }])
        .unwrap_err();
    assert!(matches!(e, Error::Run(RunError::Load(_))), "{e:?}");
    assert_eq!(inst.run("Use3", vec![]).unwrap().value, Variant::Long(6));
    // Remove both users at once.
    inst.apply_edits(&[
        ModuleEdit::Replace {
            name: "Module1".into(),
            source: STATE.into(),
        },
        ModuleEdit::Remove {
            name: "Helpers".into(),
        },
    ])
    .unwrap();
    assert!(inst.run("Helpers.Triple", vec![1.into()]).is_err());
}

#[test]
fn breakpoints_follow_an_edit() {
    let mut inst = one(MAIN);
    inst.set_breakpoints(&[("Module1", 11)]);
    let log: Log = Rc::default();
    inst.attach_debugger(scripted(&log, vec![]));
    inst.apply_edits(&[ModuleEdit::Replace {
        name: "Module1".into(),
        source: format!("' header\n{MAIN}"),
    }])
    .unwrap();
    // The request is by line: line 11 is now `Set` of the shifted source.
    inst.run("Main", vec![]).unwrap();
    assert_eq!(log.borrow().len(), 1);
    assert!(log.borrow()[0].starts_with("Breakpoint Module1.Main:11"));
}

#[test]
fn edits_keep_the_trust_decision_and_the_debug_sink() {
    let mut inst = one("Public Sub P()\n  Debug.Print \"one\"\nEnd Sub\n");
    inst.run("P", vec![]).unwrap();
    inst.apply_edits(&[ModuleEdit::Replace {
        name: "Module1".into(),
        source: "Public Sub P()\n  Debug.Print \"two\"\nEnd Sub\n".into(),
    }])
    .unwrap();
    assert!(inst.is_trusted());
    inst.run("P", vec![]).unwrap();
    assert_eq!(inst.output(), "one\ntwo\n");
}

// ---- Run Macro -----------------------------------------------------------------------------

#[test]
fn callable_procedures_and_run_by_name() {
    let mut inst = load(&[
        (
            "Module1",
            ModuleKind::Standard,
            "Public Sub Hello()\n  Debug.Print \"hello\"\nEnd Sub\nPublic Function Add(ByVal a As Long, Optional ByVal b As Long = 1) As Long\n  Add = a + b\nEnd Function\nPrivate Sub Hidden()\nEnd Sub\nPublic Property Get Prop() As Long\nEnd Property\nPublic Sub Dup()\nEnd Sub\n",
        ),
        (
            "Module2",
            ModuleKind::Standard,
            "Public Sub Dup()\nEnd Sub\n",
        ),
        ("Plain", ModuleKind::Class, "Public Sub Method()\nEnd Sub\n"),
    ]);
    let list = inst.callable_procedures();
    let names: Vec<&str> = list.iter().map(|p| p.qualified_name.as_str()).collect();
    assert_eq!(
        names,
        vec!["Module1.Hello", "Module1.Add", "Module1.Dup", "Module2.Dup"]
    );
    let add = &list[1];
    assert!(add.requires_arguments);
    assert_eq!(add.params.len(), 2);
    assert_eq!(add.params[1].type_name, "Long");
    assert!(add.params[1].optional);
    assert_eq!(add.return_type.as_deref(), Some("Long"));
    assert!(!list[0].requires_arguments);
    assert!(list[0].unique_bare_name);
    assert!(!list[2].unique_bare_name, "Dup is declared twice");
    // Run by the listed name, with arguments, any case.
    assert_eq!(
        inst.run("module1.ADD", vec![2.into(), 5.into()])
            .unwrap()
            .value,
        Variant::Long(7)
    );
    assert_eq!(inst.run("Hello", vec![]).unwrap().output, "hello\n");
    assert!(matches!(
        inst.run("Dup", vec![]),
        Err(Error::Run(RunError::AmbiguousProcedure(_)))
    ));
    assert!(matches!(
        inst.run("Module1.Hidden", vec![]),
        Err(Error::Run(RunError::NotCallable(..)))
    ));
    assert!(matches!(
        inst.run("Plain.Method", vec![]),
        Err(Error::Run(RunError::NotCallable(..)))
    ));
    assert!(matches!(
        inst.run("Nope.Missing", vec![]),
        Err(Error::Run(RunError::NoSuchProcedure(_)))
    ));
}
