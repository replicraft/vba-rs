//! Per-module host classes through the facade: the controls of one form are
//! checked, early bound and run against the class described for that form,
//! and a form whose design changed is analysed again without a new engine.
#![allow(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use vba::host_model::{
    CheckedLibrary, Handlers, HostClass, HostEvent, HostLibrary, HostMember, HostObject, HostParam,
};
use vba::{Arg, Engine, ModuleEdit, ModuleKind, ModuleScope, Project, Variant};
use vba_automation::ObjectTable;
use vba_value::ObjectRef;

type Cell = Rc<RefCell<Variant>>;

/// The description of the Orders form; `with_city` is a later design that
/// also has a `txtCity` control.
fn library(with_city: bool) -> CheckedLibrary {
    let mut form = HostClass::new("Form_Orders")
        .member(HostMember::property("txtName", "TextBox").read_only())
        .member(HostMember::property("Caption", "String"))
        .event(HostEvent::new("BeforeUpdate").param(HostParam::new("Cancel", "Integer").by_ref()))
        .event(HostEvent::new("Load"));
    if with_city {
        form = form.member(HostMember::property("txtCity", "TextBox").read_only());
    }
    HostLibrary::new("FormLib")
        .class(
            HostClass::new("TextBox")
                .member(HostMember::property("Value", "Variant").default_member())
                .event(HostEvent::new("AfterUpdate")),
        )
        .class(form)
        .check()
        .expect("valid")
}

fn textbox(lib: &CheckedLibrary, cell: Cell) -> HostObject {
    let c2 = cell.clone();
    HostObject::new(
        lib,
        "TextBox",
        Handlers::new(cell)
            .get("Value", |s, _| Ok(s.borrow().clone()))
            .let_("Value", |s, cx| {
                *s.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            })
            .set("Value", move |_, cx| {
                *c2.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            }),
    )
    .expect("TextBox binds")
}

fn handle(table: &ObjectTable, o: HostObject) -> Variant {
    Variant::Object(ObjectRef::Handle(table.insert_obj(o)))
}

/// The `Me` of the Orders form.
fn me(lib: &CheckedLibrary, with_city: bool) -> Rc<HostObject> {
    let (l1, l2) = (lib.clone(), lib.clone());
    let name: Cell = Rc::new(RefCell::new(Variant::from("ann")));
    let city: Cell = Rc::new(RefCell::new(Variant::from("oslo")));
    let caption: Cell = Rc::new(RefCell::new(Variant::from("Orders")));
    let (c1, c2) = (caption.clone(), caption);
    let mut h = Handlers::new(())
        .get("txtName", move |_, cx| {
            Ok(handle(cx.table, textbox(&l1, name.clone())))
        })
        .get("Caption", move |_, _| Ok(c1.borrow().clone()))
        .let_("Caption", move |_, cx| {
            *c2.borrow_mut() = cx.args.value()?.clone();
            Ok(())
        });
    if with_city {
        h = h.get("txtCity", move |_, cx| {
            Ok(handle(cx.table, textbox(&l2, city.clone())))
        });
    }
    Rc::new(HostObject::new(lib, "Form_Orders", h).expect("form binds"))
}

const ORDERS: &str = "\
Option Explicit
Public Function Greet() As String
  txtName.Value = UCase(txtName.Value)
  Me.Caption = \"Orders for \" & Me!txtName & \"/\" & Me(\"txtName\")
  Greet = Caption & \"|\" & txtName
End Function
Private Sub Form_BeforeUpdate(Cancel As Integer)
  If txtName = \"ANN\" Then Cancel = True
End Sub
";

fn project(orders: &str) -> Project {
    let mut p = Project::new();
    p.add_module("Form_Orders", ModuleKind::Document, orders)
        .unwrap();
    p
}

fn engine(with_city: bool) -> Engine {
    Engine::new().module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        library(with_city),
    ))
}

#[test]
fn a_form_runs_against_its_described_class() {
    let project = project(ORDERS);
    let engine = engine(false);
    let report = engine.analyze(&project);
    assert!(!report.has_errors(), "{:?}", report.rendered());
    // Typed access: nothing is left to late binding.
    assert!(
        !report.codes().contains(&"SEM005"),
        "{:?}",
        report.rendered()
    );

    let mut inst = engine.load(&project).unwrap();
    inst.trust();
    let lib = library(false);
    let form = inst
        .create_object("Form_Orders", Some(me(&lib, false)))
        .unwrap();
    let out = inst
        .call(vba::CallTarget::Object(form.clone()), "Greet", vec![])
        .unwrap();
    assert_eq!(out.value, Variant::from("Orders for ANN/ANN|ANN"));
    let ev = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert!(ev.handled);
    assert_eq!(ev.byref, vec![Variant::Integer(-1)]);
}

