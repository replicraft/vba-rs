//! Recordsets, the database object, constants, domain functions, TempVars.

use crate::{orders_db, run};
use vba::Variant;

#[test]
fn recordset_loop_fields_bang_and_default_member() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Public Sub Main()
  Dim rs As Object
  Set rs = CurrentDb.OpenRecordset(\"Orders\", dbOpenDynaset)
  Debug.Print rs.RecordCount
  Do While Not rs.EOF
    Debug.Print rs!ID; rs!Customer; rs.Fields(\"Qty\").Value; rs(\"Customer\")
    rs.MoveNext
  Loop
  rs.Close
End Sub
",
    );
    // Numbers print with a leading sign space and a trailing space; Null
    // prints as the word Null.
    assert_eq!(out, " 3 \n 1 Ann 10 Ann\n 2 Bob 20 Bob\n 3 CyNullCy\n");
}

#[test]
fn addnew_edit_update_write_through_the_default_member_chain() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Public Sub Main()
  Dim rs As Object
  Set rs = CurrentDb.OpenRecordset(\"SELECT * FROM Orders\")
  rs.AddNew
  rs!ID = 4
  rs!Customer = \"Dee\"
  rs.Fields(\"Qty\") = 5
  rs.Update
  Debug.Print rs!Customer; rs.RecordCount
  rs.MoveFirst
  rs.Edit
  rs!Qty = rs!Qty + 1
  rs.Update
  Debug.Print rs!Qty
  On Error Resume Next
  rs!Qty = 99
  Debug.Print Err.Number; Err.Description; \"|\"; Err.Source
  Err.Clear
  rs.Update
  Debug.Print Err.Number
End Sub
",
    );
    assert_eq!(
        out,
        "Dee 4 \n 11 \n 3188 Couldn't update; currently locked.|DAO.Recordset\n 3020 \n"
    );
    let rows = db.rows("orders");
    assert_eq!(rows[3][1], Variant::from("Dee"));
    assert_eq!(rows[3][2], Variant::Integer(5)); // the literal 5 is an Integer
    assert_eq!(rows[0][2], Variant::Long(11));
}

#[test]
fn open_recordset_with_named_optional_arguments_and_host_errors() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Public Sub Main()
  Dim rs As Object
  Set rs = CurrentDb.OpenRecordset(Name:=\"Orders\", Options:=dbFailOnError)
  Debug.Print rs.Name; rs.RecordCount
  On Error GoTo Fail
  Set rs = CurrentDb.OpenRecordset(\"NoSuch\", dbOpenSnapshot)
  Exit Sub
Fail:
  Debug.Print Err.Number & \"|\" & Err.Source & \"|\" & Left$(Err.Description, 22)
  CurrentDb.Execute \"DELETE FROM Orders\", dbFailOnError
  Debug.Print CurrentDb.OpenRecordset(\"Orders\").RecordCount; CurrentDb.Name
End Sub
",
    );
    assert_eq!(
        out,
        "orders 3 \n3078|DAO.Database|Cannot find the input \n 0 current.db\n"
    );
}

#[test]
fn constants_are_real_constants_in_const_case_and_array_bounds() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Const Wanted As Long = dbOpenDynaset + 1
Public Sub Main()
  Dim a(dbOpenTable To dbOpenSnapshot) As Long
  Dim v As Long
  v = 2
  Select Case v
    Case dbOpenTable: Debug.Print \"table\"
    Case dbOpenDynaset: Debug.Print \"dynaset\"
  End Select
  Debug.Print Wanted; UBound(a); acViewPreview; vbYes
End Sub
",
    );
    assert_eq!(out, "dynaset\n 3  4  2  6 \n");
}

#[test]
fn domain_functions_use_optional_parameters_and_null_semantics() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Option Explicit
Public Sub Main()
  Dim q As Variant
  Debug.Print DLookup(\"Customer\", \"Orders\", \"ID = 2\")
  Debug.Print DLookup(\"Customer\", \"Orders\", \"[Customer] = 'Cy'\")
  q = DLookup(\"Qty\", \"Orders\", \"ID = 3\")
  Debug.Print IsNull(q); Nz(q); Nz(q, -1); TypeName(q)
  q = DLookup(\"Qty\", \"Orders\", \"ID = 99\")
  Debug.Print IsNull(q); TypeName(q)
  Debug.Print DCount(\"*\", \"Orders\"); DCount(\"*\", \"Orders\", \"Qty > 10\")
  Debug.Print \"[\" & Nz(Null) & \"]\"; Eval(\"1+2+3\"); Nz(Null, \"x\") & \"y\"
  Debug.Print Nz(Value:=Null, ValueIfNull:=7)
End Sub
",
    );
    assert_eq!(
        out,
        "Bob\nCy\nTrue-1 Null\nTrueNull\n 3  1 \n[] 6 xy\n 7 \n"
    );
}

#[test]
fn a_null_in_arithmetic_and_a_host_error_in_a_domain_function() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Public Sub Main()
  Dim n As Variant
  n = DLookup(\"Qty\", \"Orders\", \"ID = 3\")
  Debug.Print IsNull(n + 1); IsNull(n & \"\")
  On Error Resume Next
  Dim x As Long
  x = DLookup(\"Qty\", \"Orders\", \"ID = 3\")
  Debug.Print Err.Number; Err.Description
  Err.Clear
  x = DCount(\"*\", \"Missing\")
  Debug.Print Err.Number; Err.Source
End Sub
",
    );
    // Assigning Null to a Long is error 94, "Invalid use of Null".
    assert_eq!(out, "TrueFalse\n 94 Invalid use of Null\n 2001 Domain\n");
}

#[test]
fn tempvars_support_bang_call_syntax_default_value_and_remove() {
    let db = orders_db();
    let out = run(
        &db,
        "\
Public Sub Main()
  TempVars.Add \"user\", \"ann\"
  TempVars.Add \"level\", 3
  TempVars(\"level\") = TempVars(\"level\") + 1
  Debug.Print TempVars!user; TempVars(\"level\"); TempVars.Count
  TempVars.Remove \"user\"
  Debug.Print TempVars.Count; TempVars.Item(\"level\").Value
  TempVars.RemoveAll
  Debug.Print TempVars.Count
End Sub
",
    );
    assert_eq!(out, "ann 4  2 \n 1  4 \n 0 \n");
}
