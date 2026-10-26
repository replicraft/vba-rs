//! Host objects that call back into the running session: nested instances
//! and events, ByRef write-back to the host, shared state and limits, error
//! propagation, and synchronous events from host objects.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::*;
use vba::automation::{
    ConnectionPoint, DispError, DispId, DispParams, Dispatch, EventSource, InvokeFlags, MemberInfo,
    ObjectTable,
};
use vba::runtime::Error as RunError;
use vba::{
    Arg, CallTarget, Capability, Engine, Error, HostContext, ModuleKind, RunStatus, Variant,
};

const ORDERS: &str = "\
Private Sub Command1_Click()
  Debug.Print \"click begin\"
  DoCmd.OpenForm \"Details\", , \"A1\"
  Debug.Print \"click end\"
End Sub

Private Sub Command2_Click()
  Me.CustomerName = Null
  If Me.Update() Then Debug.Print \"saved\" Else Debug.Print \"refused\"
  Me.CustomerName = \"Ann\"
  If Me.Update() Then Debug.Print \"saved\" Else Debug.Print \"refused\"
End Sub

Private Sub Form_BeforeUpdate(Cancel As Integer)
  If IsNull(Me.CustomerName) Then Cancel = True
  Debug.Print \"before update, cancel=\" & Cancel
End Sub

Public Sub TryOpen(what As String)
  On Error Resume Next
  DoCmd.OpenForm what
  Debug.Print \"err \" & Err.Number & \" \" & Err.Description
End Sub

Public Sub OpenQuietly(what As String)
  On Error Resume Next
  DoCmd.OpenForm what
  Debug.Print \"still running\"
End Sub
";

