//! Events raised by the host into form modules, `Cancel` write-back, UI
//! hooks, `Screen`, reports and closing.

use crate::forms::*;
use crate::host::*;
use vba::{ModuleKind, Variant};

#[test]
fn a_control_before_update_can_cancel_and_after_update_runs_on_success() {
    let db = forms_db();
    let mut inst = forms_instance(&db);
    call(&mut inst, "OpenOrders");
    let st = form(&db, "Orders");

    // The handler sees the pending value and cancels it: the host restores.
    assert!(!user_edit(&mut inst, &st, "Customer", Variant::from("bad")).unwrap());
    assert_eq!(st.value("Customer"), Variant::from("Ann"));
    assert_eq!(st.props.borrow()["dirty"], Variant::Boolean(false));

    // Accepted: AfterUpdate (a handler with a bare control name) sets a caption.
    assert!(user_edit(&mut inst, &st, "Customer", Variant::from("Zed")).unwrap());
    assert_eq!(st.value("Customer"), Variant::from("Zed"));
    assert_eq!(
        st.ctl("lblInfo").unwrap().1.caption.borrow().as_str(),
        "Hello Zed"
    );
    assert_eq!(st.props.borrow()["dirty"], Variant::Boolean(true));
}

#[test]
fn form_before_update_cancel_blocks_the_save_and_msgbox_goes_through_the_ui_hook() {
    let db = forms_db();
    let mut inst = forms_instance(&db);
    call(&mut inst, "OpenOrders");
    let st = form(&db, "Orders");
    let before = inst.output().len();

    // Too short a name: the handler shows a message and cancels.
    user_edit(&mut inst, &st, "Customer", Variant::from("X")).unwrap();
    assert!(!save_record(&mut inst, &st).unwrap());
    assert_eq!(
        db.messages.borrow().as_slice(),
        ["Orders|Customer required|48"] // vbExclamation = 48
    );
    assert_eq!(db.rows("orders")[0][1], Variant::from("Ann"));
    assert_eq!(db.log().last().unwrap(), "save Orders cancelled");
    assert!(inst.output()[before..].contains("BeforeUpdate cancel=-1"));

    // A valid one commits to the store and runs AfterUpdate.
    user_edit(&mut inst, &st, "Customer", Variant::from("Xavier")).unwrap();
    assert!(save_record(&mut inst, &st).unwrap());
    assert_eq!(db.rows("orders")[0][1], Variant::from("Xavier"));
    assert!(
        inst.output()
            .ends_with("BeforeUpdate cancel=0\nsaved Xavier\n")
    );
    assert_eq!(st.props.borrow()["dirty"], Variant::Boolean(false));
}

#[test]
fn a_click_handler_named_after_a_control_runs_with_the_bare_control_names() {
    let db = forms_db();
    let mut inst = forms_instance(&db);
    call(&mut inst, "OpenOrders");
    let st = form(&db, "Orders");
    st.props
        .borrow_mut()
        .insert("dirty".into(), Variant::Boolean(true));
    let obj = st.object.get().unwrap();
    let out = inst
        .raise_event_on(obj, "cmdSave", "Click", vec![])
        .unwrap();
    assert!(out.handled);
    assert_eq!(out.output, "click dirty=True\n");
    assert_eq!(st.props.borrow()["dirty"], Variant::Boolean(false));
    // No handler for this event: not an error, not handled.
    let none = inst
        .raise_event_on(obj, "lblInfo", "Click", vec![])
        .unwrap();
    assert!(!none.handled);
}

#[test]
fn screen_active_form_reports_and_close_with_unload_close_and_terminate() {
    let db = forms_db();
    let probe = "\
Option Explicit
Public Sub Main()
  On Error Resume Next
  Debug.Print Screen.ActiveForm.Name
  Debug.Print Err.Number; Err.Source
  Err.Clear
  OpenOrders
  Debug.Print Screen.ActiveForm.Name; Screen.ActiveForm.Caption
  OpenSummary
  Debug.Print Reports.Count; Reports!Summary.Name; Forms.Count
  DoCmd.Close acForm, \"Orders\"
  Debug.Print Forms.Count; Reports.Count
  DoCmd.Close acForm, \"Orders\"
  Debug.Print Err.Number
  DoCmd.Close acReport, \"Summary\"
  Debug.Print Reports.Count
End Sub
";
    let mut inst = forms_instance_with(&db, &[("Probe", ModuleKind::Standard, probe)]);
    let out = call(&mut inst, "Main");
    assert_eq!(
        out,
        "\x202475 Screen\n\
         open args=-\nload Orders rows=3\ncurrent Ann/10/1/Ann\n\
         OrdersOrders\n\
         report open\nreport load 30\n\
         \x201 Summary 1 \n\
         unload\nclose loads=1\nterminate\n\
         \x200  1 \n\
         \x202489 \n\
         \x200 \n"
    );
}
