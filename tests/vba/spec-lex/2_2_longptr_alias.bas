' INV: vbal.2.2, type.longptr
' SOURCE: spec ms-vbal 2.2 entities and declared types, LongPtr is a type alias valid wherever its underlying declared type is valid
' EXPECT: 6
' EXPECT: 8
Public Sub Main()
    Dim p As LongPtr
    p = 5
    Debug.Print CStr(p + 1)
    Dim a(1 To 2) As LongPtr
    a(1) = 3
    a(2) = 5
    Debug.Print CStr(a(1) + a(2))
End Sub