const DETAILS: &str = "\
Private Sub Form_Open(Cancel As Integer)
  Debug.Print \"details open \" & Me.OpenArgs & \" limit=\" & Application.Setting(\"Limit\")
  If Application.Setting(\"Block\") Then Cancel = True
End Sub

Private Sub Form_Load()
  Debug.Print \"details load\"
End Sub
";

const CONFIG: &str = "Public Limit As Long\nPublic Block As Boolean\n";

fn modules() -> Vec<(&'static str, ModuleKind, &'static str)> {
    vec![
        ("Form_Orders", ModuleKind::Document, ORDERS),
        ("Form_Details", ModuleKind::Document, DETAILS),
        (
            "Form_Boom",
            ModuleKind::Document,
            "Private Sub Form_Load()\n  Err.Raise 6002, \"Boom\", \"load failed\"\nEnd Sub\n",
        ),
        (
            "Form_Spin",
            ModuleKind::Document,
            "Private Sub Form_Load()\n  Do\n  Loop\nEnd Sub\n",
        ),
        (
            "Form_Stop",
            ModuleKind::Document,
            "Private Sub Form_Load()\n  Debug.Print \"ending\"\n  End\nEnd Sub\n",
        ),
        ("Config", ModuleKind::Standard, CONFIG),
        (
            "Module1",
            ModuleKind::Standard,
            "Public Function Rec(ByVal n As Long) As Long\n  Debug.Print n\n  Rec = Application.Run(\"Rec\", n + 1)\nEnd Function\nPublic Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\nPublic Function ViaHost(ByVal n As Long) As Long\n  ViaHost = Application.Run(\"Twice\", n) + 1\nEnd Function\nPublic Function Burn(ByVal n As Long) As Long\n  Dim i As Long\n  For i = 1 To n\n  Next\n  Burn = n\nEnd Function\nPublic Function ViaBurn(ByVal n As Long) As Long\n  ViaBurn = Application.Run(\"Burn\", n)\nEnd Function\n",
        ),
    ]
}

fn setup() -> (Rc<AppState>, vba::Instance) {
    let app = Rc::new(AppState::default());
    let engine = app_engine(&app).host_members(["CustomerName"]);
    let mut inst = load_app(&engine, &modules());
    inst.set_global("Config.Limit", Variant::Long(5)).unwrap();
    (app, inst)
}

fn orders_form(inst: &mut vba::Instance) -> (Rc<FormState>, vba::automation::ObjectRef) {
    let st = FormState::new(&[
        ("CustomerName", Variant::from("Ann")),
        ("Caption", Variant::from("Orders")),
    ]);
    let host = FormHost::new(&st);
    let form = inst
        .create_object("Form_Orders", Some(host.clone()))
        .unwrap();
    host.me.set(Some(form));
    (st, form)
}

#[test]
fn opening_a_form_runs_its_handlers_synchronously_in_the_calling_statement() {
    let (app, mut inst) = setup();
    let (_st, orders) = orders_form(&mut inst);
    let out = inst
        .raise_event_on(orders, "Command1", "Click", vec![])
        .unwrap();
    // The nested Open and Load handlers ran inside the OpenForm statement,
    // before "click end"; they read a VBA global through the host.
    assert_eq!(
        out.output,
        "click begin\ndetails open A1 limit=5\ndetails load\nclick end\n"
    );
    assert_eq!(
        *app.log.borrow(),
        [
            "OpenForm Details begin",
            "Setting Limit",
            "Setting Block",
            "OpenForm Details end"
        ]
    );
    // A second, separate instance exists, with its own host object.
    let forms = app.forms.borrow();
    assert_eq!(forms.len(), 1);
    assert_ne!(forms[0].object, orders);
    assert_eq!(forms[0].state.get("OpenArgs"), Variant::from("A1"));
    drop(forms);
    // And it is a real instance the embedder can keep using.
    let details = app.forms.borrow()[0].object;
    assert!(inst.has_procedure(details, "Form_Load"));
    inst.destroy_object(details).unwrap();
}

#[test]
fn a_handler_cancel_flag_is_visible_to_the_host_and_stops_the_open() {
    let (app, mut inst) = setup();
    inst.set_global("Config.Block", Variant::Boolean(true))
        .unwrap();
    let (_st, orders) = orders_form(&mut inst);
    let err = inst
        .raise_event_on(orders, "Command1", "Click", vec![])
        .unwrap_err();
    // The host destroyed the half-open form and raised its own error 2501.
    match &err {
        Error::Run(RunError::Host(r)) => assert_eq!(r.number, 2501, "{r}"),
        other => panic!("{other:?}"),
    }
    assert!(app.forms.borrow().is_empty());
    assert_eq!(
        *app.log.borrow(),
        [
            "OpenForm Details begin",
            "Setting Limit",
            "Setting Block",
            "OpenForm Details cancelled"
        ]
    );
    // No Load handler ran.
    assert!(!inst.output().contains("details load"), "{}", inst.output());
}

#[test]
fn before_update_cancel_is_written_back_to_the_host() {
    let (_app, mut inst) = setup();
    let (st, orders) = orders_form(&mut inst);
    let out = inst
        .raise_event_on(orders, "Command2", "Click", vec![])
        .unwrap();
    assert_eq!(
        out.output,
        "before update, cancel=-1\nrefused\nbefore update, cancel=0\nsaved\n"
    );
    let updates: Vec<String> = st
        .log()
        .into_iter()
        .filter(|l| l.starts_with("update"))
        .collect();
    assert_eq!(updates, ["update cancelled", "update committed"]);
}

#[test]
fn nested_errors_reach_the_outer_code_as_trappable_errors() {
    let (app, mut inst) = setup();
    // The outer procedure traps it.
    let r = inst
        .call(
            CallTarget::Project,
            "Form_Orders.TryOpen",
            vec![Arg::val("Boom")],
        )
        .unwrap();
    assert_eq!(r.output, "err 6002 load failed\n");
    // The failed form was created before the error; the host keeps it.
    assert_eq!(app.forms.borrow().len(), 1);
    // The same procedure on a form that opens fine just runs through.
    let err = inst
        .call(
            CallTarget::Project,
            "Form_Orders.TryOpen",
            vec![Arg::val("Details")],
        )
        .unwrap();
    assert!(err.output.starts_with("details open"), "{}", err.output);
}

#[test]
fn a_nested_error_the_host_returns_unhandled_ends_the_outer_call() {
    let app = Rc::new(AppState::default());
    let engine = app_engine(&app);
    let mut inst = load_app(
        &engine,
        &[
            (
                "Form_Boom",
                ModuleKind::Document,
                "Private Sub Form_Load()\n  Err.Raise 6002, \"Boom\", \"load failed\"\nEnd Sub\n",
            ),
            (
                "Module1",
                ModuleKind::Standard,
                "Public Sub Go()\n  DoCmd.OpenForm \"Boom\"\n  Debug.Print \"not reached\"\nEnd Sub\n",
            ),
        ],
    );
    let err = inst.call(CallTarget::Project, "Go", vec![]).unwrap_err();
    let report = match &err {
        Error::Run(r) => r.report().expect("a report"),
        other => panic!("{other:?}"),
    };
    assert_eq!(report.number, 6002);
    assert_eq!(report.description, "load failed");
    assert!(matches!(err, Error::Run(RunError::Host(_))), "{err:?}");
    assert_eq!(report.frames.len(), 1);
    assert_eq!(report.frames[0].procedure, "Go");
    assert!(!inst.output().contains("not reached"));
}

#[test]
fn a_fatal_fault_in_nested_code_ends_the_outer_run_even_if_the_host_swallows_it() {
    let app = Rc::new(AppState::default());
    let engine = app_engine(&app).max_steps(50_000);
    let mut inst = load_app(&engine, &modules());
    // The host returns the nested error; VBA has On Error Resume Next.
    let err = inst
        .call(
            CallTarget::Project,
            "Form_Orders.OpenQuietly",
            vec![Arg::val("Spin")],
        )
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
    assert!(
        !inst.output().contains("still running"),
        "{}",
        inst.output()
    );
}

#[test]
fn end_in_nested_code_ends_the_whole_program() {
    let (app, mut inst) = setup();
    let (_st, orders) = orders_form(&mut inst);
    inst.set_global("Config.Limit", Variant::Long(1)).unwrap();
    let r = inst
        .call(
            CallTarget::Project,
            "Form_Orders.OpenQuietly",
            vec![Arg::val("Stop")],
        )
        .unwrap();
    assert_eq!(r.status, RunStatus::Ended);
    assert!(inst.output().contains("ending"), "{}", inst.output());
    assert!(!inst.output().contains("still running"));
    let _ = (app, orders);
}

#[test]
fn nesting_is_bounded() {
    let (_app, mut inst) = setup();
    let err = inst
        .call(CallTarget::Project, "Rec", vec![Arg::val(0)])
        .unwrap_err();
    // Levels 0..=4 ran: the top-level call plus four nested runs.
    assert_eq!(inst.output(), " 0 \n 1 \n 2 \n 3 \n 4 \n");
    let report = match &err {
        Error::Run(r) => r.report().expect("a report"),
        other => panic!("{other:?}"),
    };
    assert!(
        report.description.contains("re-entrancy depth limit (4)"),
        "{report}"
    );
    // The limit is configurable.
    let app = Rc::new(AppState::default());
    let engine = app_engine(&app).vm_config(vba::VmConfig {
        max_reentrancy_depth: 8,
        ..vba::VmConfig::default()
    });
    let mut inst = load_app(&engine, &modules());
    let _ = inst
        .call(CallTarget::Project, "Rec", vec![Arg::val(0)])
        .unwrap_err();
    assert_eq!(
        inst.output(),
        " 0 \n 1 \n 2 \n 3 \n 4 \n 5 \n 6 \n 7 \n 8 \n"
    );
}

#[test]
fn nested_calls_share_state_and_the_step_budget() {
    let (_app, mut inst) = setup();
    let r = inst
        .call(CallTarget::Project, "ViaHost", vec![Arg::val(20)])
        .unwrap();
    assert_eq!(r.value, Variant::Long(41));

    // Steps used by the loop alone...
    let steps = |inst: &vba::Instance| inst.session().host().budget().steps_used();
    let app = Rc::new(AppState::default());
    let mut direct = load_app(&app_engine(&app), &modules());
    direct
        .call(CallTarget::Project, "Burn", vec![Arg::val(500)])
        .unwrap();
    let alone = steps(&direct);
    assert!(alone > 500);

    // ...are charged to the one budget when a host object runs it nested,
    // on top of what the outer code spends: a limit that is just enough for
    // the loop alone is exceeded.
    let engine = app_engine(&app).max_steps(alone);
    let mut limited = load_app(&engine, &modules());
    limited
        .call(CallTarget::Project, "Burn", vec![Arg::val(500)])
        .unwrap();
    let mut limited = load_app(&engine, &modules());
    let err = limited
        .call(CallTarget::Project, "ViaBurn", vec![Arg::val(500)])
        .unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Limit(_))), "{err:?}");
}

