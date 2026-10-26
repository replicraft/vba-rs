' INV: vbal.5.4.4, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Default policy returns the error to the caller's policy
' EXPECT: caught 11
Private Sub Inner()
    Dim x As Long
    x = 1 / 0
    Debug.Print "not reached"
End Sub
Public Sub Main()
    On Error GoTo h
    Inner
    Debug.Print "not reached"
    Exit Sub
h:
    Debug.Print "caught " & CStr(Err.Number)
End Sub
