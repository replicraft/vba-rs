' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume Next continues after the faulting statement
' EXPECT: a
' EXPECT: b
Public Sub Main()
    On Error GoTo h
    Debug.Print "a"
    Error 5
    Debug.Print "b"
    Exit Sub
h:
    Resume Next
End Sub
