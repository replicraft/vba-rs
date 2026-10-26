//! Error detail: numbers, sources, lines and call stacks.

mod common;

use common::*;
use vba::runtime::Error as RunError;
use vba::{Arg, CallTarget, Error, ModuleKind, Variant};

const MODULE1: &str = "\
Public Sub Outer()
  Mid1
End Sub

Public Sub Mid1()
  Inner 5
End Sub

Public Sub Inner(ByVal n As Long)
  Dim x As Long
  x = n \\ 0
End Sub

Public Sub Raiser()
  Err.Raise 5000, \"MyApp.Raiser\", \"custom failure\", \"help.hlp\", 77
End Sub

Public Sub Numbered()
10  Dim x As Long
20  x = 1 \\ 0
End Sub

Public Sub Tolerant()
  On Error Resume Next
  Dim x As Long
  x = 1 \\ 0
  Debug.Print \"after \" & Err.Number
End Sub

Public Sub Trapped()
  On Error Resume Next
  Inner 1
  Debug.Print \"trapped \" & Err.Number
End Sub

Public Sub Clean()
  Debug.Print \"clean\"
End Sub
";

fn report(e: &Error) -> &vba::ErrorReport {
    match e {
        Error::Run(r) => r.report().expect("an error with a report"),
        other => panic!("not a VBA error: {other:?}"),
    }
}

fn inst() -> vba::Instance {
    load(&[("Module1", ModuleKind::Standard, MODULE1)])
}

#[test]
fn nested_frames_are_reported_innermost_first() {
    let mut inst = inst();
    let err = inst.call(CallTarget::Project, "Outer", vec![]).unwrap_err();
    let r = report(&err);
    assert_eq!(r.number, 11);
    assert_eq!(r.description, "Division by zero");
    let names: Vec<String> = r
        .frames
        .iter()
        .map(|f| format!("{}.{}", f.module, f.procedure))
        .collect();
    assert_eq!(names, ["Module1.Inner", "Module1.Mid1", "Module1.Outer"]);
    assert_eq!(r.stack, names);
    // Source lines: the failing statement, the call in Mid1, the call in Outer.
    let lines: Vec<u32> = r.frames.iter().map(|f| f.source_line).collect();
    assert_eq!(lines, [11, 6, 2]);
    let info = r.info();
    assert_eq!(info.module, "Module1");
    assert_eq!(info.procedure, "Inner");
    assert_eq!(info.line, 11);
    assert_eq!(info.erl, 0);
    assert_eq!(info.number, 11);
    assert_eq!(
        err.to_string(),
        "Run-time error '11': Division by zero (in Module1.Inner, source line 11)"
    );
    let detail = match &err {
        Error::Run(r) => r.detail(),
        _ => unreachable!(),
    };
    assert!(
        detail.contains("\n  at Module1.Inner (source line 11)"),
        "{detail}"
    );
    assert!(
        detail.contains("\n  at Module1.Outer (source line 2)"),
        "{detail}"
    );
    assert_eq!(
        match &err {
            Error::Run(r) => r.stack().len(),
            _ => 0,
        },
        3
    );
}

#[test]
fn erl_line_numbers_take_precedence() {
    let mut inst = inst();
    let err = inst
        .call(CallTarget::Project, "Numbered", vec![])
        .unwrap_err();
    let r = report(&err);
    assert_eq!(r.line, 20);
    assert_eq!(r.info().line, 20);
    assert_eq!(r.info().erl, 20);
    assert_eq!(r.source_line, 20);
    assert!(
        err.to_string().ends_with("(in Module1.Numbered, line 20)"),
        "{err}"
    );
}

#[test]
fn err_raise_carries_number_source_help_and_description() {
    let mut inst = inst();
    let err = inst
        .call(CallTarget::Project, "Raiser", vec![])
        .unwrap_err();
    let info = report(&err).info();
    assert_eq!(info.number, 5000);
    assert_eq!(info.source, "MyApp.Raiser");
    assert_eq!(info.description, "custom failure");
    assert_eq!(info.help_file, "help.hlp");
    assert_eq!(info.help_context, 77);
    assert_eq!(info.procedure, "Raiser");
    assert!(matches!(err, Error::Run(RunError::Vba(_))));
    // Without a source the project name is the source.
    let err = inst
        .call(CallTarget::Project, "Inner", vec![Arg::val(1)])
        .unwrap_err();
    assert_eq!(report(&err).info().source, "VBAProject");
    assert!(format!("{:#}", report(&err)).contains("source: VBAProject"));
}

