' INV: vbal.6.1.2.11.1.5
' SOURCE: spec ms-vbal 6.1.2.11.1.5 ChrB outside 0..255 raises error 6 (Overflow)
' XFAIL: wrong pending
' EXPECT: err256=6
' EXPECT: errm1=6
Public Sub Main()
    Dim s As String
    On Error Resume Next
    s = ChrB(256)
    Debug.Print "err256=" & CStr(Err.Number)
    Err.Clear
    s = ChrB(-1)
    Debug.Print "errm1=" & CStr(Err.Number)
End Sub
