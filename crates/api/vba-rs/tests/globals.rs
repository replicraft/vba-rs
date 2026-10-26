//! Module-level state and project metadata.

mod common;

use common::*;
use vba::runtime::Error as RunError;
use vba::{Error, ModuleKind, ProcKind, Variant};

const CONFIG: &str = "\
Public Const Limit As Long = 10
Public Const Greeting = \"hi\"
Private Const Secret As Long = 1
Public Counter As Integer
Public Title As String
Public Names() As String
Public Fixed(1 To 3) As Long
Public Owner As Object
Private Hidden As Long
Public Type Pt
  X As Long
End Type
Public Origin As Pt
Public Sub Bump()
  Counter = Counter + 1
End Sub
";

const OTHER: &str = "Public Counter As Long\nPublic Only As Long\n";

fn inst() -> vba::Instance {
    load(&[
        ("Config", ModuleKind::Standard, CONFIG),
        ("Other", ModuleKind::Standard, OTHER),
        (
            "Thing",
            ModuleKind::Class,
            "Public Size As Long\nPublic Sub Grow()\nEnd Sub\n",
        ),
    ])
}

fn not_accessible(e: &Error) -> bool {
    matches!(e, Error::Run(RunError::NotAccessible(..)))
}

#[test]
fn reads_variables_and_constants_qualified_and_unqualified() {
    let mut inst = inst();
    inst.call(vba::CallTarget::Project, "Bump", vec![]).unwrap();
    assert_eq!(
        inst.get_global("Config.Counter").unwrap(),
        Variant::Integer(1)
    );
    assert_eq!(inst.get_global("config.limit").unwrap(), Variant::Long(10));
    assert_eq!(inst.get_global("Limit").unwrap(), Variant::Long(10));
    assert_eq!(inst.get_global("Greeting").unwrap(), Variant::from("hi"));
    assert_eq!(inst.get_global("Only").unwrap(), Variant::Long(0));
    assert_eq!(inst.get_global("Title").unwrap(), Variant::from(""));
}

#[test]
fn unqualified_names_must_be_unambiguous() {
    let mut inst = inst();
    let err = inst.get_global("Counter").unwrap_err();
    assert!(
        matches!(err, Error::Run(RunError::AmbiguousMember(_))),
        "{err:?}"
    );
    assert_eq!(inst.get_global("Other.Counter").unwrap(), Variant::Long(0));
    let err = inst.set_global("Counter", Variant::Long(1)).unwrap_err();
    assert!(
        matches!(err, Error::Run(RunError::AmbiguousMember(_))),
        "{err:?}"
    );
}

#[test]
fn unknown_private_and_unsupported_names_are_clear_errors() {
    let mut inst = inst();
    for name in ["Nope", "Config.Nope", "Nope.X", "Bump", "Secret", "Hidden"] {
        let err = inst.get_global(name).unwrap_err();
        assert!(
            matches!(
                err,
                Error::Run(RunError::NoSuchMember(_) | RunError::NotAccessible(..))
            ),
            "{name}: {err:?}"
        );
    }
    // Qualified private access says why.
    let err = inst.get_global("Config.Hidden").unwrap_err();
    assert!(not_accessible(&err), "{err:?}");
    assert!(err.to_string().contains("private"), "{err}");
    let err = inst.get_global("Config.Secret").unwrap_err();
    assert!(not_accessible(&err) || matches!(err, Error::Run(RunError::NoSuchMember(_))));
    // A user-defined type has no host representation.
    assert!(not_accessible(&inst.get_global("Origin").unwrap_err()));
    // A class's variable is a field of an instance.
    let err = inst.get_global("Thing.Size").unwrap_err();
    assert!(not_accessible(&err), "{err:?}");
    assert!(err.to_string().contains("get_member"), "{err}");
}

#[test]
fn assignment_converts_to_the_declared_type() {
    let mut inst = inst();
    inst.set_global("Config.Counter", Variant::Long(41))
        .unwrap();
    assert_eq!(
        inst.get_global("Config.Counter").unwrap(),
        Variant::Integer(41)
    );
    inst.set_global("Title", Variant::Long(7)).unwrap();
    assert_eq!(inst.get_global("Title").unwrap(), Variant::from("7"));
    inst.set_global("Config.Counter", Variant::from("12"))
        .unwrap();
    assert_eq!(
        inst.get_global("Config.Counter").unwrap(),
        Variant::Integer(12)
    );
    // Overflow and type mismatch are VBA errors, and leave the value alone.
    let err = inst
        .set_global("Config.Counter", Variant::Long(70_000))
        .unwrap_err();
    assert_eq!(vba_number(&err), 6);
    let err = inst
        .set_global("Config.Counter", Variant::from("abc"))
        .unwrap_err();
    assert_eq!(vba_number(&err), 13);
    assert_eq!(
        inst.get_global("Config.Counter").unwrap(),
        Variant::Integer(12)
    );
    // VBA code sees what the host wrote.
    inst.call(vba::CallTarget::Project, "Bump", vec![]).unwrap();
    assert_eq!(
        inst.get_global("Config.Counter").unwrap(),
        Variant::Integer(13)
    );
}

