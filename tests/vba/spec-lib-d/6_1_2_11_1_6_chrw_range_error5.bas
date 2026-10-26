' INV: vbal.6.1.2.11.1.6
' SOURCE: spec ms-vbal 6.1.2.11.1.6 ChrW outside -32767..65535 raises error 5
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: ChrW(-32768) is accepted
' EXPECT: hi=5
' EXPECT: lo=5
Public Sub Main()
    Dim s As String
    On Error Resume Next
    s = ChrW(65536)
    Debug.Print "hi=" & CStr(Err.Number)
    Err.Clear
    s = ChrW(-32768)
    Debug.Print "lo=" & CStr(Err.Number)
End Sub
