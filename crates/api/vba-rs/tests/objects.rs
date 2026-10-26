//! Instances of document and class modules with a host base object.

mod common;

use common::*;
use vba::{Arg, ModuleKind, Variant};

const FORM: &str = "\
Option Explicit
Private Clicks As Long
Public Tag As String

Private Sub Class_Initialize()
  Debug.Print \"init\"
End Sub

Private Sub Class_Terminate()
  Debug.Print \"term\" & Clicks
End Sub

Public Sub Bump()
  Clicks = Clicks + 1
End Sub

Public Function Count() As Long
  Count = Clicks
End Function

Public Property Get Doubled() As Long
  Doubled = Clicks * 2
End Property

Private Sub Form_BeforeUpdate(Cancel As Integer)
  If IsNull(Me.CustomerName) Then Cancel = True
  If Me!CustomerName = \"\" Then Cancel = True
End Sub

Private Sub Form_Current()
  Debug.Print \"current \" & Me.CustomerName
End Sub

Private Sub Command1_Click()
  Me.Caption = \"clicked\"
  Bump
End Sub

Public Sub Touch()
  Me.Caption = \"Edited\"
  CustomerName = \"Bob\"
  Me.Controls(\"City\").Value = \"Paris\"
  Me(\"Zip\") = \"75000\"
  Debug.Print Me!CustomerName & \"/\" & Me.Controls(\"City\") & \"/\" & Me(\"Zip\")
  If Me.Dirty Then Debug.Print \"dirty\"
  Me.Requery
End Sub
";

fn state() -> std::rc::Rc<FormState> {
    FormState::new(&[
        ("CustomerName", Variant::from("Ann")),
        ("Caption", Variant::from("Orders")),
        ("Dirty", Variant::Boolean(true)),
        ("City", Variant::from("Rome")),
        ("Zip", Variant::from("00100")),
    ])
}

#[test]
fn two_instances_have_separate_module_state_and_initializers() {
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let a = inst.create_object("Form_Orders", None).unwrap();
    let b = inst.create_object("Form_Orders", None).unwrap();
    assert_ne!(a, b);
    assert_eq!(inst.output(), "init\ninit\n");
    inst.call_on(a, "Bump", vec![]).unwrap();
    inst.call_on(a, "Bump", vec![]).unwrap();
    inst.call_on(b, "Bump", vec![]).unwrap();
    assert_eq!(
        inst.call_on(a, "Count", vec![]).unwrap().value,
        Variant::Long(2)
    );
    assert_eq!(
        inst.call_on(b, "Count", vec![]).unwrap().value,
        Variant::Long(1)
    );
    assert_eq!(
        inst.call_on(a, "Doubled", vec![]).unwrap().value,
        Variant::Long(4)
    );
    // Public variables are per instance too.
    let a_tag = inst.get_member(a, "Tag").unwrap();
    assert_eq!(a_tag, Variant::from(""));
    inst.set_member(a, "Tag", Variant::from("A")).unwrap();
    assert_eq!(inst.get_member(a, "Tag").unwrap(), Variant::from("A"));
    assert_eq!(inst.get_member(b, "Tag").unwrap(), Variant::from(""));
}

#[test]
fn destroy_object_runs_class_terminate_deterministically() {
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let a = inst.create_object("Form_Orders", None).unwrap();
    let b = inst.create_object("Form_Orders", None).unwrap();
    inst.call_on(a, "Bump", vec![]).unwrap();
    let before = inst.output().len();
    inst.destroy_object(a).unwrap();
    assert_eq!(&inst.output()[before..], "term1\n");
    // The other instance is untouched, and a destroyed one is gone.
    assert_eq!(
        inst.call_on(b, "Count", vec![]).unwrap().value,
        Variant::Long(0)
    );
    let err = inst.call_on(a, "Count", vec![]).unwrap_err();
    assert_eq!(vba_number(&err), 91);
    assert_eq!(vba_number(&inst.destroy_object(a).unwrap_err()), 91);
}

#[test]
fn members_the_module_lacks_reach_the_host_object() {
    let st = state();
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let form = inst
        .create_object("Form_Orders", Some(FormHost::new(&st)))
        .unwrap();
    inst.call_on(form, "Touch", vec![]).unwrap();
    assert_eq!(st.get("Caption"), Variant::from("Edited"));
    assert_eq!(st.get("CustomerName"), Variant::from("Bob"));
    assert_eq!(st.get("City"), Variant::from("Paris"));
    assert_eq!(st.get("Zip"), Variant::from("75000"));
    assert!(
        inst.output().ends_with("Bob/Paris/75000\ndirty\n"),
        "{}",
        inst.output()
    );
    assert!(
        st.log().contains(&"call Requery".to_owned()),
        "{:?}",
        st.log()
    );
    assert!(st.log().contains(&"control put city".to_owned()));
}

#[test]
fn each_instance_reaches_its_own_host_object() {
    let (s1, s2) = (state(), state());
    s2.props
        .borrow_mut()
        .insert("customername".into(), Variant::from("Zed"));
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let f1 = inst
        .create_object("Form_Orders", Some(FormHost::new(&s1)))
        .unwrap();
    let f2 = inst
        .create_object("Form_Orders", Some(FormHost::new(&s2)))
        .unwrap();
    let out = inst.raise_event_on(f1, "Form", "Current", vec![]).unwrap();
    assert!(out.handled);
    assert_eq!(out.output, "current Ann\n");
    let out = inst.raise_event_on(f2, "Form", "Current", vec![]).unwrap();
    assert_eq!(out.output, "current Zed\n");
}

#[test]
fn a_member_without_host_object_is_error_438() {
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let form = inst.create_object("Form_Orders", None).unwrap();
    let err = inst.call_on(form, "Touch", vec![]).unwrap_err();
    assert_eq!(vba_number(&err), 438);
    // A host object that lacks the member reports its own error.
    let st = FormState::new(&[("Caption", Variant::from("x"))]);
    let form = inst
        .create_object("Form_Orders", Some(FormHost::new(&st)))
        .unwrap();
    let err = inst.call_on(form, "Touch", vec![]).unwrap_err();
    assert!(matches!(err, vba::Error::Run(_)), "{err:?}");
}

#[test]
fn raise_event_on_writes_back_cancel() {
    let st = state();
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let form = inst
        .create_object("Form_Orders", Some(FormHost::new(&st)))
        .unwrap();
    let ok = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert!(ok.handled);
    assert_eq!(ok.byref, vec![Variant::Integer(0)]);

    st.props
        .borrow_mut()
        .insert("customername".into(), Variant::from(""));
    let cancelled = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert_eq!(cancelled.byref, vec![Variant::Integer(-1)]);

    st.props
        .borrow_mut()
        .insert("customername".into(), Variant::Null);
    let cancelled = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert_eq!(cancelled.byref, vec![Variant::Integer(-1)]);
}

#[test]
fn raise_event_on_without_a_handler_is_not_an_error() {
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let form = inst.create_object("Form_Orders", None).unwrap();
    let out = inst
        .raise_event_on(
            form,
            "Form",
            "Unload",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert!(!out.handled);
    assert_eq!(out.byref, vec![Variant::Integer(0)]);
    assert!(inst.has_procedure(form, "Form_Current"));
    assert!(inst.has_procedure(form, "command1_click"));
    assert!(!inst.has_procedure(form, "Form_Unload"));
}

#[test]
fn control_events_run_on_the_instance_they_are_raised_on() {
    let (s1, s2) = (state(), state());
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let f1 = inst
        .create_object("Form_Orders", Some(FormHost::new(&s1)))
        .unwrap();
    let f2 = inst
        .create_object("Form_Orders", Some(FormHost::new(&s2)))
        .unwrap();
    inst.raise_event_on(f2, "Command1", "Click", vec![])
        .unwrap();
    inst.raise_event_on(f2, "Command1", "Click", vec![])
        .unwrap();
    assert_eq!(s1.get("Caption"), Variant::from("Orders"));
    assert_eq!(s2.get("Caption"), Variant::from("clicked"));
    assert_eq!(
        inst.call_on(f1, "Count", vec![]).unwrap().value,
        Variant::Long(0)
    );
    assert_eq!(
        inst.call_on(f2, "Count", vec![]).unwrap().value,
        Variant::Long(2)
    );
}

#[test]
fn call_on_only_runs_public_procedures_but_events_may_be_private() {
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let form = inst.create_object("Form_Orders", None).unwrap();
    let err = inst.call_on(form, "Form_Current", vec![]).unwrap_err();
    assert!(matches!(
        err,
        vba::Error::Run(vba::RunError::NotCallable(..))
    ));
    let err = inst.call_on(form, "Nope", vec![]).unwrap_err();
    assert!(matches!(
        err,
        vba::Error::Run(vba::RunError::NoSuchProcedure(..))
    ));
}

#[test]
fn class_modules_get_instances_too_and_standard_modules_do_not() {
    let mut inst = load(&[
        (
            "Counter",
            ModuleKind::Class,
            "Private n As Long\nPublic Sub Inc()\n  n = n + 1\nEnd Sub\nPublic Function Value() As Long\n  Value = n\nEnd Function\n",
        ),
        (
            "Module1",
            ModuleKind::Standard,
            "Public Sub Main()\nEnd Sub\n",
        ),
    ]);
    let a = inst.create_object("Counter", None).unwrap();
    let b = inst.create_object("counter", None).unwrap();
    inst.call_on(a, "Inc", vec![]).unwrap();
    assert_eq!(
        inst.call_on(a, "Value", vec![]).unwrap().value,
        Variant::Long(1)
    );
    assert_eq!(
        inst.call_on(b, "Value", vec![]).unwrap().value,
        Variant::Long(0)
    );
    assert!(inst.create_object("Module1", None).is_err());
    assert!(inst.create_object("Nope", None).is_err());
}

#[test]
fn default_instances_remain_available_and_separate() {
    let st = state();
    let mut inst = load(&[("Form_Orders", ModuleKind::Document, FORM)]);
    let d = inst
        .default_instance("Form_Orders", Some(FormHost::new(&st)))
        .unwrap();
    let again = inst.default_instance("Form_Orders", None).unwrap();
    assert_eq!(d, again);
    let other = inst.create_object("Form_Orders", None).unwrap();
    assert_ne!(d, other);
    inst.call_on(d, "Bump", vec![]).unwrap();
    // `Form_Orders.Count` by name runs on the default instance.
    let r = inst
        .call(vba::CallTarget::Project, "Form_Orders.Count", vec![])
        .unwrap();
    assert_eq!(r.value, Variant::Long(1));
    assert_eq!(
        inst.call_on(other, "Count", vec![]).unwrap().value,
        Variant::Long(0)
    );
    // A default instance lives as long as the session.
    assert_eq!(vba_number(&inst.destroy_object(d).unwrap_err()), 5);
    // Binding a second host base is refused.
    assert_eq!(
        vba_number(
            &inst
                .default_instance("Form_Orders", Some(FormHost::new(&st)))
                .unwrap_err()
        ),
        457
    );
}

#[test]
fn only_listed_host_names_are_late_bound_under_option_explicit() {
    let mut p = vba::Project::new();
    p.add_module(
        "Form_Orders",
        ModuleKind::Document,
        "Option Explicit\nPublic Sub Go()\n  Typo = 1\nEnd Sub\n",
    )
    .unwrap();
    let err = vba::Engine::new()
        .host_members(["CustomerName"])
        .load(&p)
        .err()
        .unwrap();
    assert!(err.to_string().contains("Typo"), "{err}");
}

#[test]
fn an_instance_kept_by_vba_outlives_the_hosts_reference() {
    let mut inst = load(&[
        (
            "Keeper",
            ModuleKind::Class,
            "Public Held As Object\nPrivate Sub Class_Terminate()\n  Debug.Print \"keeper gone\"\nEnd Sub\n",
        ),
        (
            "Thing",
            ModuleKind::Class,
            "Private Sub Class_Terminate()\n  Debug.Print \"thing gone\"\nEnd Sub\n",
        ),
        (
            "Module1",
            ModuleKind::Standard,
            "Public Sub Hold(k As Object, t As Object)\n  Set k.Held = t\nEnd Sub\n",
        ),
    ]);
    let keeper = inst.create_object("Keeper", None).unwrap();
    let thing = inst.create_object("Thing", None).unwrap();
    inst.call(
        vba::CallTarget::Project,
        "Hold",
        vec![
            Arg::val(Variant::Object(keeper)),
            Arg::val(Variant::Object(thing)),
        ],
    )
    .unwrap();
    inst.destroy_object(thing).unwrap();
    assert_eq!(inst.output(), "", "VBA still holds the thing");
    inst.destroy_object(keeper).unwrap();
    assert_eq!(inst.output(), "keeper gone\nthing gone\n");
}
