' INV: stmt.type, type.udt
' SOURCE: spec ms-vbal 5.2.3.3 User Defined Type declarations, 2.3 default values
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
