' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 On Error GoTo 0 disables the error-handling policy
' EXPECT: before
' ERROR: 11
Public Sub Main()
    Dim x As Long
    On Error Resume Next
    x = 1 / 0
    On Error GoTo 0
    Debug.Print "before"
    x = 1 / 0
    Debug.Print "after"
End Sub
