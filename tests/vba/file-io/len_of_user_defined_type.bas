' INV: func.len, type.udt
' SOURCE: derived Len of a user-defined type is its size in bytes: Long (4) + String * 4 (4)
' EXPECT:  8 
Type Rec
    ID As Long
    Name As String * 4
End Type
Public Sub Main()
    Dim r As Rec
    Debug.Print Len(r)
End Sub