/// Keeps the context it is given, to prove it stops working.
struct Spy {
    kept: Rc<RefCell<Option<HostContext>>>,
    reentrant: bool,
}

impl Dispatch for Spy {
    fn class_name(&self) -> String {
        "Spy".into()
    }
    fn reentrant(&self) -> bool {
        self.reentrant
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("Grab").then_some(DispId(1))
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == DispId(1)).then(|| MemberInfo {
            name: "Grab".into(),
            id,
            supports: InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            params: vec![],
        })
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        _: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let ctx = HostContext::current(table);
        let got = ctx.is_some();
        *self.kept.borrow_mut() = ctx;
        Ok(Variant::Boolean(got))
    }
}

#[test]
fn a_context_is_only_valid_during_its_host_call_and_only_for_reentrant_objects() {
    let kept = Rc::new(RefCell::new(None));
    for (reentrant, expect) in [(true, true), (false, false)] {
        let k = kept.clone();
        let engine = Engine::new().dispatch_global("Spy", move || Spy {
            kept: k.clone(),
            reentrant,
        });
        let mut inst = load_app(
            &engine,
            &[(
                "Module1",
                ModuleKind::Standard,
                "Public Function Go() As Boolean\n  Go = Spy.Grab\nEnd Function\n",
            )],
        );
        kept.borrow_mut().take();
        let r = inst.call(CallTarget::Project, "Go", vec![]).unwrap();
        assert_eq!(r.value, Variant::Boolean(expect));
        if expect {
            let ctx = kept.borrow().clone().expect("kept");
            assert!(!ctx.is_active());
            let err = ctx.get_global("Module1.x").unwrap_err();
            assert!(
                matches!(err, Error::Run(RunError::Reentrancy(_))),
                "{err:?}"
            );
            assert_eq!(err.kind(), "reentrancy");
        } else {
            assert!(kept.borrow().is_none());
        }
    }
}

