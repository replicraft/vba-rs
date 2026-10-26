' INV: stmt.put, stmt.get, type.udt
' SOURCE: derived ms-vbal 5.4.5.11 a dynamic array member of a user-defined type is written with its descriptor in member order
' EXPECT: 7 3 1 2 3
' GRANT: file-read
' GRANT: file-write
Private Type R
    id As Integer
    v() As Integer
End Type
Public Sub Main()
    MkDir "/t"
    Dim x As R, y As R
    x.id = 7
    ReDim x.v(1 To 3)
    x.v(1) = 1: x.v(2) = 2: x.v(3) = 3
    Open "/t/g.bin" For Binary As #1
    Put #1, , x
    Seek #1, 1
    Get #1, , y
    Close #1
    Debug.Print CStr(y.id) & " " & CStr(UBound(y.v)) & " " & CStr(y.v(1)) & " " & CStr(y.v(2)) & " " & CStr(y.v(3))
End Sub
