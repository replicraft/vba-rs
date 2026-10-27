//! Forms, reports, controls and events.

use std::rc::Rc;

use crate::host::*;
use crate::orders_db;
use vba::{CallTarget, Instance, ModuleKind, Variant};

pub const FORM_ORDERS: &str = "\
Option Explicit
Private mLoads As Long

Public Sub Greet(ByVal who As String)
  Debug.Print \"hello \" & who & \" from \" & Me.Name
End Sub

Public Property Get Doubled() As Long
  Doubled = Qty * 2
End Property

Private Sub Form_Open(Cancel As Integer)
  Debug.Print \"open args=\" & Nz(Me.OpenArgs, \"-\")
  If Me.OpenArgs = \"deny\" Then Cancel = True
End Sub

Private Sub Form_Load()
  mLoads = mLoads + 1
  Debug.Print \"load \" & Me.Name & \" rows=\" & DCount(\"*\", Me.RecordSource)
End Sub

Private Sub Form_Current()
  Debug.Print \"current \" & Customer & \"/\" & Me!Qty & \"/\" & Me(\"ID\") & \"/\" & Me.Controls(\"Customer\").Value
  cmdSave.Enabled = Not IsNull(Customer)
End Sub

Private Sub Form_BeforeUpdate(Cancel As Integer)
  If Len(Nz(Customer, \"\")) < 2 Then
    MsgBox \"Customer required\", vbExclamation, \"Orders\"
    Cancel = True
  End If
  Debug.Print \"BeforeUpdate cancel=\" & Cancel
End Sub

Private Sub Form_AfterUpdate()
  Debug.Print \"saved \" & Customer
End Sub

Private Sub Customer_BeforeUpdate(Cancel As Integer)
  If Customer = \"bad\" Then Cancel = True
End Sub

Private Sub Customer_AfterUpdate()
  lblInfo.Caption = \"Hello \" & Customer
End Sub

Private Sub cmdSave_Click()
  Debug.Print \"click dirty=\" & Me.Dirty
  Me.Dirty = False
End Sub

Private Sub Form_Unload(Cancel As Integer)
  Debug.Print \"unload\"
End Sub

Private Sub Form_Close()
  Debug.Print \"close loads=\" & mLoads
End Sub

Private Sub Class_Terminate()
  Debug.Print \"terminate\"
End Sub
";

const FORM_DETAILS: &str = "\
Option Explicit
Private Sub Form_Load()
  Debug.Print \"details load \" & Me.Caption
End Sub
";

const REPORT_SUMMARY: &str = "\
Option Explicit
Private Sub Report_Open(Cancel As Integer)
  Debug.Print \"report open\"
End Sub
Private Sub Report_Load()
  Debug.Print \"report load \" & Total
End Sub
";

const MODULE1: &str = "\
Option Explicit
Public Sub OpenOrders(Optional args As Variant)
  DoCmd.OpenForm \"Orders\", acViewNormal, , , , , args
End Sub
Public Sub OpenDetails()
  DoCmd.OpenForm \"Details\"
End Sub
Public Sub OpenSummary()
  DoCmd.OpenReport \"Summary\", acViewPreview
End Sub
";

pub fn forms_db() -> Rc<Db> {
    let db = orders_db();
    db.add_form(
        "Orders",
        Some("Orders"),
        &[
            ("ID", Variant::Null),
            ("Customer", Variant::Null),
            ("Qty", Variant::Null),
            ("cmdSave", Variant::Null),
            ("lblInfo", Variant::Null),
        ],
    );
    db.add_form("Details", None, &[("txtNote", Variant::from("n/a"))]);
    db.add_form("Summary", None, &[("Total", Variant::Long(30))]);
    db
}

pub fn forms_instance(db: &Rc<Db>) -> Instance {
    forms_instance_with(db, &[])
}

/// The forms project plus extra modules (a probe, a class).
pub fn forms_instance_with(db: &Rc<Db>, extra: &[(&str, ModuleKind, &str)]) -> Instance {
    let mut mods = vec![
        ("Module1", ModuleKind::Standard, MODULE1),
        ("Form_Orders", ModuleKind::Document, FORM_ORDERS),
        ("Form_Details", ModuleKind::Document, FORM_DETAILS),
        ("Report_Summary", ModuleKind::Document, REPORT_SUMMARY),
    ];
    mods.extend_from_slice(extra);
    load(db, &mods)
}

pub fn call(inst: &mut Instance, name: &str) -> String {
    inst.call(CallTarget::Project, name, vec![])
        .unwrap_or_else(|e| panic!("{e}"))
        .output
}

pub fn form(db: &Rc<Db>, name: &str) -> Rc<FormState> {
    db.forms
        .borrow()
        .iter()
        .find(|f| f.name == name)
        .expect("open form")
        .clone()
}

#[test]
fn open_form_runs_open_load_current_in_order_and_binds_the_first_record() {
    let db = forms_db();
    let mut inst = forms_instance(&db);
    let out = call(&mut inst, "OpenOrders");
    assert_eq!(
        out,
        "open args=-\nload Orders rows=3\ncurrent Ann/10/1/Ann\n"
    );
    assert_eq!(db.log(), ["open Orders begin", "open Orders end"]);
    // Controls were bound; cmdSave.Enabled was set from VBA.
    let st = form(&db, "Orders");
    assert_eq!(st.value("Customer"), Variant::from("Ann"));
    assert!(st.ctl("cmdSave").unwrap().1.enabled.get());
}