// ---- synchronous events from host objects --------------------------------------

/// A host button: `Press()` fires `Click(Cancel)` and returns whether the
/// handler left `Cancel` set. With `reentrant` it is delivered at once.
struct Button {
    cp: ConnectionPoint,
    reentrant: bool,
}

impl Dispatch for Button {
    fn class_name(&self) -> String {
        "Button".into()
    }
    fn reentrant(&self) -> bool {
        self.reentrant
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("Press").then_some(DispId(1))
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == DispId(1)).then(|| MemberInfo {
            name: "Press".into(),
            id,
            supports: InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            params: vec![],
        })
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        _: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let mut args = vec![vba::automation::Arg::by_ref(Variant::Boolean(false))];
        self.cp.fire(table, "Click", &mut args)?;
        Ok(args[0].value.clone())
    }
    fn event_source(&self) -> Option<&dyn EventSource> {
        Some(&self.cp)
    }
}

fn button_instance(reentrant: bool) -> vba::Instance {
    use vba::raw::sema::{LibEvent, LibMember, LibParam, StaticLibrary, Type};
    let lib = StaticLibrary::new("Lib")
        .with_type(
            "Button",
            vec![LibMember::function("Press", vec![], Type::Boolean)],
        )
        .with_events(
            "Button",
            vec![LibEvent {
                name: "Click".into(),
                params: vec![LibParam {
                    by_ref: true,
                    ..LibParam::required("Cancel", Type::Boolean)
                }],
            }],
        );
    let engine =
        Engine::new()
            .grant(Capability::ComCreate)
            .dispatch_class("Lib.Button", move || Button {
                cp: ConnectionPoint::new("ButtonEvents"),
                reentrant,
            });
    let mut p = vba::Project::new();
    p.add_module(
        "Watcher",
        ModuleKind::Class,
        "Private WithEvents btn As Lib.Button\nPublic Sub Attach(b As Lib.Button)\n  Set btn = b\nEnd Sub\nPrivate Sub btn_Click(Cancel As Boolean)\n  Debug.Print \"handler\"\n  Cancel = True\nEnd Sub\n",
    )
    .unwrap();
    p.add_module(
        "Module1",
        ModuleKind::Standard,
        "Public Sub Go()\n  Dim b As Lib.Button\n  Set b = CreateObject(\"Lib.Button\")\n  Dim w As New Watcher\n  w.Attach b\n  Debug.Print \"pressed=\" & b.Press()\n  Debug.Print \"done\"\nEnd Sub\n",
    )
    .unwrap();
    let mut inst = engine
        .load_with(&p, |b| b.extra_provider(lib))
        .expect("loads");
    inst.trust();
    inst
}

#[test]
fn events_from_a_reentrant_host_object_are_synchronous_with_byref_write_back() {
    let mut inst = button_instance(true);
    let r = inst.call(CallTarget::Project, "Go", vec![]).unwrap();
    // The handler ran inside Press; its Cancel reached the host, which
    // returned it to the VBA statement that was waiting.
    assert_eq!(r.output, "handler\npressed=True\ndone\n");
}

#[test]
fn events_from_other_host_objects_are_still_queued_until_the_call_returns() {
    let mut inst = button_instance(false);
    let r = inst.call(CallTarget::Project, "Go", vec![]).unwrap();
    // Queued while Press ran, delivered right after the call returned: the
    // handler's Cancel cannot reach the host (False), unlike above.
    assert_eq!(r.output, "handler\npressed=False\ndone\n");
}

