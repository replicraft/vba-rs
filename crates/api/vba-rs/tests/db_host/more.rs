//! Coercion through default members, `Controls`, multi-argument events.

use crate::forms::*;
use crate::{orders_db, run};
use vba::{Arg, ModuleKind, Variant};

#[test]
fn default_member_objects_coerce_inside_builtins_with_blocks_and_comparisons() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Compare Database
Option Explicit
Public Sub Main()
  Dim rs As Object
  Set rs = CurrentDb.OpenRecordset(\"Orders\")
  With rs
    Do While Not .EOF
      If IsNull(!Qty) Then
        Debug.Print !Customer & \": null\"
      Else
        Debug.Print !Customer & \": \" & !Qty * 2 & \" \" & Format(!Qty, \"000\") & \" \" & CStr(!ID) & \" \" & Len(!Customer)
      End If
      .MoveNext
    Loop
  End With
  rs.MoveFirst
  If rs!Qty > 5 And rs!Customer = \"Ann\" Then Debug.Print \"ok\"
  Debug.Print TypeName(rs!Qty); TypeName(rs!Customer); VarType(rs!Qty)
  rs.MoveLast
  Debug.Print TypeName(rs!Qty); IsNull(rs!Qty)
End Sub
",
    );
    // IsNull and VarType look at the default member (the field's value);
    // TypeName names the object ("Field"). (unverified against a real host:
    // the TypeName behavior for objects with a default member.)
    assert_eq!(
        out,
        "Ann: 20 010 1 3\nBob: 40 020 2 3\nCy: null\nok\nFieldField 3 \nFieldTrue\n"
    );
}

#[test]
fn controls_collection_counts_enumerates_and_resolves_types() {
    let db = forms_db();
    let probe = "\
Option Explicit
Public Sub Main()
  Dim c As Object, s As String
  Dim f As Object
  Set f = Forms!Orders
  For Each c In f.Controls
    s = s & c.Name & \":\" & TypeName(c) & \",\"
  Next
  Debug.Print s
  Debug.Print f.Controls.Count; f.Controls(\"Customer\").Name; f.Controls(2).Name; f.Controls.Item(\"qty\").Value
  Debug.Print f.Controls(\"Customer\") Is f.Customer
  On Error Resume Next
  Debug.Print f.Controls(\"nope\").Name
  Debug.Print Err.Number
End Sub
";
    let mut inst = forms_instance_with(&db, &[("Probe", ModuleKind::Standard, probe)]);
    call(&mut inst, "OpenOrders");
    let out = call(&mut inst, "Main");
    assert_eq!(
        out,
        "ID:TextBox,Customer:TextBox,Qty:TextBox,cmdSave:TextBox,lblInfo:TextBox,\n\
         \x205 CustomerQty 10 \n\
         True\n\
         \x202465 \n"
    );
}

#[test]
fn an_event_with_several_arguments_writes_only_the_byref_ones_back() {
    let db = forms_db();
    let src = "\
Option Explicit
Private Sub Form_Error(ByVal DataErr As Integer, Response As Integer)
  Debug.Print \"error \" & DataErr
  If DataErr = 2169 Then Response = 1
End Sub
Private Sub Form_KeyDown(KeyCode As Integer, ByVal Shift As Integer)
  KeyCode = 0
  Shift = 99
End Sub
";
    let mut inst = crate::host::load(
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
    call(&mut inst, "OpenOrders");
    let obj = form(&db, "Orders").object.get().unwrap();
    let out = inst
        .raise_event_on(
            obj,
            "Form",
            "Error",
            vec![Arg::val(2169), Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert_eq!(out.output, "error 2169\n");
    assert_eq!(out.byref, vec![Variant::Integer(1)]);

    // KeyCode is ByRef and comes back; the ByVal Shift is reported unchanged.
    let out = inst
        .raise_event_on(
            obj,
            "Form",
            "KeyDown",
            vec![
                Arg::by_ref(Variant::Integer(65)),
                Arg::by_ref(Variant::Integer(4)),
            ],
        )
        .unwrap();
    assert_eq!(out.byref, vec![Variant::Integer(0), Variant::Integer(4)]);
}
