' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Preserve may change the upper bound of the last dimension
' EXPECT: 5 0 7
Public Sub Main()
    Dim a() As Long
    ReDim a(1, 1)
    a(1, 1) = 5
    a(0, 1) = 7
    ReDim Preserve a(1, 3)
    Debug.Print CStr(a(1, 1)) & " " & CStr(a(1, 3)) & " " & CStr(a(0, 1))
End Sub
