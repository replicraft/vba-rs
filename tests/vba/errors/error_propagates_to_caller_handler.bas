' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Default and Goto error-handling policies
' EXPECT: caught 13
Sub Inner()
    Error 13
End Sub
Public Sub Main()
    On Error GoTo H
    Inner
    Debug.Print "no"
    Exit Sub
H:
    Debug.Print "caught " & Err.Number
End Sub
