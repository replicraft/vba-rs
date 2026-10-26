' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Goto policy resets to Default, Terminate policy for host-invoked procedure
' ERROR: 11
Public Sub Main()
    On Error GoTo H
    Error 5
H:
    Error 11
End Sub
