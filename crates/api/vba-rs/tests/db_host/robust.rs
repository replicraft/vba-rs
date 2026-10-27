//! Hostile and awkward host behavior: stale handles, odd error numbers,
//! arrays, dates, leaks, limits and failing handlers.

use vba::automation::testing::MemObject;
use vba::automation::{DispError, ObjectHandle, ObjectRef, ParamInfo};
use vba::{CallLimits, CallTarget, Engine, ModuleKind, Project, Variant};

use crate::forms::*;
use crate::host::*;
use crate::{orders_db, run};

fn plain(engine: Engine, code: &str) -> (vba::Instance, String) {
    let mut inst = engine.load(&Project::from_source("M", code)).unwrap();
    inst.trust();
    let out = match inst.run("Main", vec![]) {
        Ok(o) => o.output,
        Err(e) => format!("ERR {} {e}", e.kind()),
    };
    (inst, out)
}

#[test]
fn host_errors_with_odd_numbers_never_pass_as_success_or_panic() {
    let engine = Engine::new().dispatch_global("Host", || {
        MemObject::new("Host")
            .method("Zero", vec![], |_, _| Err(DispError::raise(0, "zero", "H")))
            .method("Neg", vec![], |_, _| Err(DispError::raise(-5, "neg", "H")))
            .method("Huge", vec![], |_, _| {
                Err(DispError::raise(70000, "huge", "H"))
            })
            .method("Fail", vec![], |_, _| {
                Err(DispError::Hresult(0x8000_4005_u32 as i32))
            })
            .method("Blank", vec![], |_, _| Err(DispError::raise(7777, "", "")))
    });
    let (_, out) = plain(
        engine,
        "\
Public Sub Main()
  On Error Resume Next
  Dim m
  For Each m In Array(\"Zero\", \"Neg\", \"Huge\", \"Fail\", \"Blank\")
    Err.Clear
    CallByName Host, m, VbMethod
    Debug.Print m; \"=\"; Err.Number; \"|\"; Err.Description; \"|\"; Err.Source
  Next
End Sub
",
    );
    // Facts worth pinning: every failure is an error (a host error number
    // 0 does not turn into success), and nothing panics.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 5, "{out}");
    // Number 0 is raised as 5; the others are kept as given.
    assert_eq!(lines[0], "Zero= 5 |zero|H");
    assert!(lines[1].starts_with("Neg=-5 |neg|H"), "{out}");
    assert!(lines[3].starts_with("Fail=-2147467259 |"), "{out}");
    // An empty description and source fall back to the generic text and the
    // class name of the object.
    assert_eq!(
        lines[4],
        "Blank= 7777 |Application-defined or object-defined error|Host"
    );
}

#[test]
fn a_stale_object_handle_returned_by_the_host_is_an_error_not_a_crash() {
    let engine = Engine::new().dispatch_global("Host", || {
        MemObject::new("Host")
            .method("Ghost", vec![], |_, _| {
                Ok(Variant::Object(ObjectRef::Handle(ObjectHandle(987_654))))
            })
            .method("Plain", vec![], |_, _| Ok(Variant::Long(1)))
    });
    let (mut inst, out) = plain(
        engine,
        "\
Public Sub Main()
  On Error Resume Next
  Dim o As Object
  Set o = Host.Ghost
  Err.Clear
  Debug.Print o.Anything
  Debug.Print Err.Number <> 0
  Debug.Print Host.Plain
End Sub
",
    );
    // Using the dead handle fails like any bad object; nothing crashes.
    assert_eq!(out, "True\n 1 \n");
    assert!(inst.is_usable());
    assert!(inst.run("Main", vec![]).is_ok());
}

#[test]
fn arrays_dates_and_currency_cross_the_boundary() {
    let engine = Engine::new().dispatch_global("Host", || {
        MemObject::new("Host")
            .method("Names", vec![], |_, _| {
                Ok(Variant::from(vec![
                    Variant::from("a"),
                    Variant::from("b"),
                    Variant::Null,
                ]))
            })
            .method("Stamp", vec![], |_, _| {
                Ok(Variant::date_from_ymd_hms(2024, 2, 29, 13, 5, 0).unwrap())
            })
            .method("Echo", vec![ParamInfo::required("v").by_val()], |_, c| {
                Ok(c.args[0].value.clone())
            })
    });
    let (_, out) = plain(
        engine,
        "\
Public Sub Main()
  Dim v As Variant, x As Variant, s As String
  v = Host.Names
  Debug.Print LBound(v); UBound(v); IsArray(v)
  For Each x In v
    s = s & Nz2(x) & \",\"
  Next
  Debug.Print s
  Debug.Print Format(Host.Stamp, \"yyyy-mm-dd hh:nn\"); Weekday(Host.Stamp); DateAdd(\"d\", 1, Host.Stamp) > Host.Stamp
  Debug.Print Host.Echo(CCur(1.25)) * 2; TypeName(Host.Echo(CCur(1))); TypeName(Host.Echo(#1/1/2000#))
  Debug.Print UBound(Host.Echo(Array(1, 2, 3)))
End Sub
Private Function Nz2(v As Variant) As String
  If IsNull(v) Then Nz2 = \"<null>\" Else Nz2 = v
End Function
",
    );
    assert_eq!(
        out,
        " 0  2 True\na,b,<null>,\n2024-02-29 13:05 5 True\n 2.5 CurrencyDate\n 2 \n"
    );
}

#[test]
fn opening_and_closing_forms_does_not_leak_host_objects() {
    let db = forms_db();
    let probe = "\
Option Explicit
Public Sub Cycle()
  Dim i As Long, f As Object, c As Object
  For i = 1 To 3
    DoCmd.OpenForm \"Orders\"
    Set f = Forms!Orders
    For Each c In f.Controls
      Debug.Print c.Name;
    Next
    Debug.Print
    Set c = Nothing
    Set f = Nothing
    DoCmd.Close acForm, \"Orders\"
  Next
End Sub
";
    let mut inst = forms_instance_with(&db, &[("Probe", ModuleKind::Standard, probe)]);
    // Warm up once: global objects are created on first use and cached.
    call(&mut inst, "Cycle");
    let live = inst.session().host().objects().live_count();
    call(&mut inst, "Cycle");
    call(&mut inst, "Cycle");
    assert_eq!(
        inst.session().host().objects().live_count(),
        live,
        "each cycle must give every object back"
    );
    assert!(db.forms.borrow().is_empty());
}

#[test]
fn a_runaway_handler_called_through_the_host_hits_the_call_limit_and_the_instance_recovers() {
    let db = forms_db();
    let spin = "\
Option Explicit
Private Sub Form_Load()
  Dim i As Long
  Do
    i = i + 1
  Loop
End Sub
";
    let mut inst = load(
        &db,
        &[
            (
                "Module1",
                ModuleKind::Standard,
                "Public Sub OpenOrders()\n DoCmd.OpenForm \"Orders\"\nEnd Sub\nPublic Function Ok() As Long\n Ok = 1\nEnd Function\n",
            ),
            ("Form_Orders", ModuleKind::Document, spin),
        ],
    );
    let r = inst.with_call_limits(CallLimits::new().max_steps(50_000), |i| {
        i.call(CallTarget::Project, "OpenOrders", vec![])
    });
    let err = r.unwrap_err();
    assert_eq!(err.kind(), "limit", "{err}");
    // A fatal fault ends the whole run, nested part included; the host's own
    // state (a half-opened form) is the host's to clean up.
    inst.reset_step_counter();
    assert!(inst.is_usable());
    assert_eq!(
        inst.call(CallTarget::Project, "Ok", vec![]).unwrap().value,
        Variant::Long(1)
    );
}

#[test]
fn a_failing_handler_reports_its_stack_and_does_not_stop_later_events() {
    let db = forms_db();
    let src = "\
Option Explicit
Private Sub Form_Current()
  Helper
End Sub
Private Sub Helper()
  Dim n As Long
  n = 1 \\ 0
End Sub
Private Sub cmdSave_Click()
  Debug.Print \"still alive\"
End Sub
";
    let mut inst = load(
        &db,
        &[
            (
                "Module1",
                ModuleKind::Standard,
                "Public Sub OpenOrders()\n DoCmd.OpenForm \"Orders\"\nEnd Sub\n",
            ),
            ("Form_Orders", ModuleKind::Document, src),
        ],
    );
    // The error inside Form_Current surfaces through DoCmd.OpenForm.
    let err = inst
        .call(CallTarget::Project, "OpenOrders", vec![])
        .unwrap_err();
    let vba::Error::Run(run) = &err else { panic!() };
    let info = run.error_info().unwrap();
    assert_eq!(info.number, 11);
    // Known limit: the frames of the nested handler are not part of the
    // outer report (number, description and source are).
    let procs: Vec<String> = run
        .report()
        .unwrap()
        .frames
        .iter()
        .map(|f| f.procedure.clone())
        .collect();
    assert_eq!(procs, ["OpenOrders"]);
    assert!(inst.is_usable());
    // The form is half open on the host side; later events still run.
    let st = form(&db, "Orders");
    let obj = st.object.get().unwrap();
    let out = inst
        .raise_event_on(obj, "cmdSave", "Click", vec![])
        .unwrap();
    assert_eq!(out.output, "still alive\n");
}

#[test]
fn call_by_name_reaches_host_methods_and_properties() {
    let db = forms_db();
    let probe = "\
Option Explicit
Public Sub Main()
  Dim f As Object
  Set f = Forms!Orders
  CallByName f, \"Greet\", VbMethod, \"cbn\"
  CallByName f.Controls(\"Customer\"), \"Visible\", VbLet, False
  Debug.Print CallByName(f.Controls(\"Customer\"), \"Visible\", VbGet)
  CallByName f, \"Caption\", VbLet, \"Renamed\"
  Debug.Print CallByName(f, \"Caption\", VbGet); CallByName(f, \"Doubled\", VbGet)
End Sub
";
    let mut inst = forms_instance_with(&db, &[("Probe", ModuleKind::Standard, probe)]);
    call(&mut inst, "OpenOrders");
    let out = call(&mut inst, "Main");
    assert_eq!(out, "hello cbn from Orders\nFalse\nRenamed 20 \n");
}

#[test]
fn undeclared_control_names_are_reported_at_load_not_at_run() {
    let db = forms_db();
    let mut p = Project::new();
    p.add_module(
        "Form_Orders",
        ModuleKind::Document,
        "Option Explicit\nPrivate Sub Form_Current()\n  txtMissing = 1\nEnd Sub\n",
    )
    .unwrap();
    let report = engine(&db).analyze(&p);
    assert!(report.has_errors());
    let text = report.rendered().join("\n");
    assert!(text.contains("txtMissing"), "{text}");
    let err = engine(&db).load(&p).expect_err("refused");
    assert_eq!(err.kind(), "load");
}

#[test]
fn run_helper_keeps_working_after_all_of_the_above() {
    let db = orders_db();
    assert_eq!(
        run(
            &db,
            "Public Sub Main()\n Debug.Print DCount(\"*\", \"Orders\")\nEnd Sub\n"
        ),
        " 3 \n"
    );
}
