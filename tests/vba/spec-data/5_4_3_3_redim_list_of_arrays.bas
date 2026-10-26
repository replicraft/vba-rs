' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 redim declaration list
' EXPECT: 1 2
Public Sub Main()
    Dim a() As Long, b() As Long
    ReDim a(1), b(2)
    Debug.Print CStr(UBound(a)) & " " & CStr(UBound(b))
End Sub