#[test]
fn analysis_knows_the_engines_globals_and_host_members() {
    let mut p = vba::Project::new();
    p.add_module(
        "Module1",
        ModuleKind::Standard,
        "Option Explicit\nPublic Sub Go()\n  DoCmd.OpenForm \"Details\"\n  Application.Run \"Go\"\nEnd Sub\n",
    )
    .unwrap();
    p.add_module(
        "Form_Orders",
        ModuleKind::Document,
        "Option Explicit\nPrivate Sub Form_Current()\n  Caption = \"x\"\nEnd Sub\n",
    )
    .unwrap();
    // A plain engine does not know DoCmd, Application or Caption.
    let plain = Engine::new().analyze(&p);
    assert!(plain.has_errors());
    let app = Rc::new(AppState::default());
    let report = app_engine(&app).host_members(["Caption"]).analyze(&p);
    assert!(!report.has_errors(), "{:?}", report.rendered());
}

/// A button that stays reachable from the test, and a watcher that keeps it.
fn persistent_button() -> (vba::Instance, Rc<RefCell<Option<Rc<Button>>>>) {
    use vba::automation::ObjectRef;
    use vba::raw::sema::{LibEvent, LibMember, LibParam, StaticLibrary, Type};
    let lib = StaticLibrary::new("Lib")
        .with_type(
            "Button",
            vec![LibMember::function("Press", vec![], Type::Boolean)],
        )
        .with_events(
            "Button",
            vec![LibEvent {
                name: "Click".into(),
                params: vec![LibParam {
                    by_ref: true,
                    ..LibParam::required("Cancel", Type::Boolean)
                }],
            }],
        );
    let kept: Rc<RefCell<Option<Rc<Button>>>> = Rc::default();
    let k = kept.clone();
    let engine = Engine::new()
        .grant(Capability::ComCreate)
        .class("Lib.Button", move |table| {
            let b = Rc::new(Button {
                cp: ConnectionPoint::new("ButtonEvents"),
                reentrant: true,
            });
            *k.borrow_mut() = Some(b.clone());
            Ok(Variant::Object(ObjectRef::Handle(table.insert(b))))
        });
    let mut p = vba::Project::new();
    p.add_module(
        "Watcher",
        ModuleKind::Class,
        "Private WithEvents btn As Lib.Button\nPrivate n As Long\nPublic Sub Attach(b As Lib.Button)\n  Set btn = b\nEnd Sub\nPrivate Sub btn_Click(Cancel As Boolean)\n  n = n + 1\n  Debug.Print \"handler \" & n\nEnd Sub\n",
    )
    .unwrap();
    p.add_module(
        "Module1",
        ModuleKind::Standard,
        "Public B As Lib.Button\nPublic W As Watcher\nPublic Sub Setup()\n  Set B = CreateObject(\"Lib.Button\")\n  Set W = New Watcher\n  W.Attach B\nEnd Sub\nPublic Sub Press()\n  Debug.Print \"pressed=\" & B.Press()\nEnd Sub\n",
    )
    .unwrap();
    let mut inst = engine
        .load_with(&p, |b| b.extra_provider(lib))
        .expect("loads");
    inst.trust();
    (inst, kept)
}

fn fire_from_outside(inst: &vba::Instance, kept: &Rc<RefCell<Option<Rc<Button>>>>) {
    let button = kept.borrow().clone().expect("created by Setup");
    let mut args = vec![vba::automation::Arg::by_ref(Variant::Boolean(false))];
    button
        .cp
        .fire(inst.session().host().objects(), "Click", &mut args)
        .unwrap();
}

#[test]
fn events_raised_outside_a_call_are_queued_and_delivered_in_order() {
    let (mut inst, kept) = persistent_button();
    inst.call(CallTarget::Project, "Setup", vec![]).unwrap();
    let before = inst.output().len();

    // The host raises while no VBA statement is waiting: queued.
    fire_from_outside(&inst, &kept);
    fire_from_outside(&inst, &kept);
    assert_eq!(&inst.output()[before..], "");
    inst.deliver_pending_events().unwrap();
    assert_eq!(&inst.output()[before..], "handler 1\nhandler 2\n");

    // An older queued event is delivered before a newer synchronous one.
    fire_from_outside(&inst, &kept);
    let r = inst.call(CallTarget::Project, "Press", vec![]).unwrap();
    assert_eq!(r.output, "handler 3\nhandler 4\npressed=False\n");
}
