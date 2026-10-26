' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume re-executes the statement that caused the error
' EXPECT: tries=3
Dim n As Long
Private Function Flaky() As Long
    n = n + 1
    If n < 3 Then Error 5
    Flaky = n
End Function
Public Sub Main()
    Dim r As Long
    On Error GoTo h
    r = Flaky()
    Debug.Print "tries=" & CStr(r)
    Exit Sub
h:
    Resume
End Sub
