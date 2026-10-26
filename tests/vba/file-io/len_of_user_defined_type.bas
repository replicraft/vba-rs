' INV: func.len, type.udt
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len of a user-defined type (with 5.4.5.11 binary file data formats)
' EXPECT:  8 
Type Rec
    ID As Long
    Name As String * 4
End Type
Public Sub Main()
    Dim r As Rec
    Debug.Print Len(r)
End Sub
