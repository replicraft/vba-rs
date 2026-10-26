' INV: vbal.5.4.4, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 an error inside the handler is not re-handled by the same handler
' EXPECT: handler once
' ERROR: 6
Public Sub Main()
    On Error GoTo h
    Error 5
    Exit Sub
h:
    Debug.Print "handler once"
    Error 6
End Sub
