' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Preserve keeps data, new elements default
' EXPECT: 10 20 0 0
Public Sub Main()
    Dim a() As Long
    ReDim a(1)
    a(0) = 10
    a(1) = 20
    ReDim Preserve a(3)
    Debug.Print CStr(a(0)) & " " & CStr(a(1)) & " " & CStr(a(2)) & " " & CStr(a(3))
End Sub
