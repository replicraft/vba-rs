//! `Forms`, `Reports`, `Screen`, control access and errors from forms.

use crate::forms::*;
use vba::{Arg, CallTarget, ModuleKind, Variant};

#[test]
fn open_args_and_a_cancelled_open_form_are_trappable_in_the_caller() {
    let db = forms_db();
    let mut inst = forms_instance(&db);
    let r = inst
        .call(CallTarget::Project, "OpenOrders", vec![Arg::val("deny")])
        .unwrap_err();
    let vba::Error::Run(run) = &r else { panic!() };
    let info = run.error_info().unwrap();
    assert_eq!(info.number, 2501);
    assert_eq!(info.source, "DoCmd");
    assert!(db.forms.borrow().is_empty());
    assert_eq!(db.log(), ["open Orders begin", "open Orders cancelled"]);

    // The same, trapped by the VBA caller; the instance stays usable.
    let db = forms_db();
    let mut inst = forms_instance_with(
        &db,
        &[(
            "Module2",
            ModuleKind::Standard,
            "Public Sub Try()\n On Error Resume Next\n OpenOrders \"deny\"\n Debug.Print Err.Number; Err.Source; Forms.Count\n OpenOrders \"x\"\n Debug.Print Forms.Count\nEnd Sub\n",
        )],
    );
    let out = call(&mut inst, "Try");
    assert!(out.contains(" 2501 DoCmd 0 \n"), "{out}");
    assert!(out.contains("open args=x\n"), "{out}");
    assert!(out.ends_with(" 1 \n"), "{out}");
}

const PROBE: &str = "\
Option Explicit
Public Sub Main()
  Dim f As Object, names As String
  For Each f In Forms
    names = names & f.Name & \",\"
  Next
  Debug.Print names; Forms.Count
  Debug.Print Forms!Orders.Caption; Forms(\"Details\").Caption; Forms(1).Name
  Debug.Print Forms!Orders!Customer; Forms!Orders.Customer.Value; Forms(\"orders\")!Qty
  Forms!Orders.Greet \"vba\"
  Debug.Print Forms!Orders.Doubled
  Forms!Orders!Qty = 7
  Forms!Orders.Qty.Visible = False
  Debug.Print Forms!Orders.Doubled; Forms!Orders.Qty.Visible
  On Error Resume Next
  Debug.Print Forms!Nope.Caption
  Debug.Print Err.Number; Err.Source
  Err.Clear
  Debug.Print Forms!Orders.NoSuchMember
  Debug.Print Err.Number
End Sub
";

#[test]
fn forms_collection_bang_call_for_each_count_and_vba_members_of_the_form() {
    let db = forms_db();
    let mut inst = forms_instance_with(&db, &[("Probe", ModuleKind::Standard, PROBE)]);
    call(&mut inst, "OpenOrders");
    call(&mut inst, "OpenDetails");
    let out = call(&mut inst, "Main");
    assert_eq!(
        out,
        "Orders,Details, 2 \n\
         OrdersDetailsDetails\n\
         AnnAnn 10 \n\
         hello vba from Orders\n\
         \x2020 \n\
         \x2014 False\n\
         \x202450 Forms\n\
         \x20438 \n"
    );
    assert_eq!(form(&db, "Orders").value("Qty"), Variant::Integer(7));
}
