//! A form whose controls are described per module, checked, run, and
//! changed at design time without rebuilding the engine.
//!
//! ```text
//! cargo run -p vba-rs --example form_scopes
//! ```
//!
//! The embedding application describes each form as a class of its own
//! (`Form_Orders`: its controls, properties and events) and declares that class
//! as the `Me` of the module. Bare names, `Me.Ctl`, `Me!Ctl` and the
//! handlers (`txtName_AfterUpdate`, `Form_BeforeUpdate`) are then resolved and
//! type checked against that form only. When the designer adds a control the
//! application describes the form again and re-analyses; nothing else is
//! rebuilt.
//!
//! See `docs/architecture/host-model.md` and section 4 of
//! `docs/architecture/embedding-guide.md`.

use std::cell::RefCell;
use std::rc::Rc;

use vba::automation::{ObjectRef, ObjectTable};
use vba::host_model::{
    CheckedLibrary, Handlers, HostClass, HostEvent, HostLibrary, HostMember, HostObject, HostParam,
};
use vba::{Arg, CallTarget, Engine, ModuleKind, ModuleScope, Project, Variant};

type Cell = Rc<RefCell<Variant>>;

/// The description of the Orders form; the second design adds `txtCity`.
fn describe(with_city: bool) -> CheckedLibrary {
    let mut form = HostClass::new("Form_Orders")
        .member(HostMember::property("txtName", "TextBox").read_only())
        .member(HostMember::property("Caption", "String"))
        .event(HostEvent::new("BeforeUpdate").param(HostParam::new("Cancel", "Integer").by_ref()));
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
        .expect("a valid description")
}

fn text_box(lib: &CheckedLibrary, cell: Cell) -> HostObject {
    let again = cell.clone();
    HostObject::new(
        lib,
        "TextBox",
        Handlers::new(cell)
            .get("Value", |c, _| Ok(c.borrow().clone()))
            .let_("Value", |c, cx| {
                *c.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            })
            .set("Value", move |_, cx| {
                *again.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            }),
    )
    .expect("the handlers match the description")
}

/// The `Me` object of a form instance.
fn form_object(lib: &CheckedLibrary, with_city: bool) -> Rc<HostObject> {
    let (for_name, for_city) = (lib.clone(), lib.clone());
    let name: Cell = Rc::new(RefCell::new(Variant::from("ann")));
    let city: Cell = Rc::new(RefCell::new(Variant::from("oslo")));
    let caption: Cell = Rc::new(RefCell::new(Variant::from("Orders")));
    let (read, write) = (caption.clone(), caption);
    let mut handlers = Handlers::new(())
        .get("txtName", move |_, cx| {
            let b = text_box(&for_name, name.clone());
            Ok(Variant::Object(ObjectRef::Handle(ObjectTable::insert_obj(
                cx.table, b,
            ))))
        })
        .get("Caption", move |_, _| Ok(read.borrow().clone()))
        .let_("Caption", move |_, cx| {
            *write.borrow_mut() = cx.args.value()?.clone();
            Ok(())
        });
    if with_city {
        handlers = handlers.get("txtCity", move |_, cx| {
            let b = text_box(&for_city, city.clone());
            Ok(Variant::Object(ObjectRef::Handle(ObjectTable::insert_obj(
                cx.table, b,
            ))))
        });
    }
    Rc::new(HostObject::new(lib, "Form_Orders", handlers).expect("the form binds"))
}

const ORDERS: &str = "\
Option Explicit
Public Function Title() As String
  Me.Caption = \"Orders for \" & txtName & \"/\" & Me!txtName.Value
  Title = Caption
End Function
Private Sub Form_BeforeUpdate(Cancel As Integer)
  If txtName = \"ann\" Then Cancel = True
End Sub
";

fn main() {
    let mut project = Project::new();
    project
        .add_module("Form_Orders", ModuleKind::Document, ORDERS)
        .expect("module");

    // The form is declared once; the engine knows nothing else about it.
    let mut engine = Engine::new().module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        describe(false),
    ));
    let report = engine.analyze(&project);
    assert!(!report.has_errors(), "{:?}", report.rendered());

    let mut inst = engine.load(&project).expect("loads");
    inst.trust();
    let form = inst
        .create_object("Form_Orders", Some(form_object(&describe(false), false)))
        .expect("form");
    let title = inst
        .call(CallTarget::Object(form), "Title", vec![])
        .expect("runs");
    println!("{:?}", title.value);
    let raised = inst
        .raise_event_on(
            form,
            "Form",
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .expect("event");
    println!("Cancel after BeforeUpdate: {:?}", raised.byref[0]);

    // A control the form does not have is an error that names the class.
    let mut typo = Project::new();
    typo.add_module(
        "Form_Orders",
        ModuleKind::Document,
        "Option Explicit\nSub A()\n  txtCity.Value = 1\nEnd Sub\n",
    )
    .expect("module");
    for line in engine.analyze(&typo).rendered() {
        println!("{}", line.lines().next().unwrap_or_default());
    }

    // The designer adds txtCity: describe the form again and re-analyse.
    engine.set_module_scope(ModuleScope::new(
        "Form_Orders",
        "Form_Orders",
        describe(true),
    ));
    println!(
        "after the design change: {} errors",
        engine.analyze(&typo).errors().count()
    );
}
