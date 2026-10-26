' INV: vbal.5.4.4.2, stmt.resume
' SOURCE: spec ms-vbal 5.4.4.2 Resume label must be defined in the procedure
' ERROR: load
Public Sub Main()
    On Error GoTo h
    Error 5
    Exit Sub
h:
    Resume nowhere
End Sub
