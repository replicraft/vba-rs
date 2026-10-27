//! Dynamic calls, UI hooks, out-parameters, class modules, `WithEvents`
//! and reentrancy.

use crate::forms::*;
use crate::host::*;
use crate::{orders_db, run};
use vba::{Arg, CallTarget, ModuleKind, Variant};

pub const MATH: &str = "\
Option Explicit
Public Function Add(ByVal a As Long, Optional ByVal b As Long = 10) As Long
  Add = a + b
End Function
Public Sub Boom(ByVal n As Long)
  Err.Raise 5000 + n, \"Math\", \"boom \" & n
End Sub
Public Function Depth(ByVal n As Long) As Long
  If n = 0 Then
    Depth = 0
  Else
    Depth = 1 + Application.Run(\"Depth\", n - 1)
  End If
End Function
";

#[test]
fn application_run_calls_vba_by_name_with_arguments_and_returns_the_value() {
    let db = orders_db();
    let code = "\
Public Sub Main()
  Debug.Print Application.Run(\"Math.Add\", 1, 2)
  Debug.Print Application.Run(\"Add\", 5)
  Application.Run \"Boom\", 1
End Sub
";
    let mut inst = load(
        &db,
        &[
            ("Math", ModuleKind::Standard, MATH),
            ("Main1", ModuleKind::Standard, code),
        ],
    );
    let err = inst.run("Main", vec![]).unwrap_err();
    let vba::Error::Run(run) = &err else { panic!() };
    let info = run.error_info().unwrap();
    // The nested error reaches the outer code unchanged and, unhandled,
    // the embedder with its details.
    assert_eq!(
        (info.number, info.description.as_str(), info.source.as_str()),
        (5001, "boom 1", "Math")
    );
    assert_eq!(inst.output(), " 3 \n 15 \n");
}

#[test]
fn nested_errors_are_trappable_and_reentrancy_is_bounded() {
    let db = orders_db();
    let code = "\
Public Sub Main()
  On Error Resume Next
  Application.Run \"Boom\", 2
  Debug.Print Err.Number; Err.Description; Err.Source
  Err.Clear
  Debug.Print Application.Run(\"Depth\", 3)
  Debug.Print Application.Run(\"Depth\", 30)
  Debug.Print Err.Number
End Sub
";
    let mut inst = load(
        &db,
        &[
            ("Math", ModuleKind::Standard, MATH),
            ("Main1", ModuleKind::Standard, code),
        ],
    );
    let r = inst.run("Main", vec![]).unwrap();
    // Depth 3 nests three host calls and works; 30 hits the reentrancy
    // limit (4), which surfaces as an ordinary trappable error 5 in the
    // frame that made the call, not as a crash or a stack overflow. The
    // failing `Debug.Print` statement is skipped by `Resume Next`.
    assert_eq!(r.status, vba::RunStatus::Completed);
    assert_eq!(inst.output(), " 5002 boom 2Math\n 3 \n 5 \n");
}

#[test]
fn the_embedder_calls_by_module_qualified_and_named_arguments() {
    let db = orders_db();
    let mut inst = load(&db, &[("Math", ModuleKind::Standard, MATH)]);
    let r = inst
        .call(
            CallTarget::Project,
            "Math.Add",
            vec![Arg::val(1), Arg::named("b", Arg::val(41))],
        )
        .unwrap();
    assert_eq!(r.value, Variant::Long(42));
    let r = inst
        .call(
            CallTarget::Project,
            "add",
            vec![Arg::val(1), Arg::missing()],
        )
        .unwrap();
    assert_eq!(r.value, Variant::Long(11));
    let err = inst
        .call(CallTarget::Project, "Math.Nope", vec![])
        .unwrap_err();
    assert_eq!(err.kind(), "no-such-procedure");
}

#[test]
fn byref_variant_out_parameters_of_host_methods_write_back() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Public Sub Main()
  Dim v As Variant, ok As Boolean
  ok = Application.TryGetOption(\"Theme\", v)
  Debug.Print ok; v; TypeName(v)
  Dim s As String
  ok = Application.TryGetOption(\"Theme\", s)
  Debug.Print s
  ok = Application.TryGetOption(\"Nope\", v)
  Debug.Print ok; v
End Sub
",
    );
    assert_eq!(out, "TruedarkString\ndark\nFalsedark\n");
}

#[test]
fn byref_write_back_into_a_typed_variable_is_converted_or_fails_with_13() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Public Sub Main()
  Dim n As Long, ok As Boolean
  n = 7
  On Error Resume Next
  ok = Application.TryGetOption(\"Theme\", n)
  Debug.Print Err.Number; n; TypeName(n)
  Err.Clear
  On Error GoTo 0
  On Error GoTo trap
  ok = Application.TryGetOption(\"Theme\", n)
  Debug.Print \"not reached\"
  Exit Sub
trap:
  Debug.Print \"trapped\"; Err.Number
End Sub
",
    );
    assert_eq!(out, " 13  7 Long\ntrapped 13 \n");
}

#[test]
fn msgbox_and_inputbox_go_through_the_ui_hooks() {
    let db = orders_db();
    db.answers.borrow_mut().extend([6, 7]); // vbYes, vbNo
    db.inputs
        .borrow_mut()
        .extend([Some("typed".to_string()), None]);
    let out = run(
        &db,
        "\
Public Sub Main()
  Dim r As VbMsgBoxResult
  r = MsgBox(\"Delete?\", vbYesNo + vbQuestion, \"Confirm\")
  Debug.Print r = vbYes
  If MsgBox(\"Again?\", vbYesNo) = vbNo Then Debug.Print \"no\"
  MsgBox \"fyi\"
  Debug.Print InputBox(\"Name?\", \"T\", \"dflt\")
  Debug.Print \"[\" & InputBox(\"Name?\") & \"]\"
End Sub
",
    );
    assert_eq!(out, "True\nno\ntyped\n[]\n");
    assert_eq!(
        db.messages.borrow().as_slice(),
        [
            "Confirm|Delete?|36",
            "VBAProject|Again?|4",
            "VBAProject|fyi|0",
            "input:Name?|dflt",
            "input:Name?|"
        ]
    );
}

#[test]
fn without_ui_hooks_msgbox_fails_with_a_trappable_error() {
    let db = orders_db();
    let project = vba::Project::from_source(
        "M",
        "Public Sub Main()\n On Error Resume Next\n MsgBox \"x\"\n Debug.Print Err.Number\nEnd Sub\n",
    );
    let mut inst = vba::Engine::new().load(&project).unwrap();
    inst.trust();
    assert_eq!(inst.run("Main", vec![]).unwrap().output, " 70 \n");
    drop(db);
}

const COUNTER: &str = "\
Option Explicit
Private mTotal As Long
Private mLimit As Long
Public Name As String
Private Sub Class_Initialize()
  mLimit = 100
  Debug.Print \"init\"
End Sub
Private Sub Class_Terminate()
  Debug.Print \"term \" & mTotal
End Sub
Public Sub Add(ByVal n As Long)
  If mTotal + n > mLimit Then Err.Raise 6, \"Counter\", \"over limit\"
  mTotal = mTotal + n
End Sub
Public Property Get Total() As Long
  Total = mTotal
End Property
Public Property Let Limit(ByVal v As Long)
  mLimit = v
End Property
Public Property Get Limit() As Long
  Limit = mLimit
End Property
";

#[test]
fn a_class_module_is_instantiated_and_driven_from_host_code() {
    let db = orders_db();
    let mut inst = load(&db, &[("Counter", ModuleKind::Class, COUNTER)]);
    let c = inst.create_object("Counter", None).unwrap();
    inst.call_on(c, "Add", vec![Arg::val(5)]).unwrap();
    inst.call_on(c, "Add", vec![Arg::val(7)]).unwrap();
    assert_eq!(
        inst.call_on(c, "Total", vec![]).unwrap().value,
        Variant::Long(12)
    );
    assert_eq!(inst.get_member(c, "Total").unwrap(), Variant::Long(12));
    // Property Let through set_member, a public field the same way.
    inst.set_member(c, "Limit", Variant::Long(20)).unwrap();
    inst.set_member(c, "Name", Variant::from("main")).unwrap();
    assert_eq!(inst.get_member(c, "Limit").unwrap(), Variant::Long(20));
    assert_eq!(inst.get_member(c, "Name").unwrap(), Variant::from("main"));
    // A VBA error from the instance reaches the host with its details.
    let e = inst.call_on(c, "Add", vec![Arg::val(50)]).unwrap_err();
    let vba::Error::Run(run) = &e else { panic!() };
    assert_eq!(run.error_info().unwrap().description, "over limit");
    inst.destroy_object(c).unwrap();
    assert_eq!(inst.output(), "init\nterm 12\n");
}

const WATCHER: &str = "\
Option Explicit
Private WithEvents mBtn As Access.Control
Public Clicks As Long
Public Sub Attach(ByVal b As Object)
  Set mBtn = b
End Sub
Private Sub mBtn_Click()
  Clicks = Clicks + 1
  Debug.Print \"watcher click \" & Clicks
End Sub
";

const WIRE: &str = "\
Option Explicit
Public W As Watcher
Public Sub Wire()
  Set W = New Watcher
  W.Attach Forms!Orders!cmdSave
End Sub
Public Sub Press()
  Forms!Orders!cmdSave.Click
End Sub
Public Function Clicks() As Long
  Clicks = W.Clicks
End Function
Public Sub Unwire()
  Set W = Nothing
End Sub
";

#[test]
fn withevents_on_a_host_control_gets_events_from_vba_and_from_the_host() {
    let db = forms_db();
    let mut inst = load_typed(
        &db,
        &[
            (
                "Module1",
                ModuleKind::Standard,
                "Public Sub OpenOrders()\n DoCmd.OpenForm \"Orders\"\nEnd Sub\n",
            ),
            ("Form_Orders", ModuleKind::Document, FORM_ORDERS),
            ("Watcher", ModuleKind::Class, WATCHER),
            ("Wire", ModuleKind::Standard, WIRE),
        ],
    );
    call(&mut inst, "OpenOrders");
    call(&mut inst, "Wire");
    // Raised by VBA through the control (reentrant host object): at once.
    assert_eq!(call(&mut inst, "Press"), "watcher click 1\n");

    // Raised by the host between calls: queued, delivered on request.
    let st = form(&db, "Orders");
    let cp = &st.ctl("cmdSave").unwrap().1.cp;
    cp.fire(inst.session().host().objects(), "Click", &mut [])
        .unwrap();
    let before = inst.output().len();
    inst.deliver_pending_events().unwrap();
    assert_eq!(&inst.output()[before..], "watcher click 2\n");
    assert_eq!(cp.sink_count(), 1);

    // Dropping the watcher unadvises.
    call(&mut inst, "Unwire");
    assert_eq!(cp.sink_count(), 0);
    assert_eq!(call(&mut inst, "Press"), "");
}

#[test]
fn property_set_on_a_host_object_keeps_the_recordset_alive() {
    let db = forms_db();
    let mut inst = load_typed(
        &db,
        &[
            (
                "Module1",
                ModuleKind::Standard,
                "Public Sub OpenDetails()\n DoCmd.OpenForm \"Details\"\nEnd Sub\n",
            ),
            ("Form_Details", ModuleKind::Document, "Option Explicit\n"),
            (
                "Probe",
                ModuleKind::Standard,
                "\
Option Explicit
Public Sub Main()
  Dim rs As DAO.Recordset, f As Access.Form
  Set f = Forms!Details
  Debug.Print f.Recordset Is Nothing
  Set rs = CurrentDb.OpenRecordset(\"Orders\")
  Set f.Recordset = rs
  Set rs = Nothing
  Debug.Print f.Recordset Is Nothing; f.Recordset.RecordCount; f.Recordset!Customer
  f.Recordset.MoveNext
  Debug.Print Forms!Details.Recordset!Customer
End Sub
",
            ),
        ],
    );
    call(&mut inst, "OpenDetails");
    assert_eq!(call(&mut inst, "Main"), "True\nFalse 3 Ann\nBob\n");
}
