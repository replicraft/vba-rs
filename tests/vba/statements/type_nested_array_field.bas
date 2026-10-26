' INV: stmt.type, type.udt
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/type-statement
' EXPECT: n 5  0 
Type Rec
    Name As String
    Vals(1 To 3) As Long
End Type
Public Sub Main()
    Dim r As Rec
    r.Name = "n"
    r.Vals(2) = 5
    Debug.Print r.Name; r.Vals(2); r.Vals(1)
End Sub
