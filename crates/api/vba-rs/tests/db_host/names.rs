//! Names with spaces, bracketed identifiers, `Select Case` on controls.

use crate::host::*;
use vba::{CallTarget, ModuleKind, Variant};

#[test]
fn bracketed_names_with_spaces_work_for_forms_controls_and_fields() {
    let db = Db::new();
    db.add_table(
        "Order Lines",
        &["Unit Price", "Product"],
        vec![vec![Variant::Long(5), Variant::from("nut")]],
    );
    db.add_form(
        "Order Form",
        Some("Order Lines"),
        &[
            ("Customer Name", Variant::from("Ann")),
            ("Unit Price", Variant::Null),
            ("Product", Variant::Null),
        ],
    );
    let form = "\
Option Explicit
Private Sub Form_Load()
  [Customer Name] = [Customer Name] & \"!\"
  Debug.Print Me![Customer Name]; Me![Unit Price]; [Product]
  Select Case [Product]
    Case \"nut\": Debug.Print \"is nut\"
  End Select
End Sub
";
    let main = "\
Option Explicit
Public Sub Main()
  DoCmd.OpenForm \"Order Form\"
  Debug.Print Forms![Order Form]![Customer Name]; Forms(\"Order Form\").[Unit Price]
  Dim rs As Object
  Set rs = CurrentDb.OpenRecordset(\"Order Lines\")
  Debug.Print rs![Unit Price] * 2; DLookup(\"[Unit Price]\", \"Order Lines\", \"[Product] = 'nut'\")
End Sub
";
    let mut inst = load(
        &db,
        &[
            ("Module1", ModuleKind::Standard, main),
            ("Form_Order Form", ModuleKind::Document, form),
        ],
    );
    let out = inst
        .call(CallTarget::Project, "Main", vec![])
        .unwrap_or_else(|e| panic!("{e}"))
        .output;
    assert_eq!(out, "Ann! 5 nut\nis nut\nAnn! 5 \n 10  5 \n");
}