#[test]
fn constants_fixed_arrays_and_udts_cannot_be_assigned() {
    let mut inst = inst();
    assert!(not_accessible(
        &inst.set_global("Limit", Variant::Long(1)).unwrap_err()
    ));
    assert!(not_accessible(
        &inst.set_global("Fixed", Variant::Long(1)).unwrap_err()
    ));
    assert!(not_accessible(
        &inst.set_global("Origin", Variant::Long(1)).unwrap_err()
    ));
    assert!(not_accessible(
        &inst
            .set_global("Config.Hidden", Variant::Long(1))
            .unwrap_err()
    ));
}

#[test]
fn objects_and_arrays_are_variables_too() {
    let mut inst = inst();
    let thing = inst.create_object("Thing", None).unwrap();
    inst.set_global("Owner", Variant::Object(thing)).unwrap();
    // VBA code can use it.
    let got = inst.get_global("Owner").unwrap();
    assert_eq!(got, Variant::Object(thing));
    inst.release(&got);
    // A non-object for an object variable is a type mismatch.
    assert_eq!(
        vba_number(&inst.set_global("Owner", Variant::Long(1)).unwrap_err()),
        13
    );
    inst.destroy_object(thing).unwrap();
    // A dynamic array variable accepts an array.
    let mut a = vba::VbaArray::new_dynamic(vba::VarType::String);
    a.redim(&[vba::raw::value::Bounds::new(0, 1)], false)
        .unwrap();
    inst.set_global("Names", Variant::Array(Box::new(a)))
        .unwrap();
    assert!(matches!(
        inst.get_global("Names").unwrap(),
        Variant::Array(_)
    ));
}

#[test]
fn metadata_lists_modules_procedures_and_variables() {
    let inst = load(&[
        ("Config", ModuleKind::Standard, CONFIG),
        (
            "Form_Orders",
            ModuleKind::Document,
            "Public Total As Currency\nPrivate Sub Form_Current()\nEnd Sub\nPrivate Sub Form_BeforeUpdate(Cancel As Integer)\nEnd Sub\nPublic Function Find(ByVal id As Long, Optional note As String = \"x\", ParamArray rest()) As Variant\nEnd Function\n",
        ),
        (
            "Form_Customers",
            ModuleKind::Document,
            "Private Sub Form_Current()\nEnd Sub\n",
        ),
        ("Thing", ModuleKind::Class, "Public Sub Grow()\nEnd Sub\n"),
    ]);
    let all = inst.modules();
    let orders = all.iter().find(|m| m.name == "Form_Orders").unwrap();
    assert_eq!(orders.kind, ModuleKind::Document);
    assert!(orders.class_like && orders.predeclared);
    let cur = orders.procedure("form_current").unwrap();
    assert!(!cur.public);
    assert_eq!(cur.kind, ProcKind::Sub);
    let before = orders.procedure("Form_BeforeUpdate").unwrap();
    assert_eq!(before.params.len(), 1);
    assert_eq!(before.params[0].name, "Cancel");
    assert_eq!(before.params[0].type_name, "Integer");
    assert!(!before.params[0].by_val);
    let find = orders.procedure("Find").unwrap();
    assert!(find.public);
    assert_eq!(find.return_type.as_deref(), Some("Variant"));
    assert_eq!(find.params.len(), 3);
    assert!(find.params[0].by_val && !find.params[0].optional);
    assert!(find.params[1].optional && find.params[1].has_default);
    assert!(find.params[2].param_array);
    assert_eq!(orders.variables.len(), 1);
    assert_eq!(orders.variables[0].type_name, "Currency");

    let config = all.iter().find(|m| m.name == "Config").unwrap();
    assert!(!config.class_like);
    let limit = config.variables.iter().find(|v| v.name == "Limit").unwrap();
    assert!(limit.constant && limit.public);
    let secret = config
        .variables
        .iter()
        .find(|v| v.name == "Secret")
        .unwrap();
    assert!(!secret.public);

    // Which forms define Form_Current: only class-like modules count.
    let mut who = inst.modules_defining("form_current");
    who.sort();
    assert_eq!(who, vec!["Form_Customers", "Form_Orders"]);
    assert!(inst.modules_defining("Bump").is_empty()); // standard module

    // The public view hides the private handlers and variables.
    let public = inst.list_public_members();
    let orders = public.iter().find(|m| m.name == "Form_Orders").unwrap();
    assert!(orders.procedure("Form_Current").is_none());
    assert!(orders.procedure("Find").is_some());
    let config = public.iter().find(|m| m.name == "Config").unwrap();
    assert!(config.variables.iter().all(|v| v.public));
    assert!(config.variables.iter().all(|v| v.name != "Secret"));
}