#[test]
fn errors_in_event_handlers_name_the_handler() {
    let mut inst = load(&[(
        "Form_Orders",
        ModuleKind::Document,
        "Private Sub Form_BeforeUpdate(Cancel As Integer)\n  Helper\nEnd Sub\nPrivate Sub Helper()\n  Err.Raise 6001, , \"bad\"\nEnd Sub\n",
    )]);
    let form = inst.create_object("Form_Orders", None).unwrap();
    let err = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap_err();
    let r = report(&err);
    assert_eq!(r.number, 6001);
    let names: Vec<String> = r.frames.iter().map(|f| f.procedure.clone()).collect();
    assert_eq!(names, ["Helper", "Form_BeforeUpdate"]);
    assert!(r.frames.iter().all(|f| f.module == "Form_Orders"));
    // The form stays usable after the failed event.
    assert!(inst.has_procedure(form, "Helper"));
}

#[test]
fn handled_errors_are_visible_through_err_with_their_origin() {
    let mut inst = inst();
    let r = inst.call(CallTarget::Project, "Tolerant", vec![]).unwrap();
    assert_eq!(r.output, "after 11\n");
    let e = inst.last_error().expect("Err still holds the error");
    assert_eq!(e.number, 11);
    assert_eq!(e.frames.len(), 1);
    assert_eq!(e.frames[0].procedure, "Tolerant");
    assert_eq!(e.frames[0].source_line, 26);

    // A caller that resumes still records where the error was raised.
    inst.call(CallTarget::Project, "Trapped", vec![]).unwrap();
    let e = inst.last_error().unwrap();
    let names: Vec<String> = e.frames.iter().map(|f| f.procedure.clone()).collect();
    assert_eq!(names, ["Inner", "Trapped"]);
    assert_eq!(e.info().procedure, "Inner");
}

#[test]
fn handled_flows_do_not_leak_into_later_failures() {
    let mut inst = inst();
    inst.call(CallTarget::Project, "Tolerant", vec![]).unwrap();
    assert!(inst.last_error().is_some());
    // An unrelated failure reports its own site only.
    let err = inst
        .call(CallTarget::Project, "Raiser", vec![])
        .unwrap_err();
    let r = report(&err);
    assert_eq!(r.number, 5000);
    assert_eq!(r.frames.len(), 1);
    assert_eq!(r.frames[0].procedure, "Raiser");
    // And the next unhandled error in the same shape is independent of it.
    let err = inst.call(CallTarget::Project, "Outer", vec![]).unwrap_err();
    assert_eq!(report(&err).frames.len(), 3);
    // A clean call succeeds and the failed call's state does not turn it
    // into an error.
    let ok = inst.call(CallTarget::Project, "Clean", vec![]).unwrap();
    assert_eq!(ok.output, "clean\n");
}

#[test]
fn host_errors_keep_the_vba_frames() {
    let st = FormState::new(&[("Caption", Variant::from("x"))]);
    let mut inst = load(&[(
        "Form_Orders",
        ModuleKind::Document,
        "Public Sub Go()\n  Me.Controls(\"missing\").Value = 1\nEnd Sub\n",
    )]);
    let form = inst
        .create_object("Form_Orders", Some(FormHost::new(&st)))
        .unwrap();
    let err = inst.call_on(form, "Go", vec![]).unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Host(_))), "{err:?}");
    let r = report(&err);
    assert_eq!(r.frames.len(), 1);
    assert_eq!(r.frames[0].procedure, "Go");
    assert_eq!(r.source_line, 2);
}

#[test]
fn errors_before_any_frame_have_an_empty_stack() {
    let mut inst = inst();
    let err = inst
        .call(CallTarget::Project, "Inner", vec![Arg::val("abc")])
        .unwrap_err();
    let r = report(&err);
    assert_eq!(r.number, 13);
    assert!(r.frames.is_empty());
    assert_eq!(r.info().line, 0);
}

#[test]
fn the_error_type_crosses_threads_and_forwards_the_details() {
    fn assert_send_sync<T: Send + Sync + 'static + std::error::Error>() {}
    assert_send_sync::<Error>();

    let engine = vba::Engine::new();
    let project = vba::Project::from_source(
        "Module1",
        "Public Sub Boom()\n  Err.Raise 5000, \"App.Rules\", \"nope\"\nEnd Sub\n",
    );
    let mut inst = engine.load(&project).unwrap();
    inst.trust();
    let err = inst.run("Boom", vec![]).unwrap_err();
    // The embedding application maps without matching on the runtime enum.
    let info = err.error_info().expect("a VBA error carries its details");
    assert_eq!(
        (info.number, info.source.as_str(), info.description.as_str()),
        (5000, "App.Rules", "nope")
    );
    assert_eq!(err.report().unwrap().frames[0].procedure, "Boom");

    let load = engine
        .load(&vba::Project::from_source("M", "Public Sub A(\nEnd Sub\n"))
        .err()
        .unwrap();
    assert!(load.error_info().is_none());
    assert!(load.report().is_none());
}
