' INV: stmt.on-error
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: main continues 5
Sub NoHandler()
    Error 5
End Sub
Public Sub Main()
    On Error Resume Next
    NoHandler
    Debug.Print "main continues " & Err.Number
End Sub
