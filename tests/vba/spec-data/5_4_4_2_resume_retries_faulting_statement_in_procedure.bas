' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume re-executes the faulting statement after the handler fixed the cause
' EXPECT: q=5
Public Sub Main()
    Dim d As Long, q As Long
    On Error GoTo h
    q = 10 / d
    Debug.Print "q=" & CStr(q)
    Exit Sub
h:
    d = 2
    Resume
End Sub
