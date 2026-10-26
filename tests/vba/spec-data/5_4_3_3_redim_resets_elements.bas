' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 elements reset to default without Preserve
' EXPECT: 0|
' EXPECT: 0|
Public Sub Main()
    Dim a() As Long
    Dim s() As String
    ReDim a(2)
    ReDim s(2)
    a(1) = 9
    s(1) = "x"
    ReDim a(2)
    ReDim s(2)
    Debug.Print CStr(a(1)) & "|" & s(1)
    ReDim a(4 To 6)
    Debug.Print CStr(a(5)) & "|" & s(0)
End Sub
