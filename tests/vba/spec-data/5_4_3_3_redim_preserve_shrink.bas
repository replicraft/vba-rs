' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Preserve with fewer elements discards the rest
' EXPECT: 1 10
' EXPECT: 1 10 0
Public Sub Main()
    Dim a() As Long
    ReDim a(2)
    a(0) = 10: a(1) = 11: a(2) = 12
    ReDim Preserve a(0)
    Debug.Print CStr(UBound(a) + 1) & " " & CStr(a(0))
    ReDim Preserve a(2)
    Debug.Print CStr(UBound(a) - 1) & " " & CStr(a(0)) & " " & CStr(a(2))
End Sub
