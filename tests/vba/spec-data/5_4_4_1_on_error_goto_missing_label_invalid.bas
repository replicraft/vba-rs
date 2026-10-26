' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 label of On Error GoTo must be defined in the procedure
' ERROR: load
Public Sub Main()
    On Error GoTo nowhere
    Debug.Print "x"
End Sub