#[test]
fn a_name_the_form_does_not_have_is_an_error_naming_the_class() {
    let engine = engine(false);
    let p = project(
        "Option Explicit\nSub A()\ntxtCity.Value = 1\nMe.txtCity.Value = 2\nMe!txtCity = 3\nEnd Sub\n",
    );
    let r = engine.analyze(&p);
    assert_eq!(
        r.codes(),
        ["SEM002", "SEM019", "SEM019"],
        "{:?}",
        r.rendered()
    );
    assert!(
        r.rendered()[0].contains("FormLib.Form_Orders"),
        "{:?}",
        r.rendered()
    );
    assert!(engine.load(&p).is_err());

    // Without a scope the engine-wide list is the only knowledge.
    let legacy = Engine::new().host_members(["txtCity"]);
    assert!(!legacy.analyze(&p).has_errors());
}

#[test]
fn handlers_are_checked_against_the_described_events() {
    let engine = engine(false);
    let p = project(
        "Option Explicit\nPrivate Sub Form_BeforeUpdate(ByVal Cancel As Integer)\nEnd Sub\n\
         Private Sub txtName_AfterUpdate()\nEnd Sub\nPrivate Sub txtName_Change()\nEnd Sub\n",
    );
    let r = engine.analyze(&p);
    assert_eq!(r.codes(), ["SEM017", "SEM017"], "{:?}", r.rendered());
    assert_eq!(r.errors().count(), 1);
}

#[test]
fn a_design_change_is_a_re_analysis_not_a_new_engine() {
    let with_city =
        "Option Explicit\nPublic Function City() As String\n  City = txtCity.Value\nEnd Function\n";
    let mut engine = engine(false);
    let p = project(with_city);
    // The first design has no txtCity.
    assert!(engine.analyze(&p).has_errors());
    // The new design does.
    engine.set_module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        library(true),
    ));
    assert_eq!(engine.module_scopes().len(), 1);
    let r = engine.analyze(&p);
    assert!(!r.has_errors(), "{:?}", r.rendered());
    let mut inst = engine.load(&p).unwrap();
    inst.trust();
    let lib = library(true);
    let form = inst
        .create_object("Form_Orders", Some(me(&lib, true)))
        .unwrap();
    let v = inst
        .call(vba::CallTarget::Object(form), "City", vec![])
        .unwrap();
    assert_eq!(v.value, Variant::from("oslo"));
    // Without a scope the engine-wide rules apply again: the name is
    // assumed to be the host's and bound late.
    assert!(engine.remove_module_scope("form_orders"));
    let legacy = engine.analyze(&p);
    assert!(!legacy.has_errors() && legacy.codes().contains(&"SEM005"));
}

#[test]
fn a_running_instance_follows_a_design_change() {
    let engine = engine(false);
    let mut inst = engine.load(&project(ORDERS)).unwrap();
    inst.trust();
    let with_city =
        "Option Explicit\nPublic Function City() As String\n  City = txtCity.Value\nEnd Function\n";
    let edit = ModuleEdit::Replace {
        name: "Form_Orders".into(),
        source: with_city.into(),
    };
    // The old design refuses the new code and nothing changes.
    assert!(inst.apply_edits(std::slice::from_ref(&edit)).is_err());
    // The form gains a control: re-analyse the instance, then the edit fits.
    inst.set_module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        library(true),
    ))
    .unwrap();
    inst.apply_edits(&[edit]).unwrap();
    let lib = library(true);
    let form = inst
        .create_object("Form_Orders", Some(me(&lib, true)))
        .unwrap();
    let v = inst
        .call(vba::CallTarget::Object(form.clone()), "City", vec![])
        .unwrap();
    assert_eq!(v.value, Variant::from("oslo"));
    // The form loses the control again: the code that uses it is refused.
    let back = inst.set_module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        library(false),
    ));
    assert!(back.is_err());
    // ...and the instance keeps the design it had.
    let again = inst
        .call(vba::CallTarget::Object(form), "City", vec![])
        .unwrap();
    assert_eq!(again.value, Variant::from("oslo"));
}
