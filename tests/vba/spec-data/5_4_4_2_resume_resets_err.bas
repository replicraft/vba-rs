' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume resets the Err object
' EXPECT: err=0
Public Sub Main()
    On Error GoTo h
    Error 5
    Debug.Print "err=" & CStr(Err.Number)
    Exit Sub
h:
    Resume Next
End Sub
