' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 label must be in the containing procedure
' ERROR: load
Private Sub Other()
other_label:
    Debug.Print "x"
End Sub
Public Sub Main()
    On Error GoTo other_label
    Other
End Sub
