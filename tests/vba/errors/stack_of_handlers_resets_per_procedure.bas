' INV: stmt.on-error
' SOURCE: spec ms-vbal 5.4.4 Default policy returns the error to the caller, Resume Next policy
' EXPECT: main continues 5
Sub NoHandler()
    Error 5
End Sub
Public Sub Main()
    On Error Resume Next
    NoHandler
    Debug.Print "main continues " & Err.Number
End Sub
