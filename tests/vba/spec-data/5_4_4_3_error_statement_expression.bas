' INV: vbal.5.4.4.3, stmt.error
' SOURCE: spec ms-vbal 5.4.4.3 error-number is an integer expression
' EXPECT: 14
Public Sub Main()
    Const base = 10
    On Error Resume Next
    Error base + 4
    Debug.Print CStr(Err.Number)
End Sub
