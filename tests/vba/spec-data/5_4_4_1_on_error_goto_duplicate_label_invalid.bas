' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 exactly one statement-label-definition for the label
' ERROR: load
Public Sub Main()
    On Error GoTo dup
    Debug.Print "x"
    Exit Sub
dup:
    Debug.Print "a"
dup:
    Debug.Print "b"
End Sub
