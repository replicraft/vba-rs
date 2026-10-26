' INV: func.len, type.udt
' SOURCE: unverified Len of a user-defined type with a dynamic array member counts what Put writes: the descriptor of 2 + 8 bytes per dimension and the elements; real VBA may report only a pointer
' EXPECT: 16
' EXPECT: 28
Type Rec
    id As Integer
    v() As Integer
End Type
Public Sub Main()
    Dim r As Rec
    ReDim r.v(1 To 2)
    Debug.Print CStr(Len(r))
    ReDim r.v(1 To 2, 1 To 2)
    Debug.Print CStr(Len(r))
End Sub
