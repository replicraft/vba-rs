' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 Goto policy continues at the handler label
' EXPECT: start
' EXPECT: handler 9
Public Sub Main()
    On Error GoTo lbl
    Debug.Print "start"
    Error 9
    Debug.Print "skipped"
    Exit Sub
lbl:
    Debug.Print "handler " & CStr(Err.Number)
End Sub
