' INV: obj.err, stmt.on-error
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  5 
Public Sub Main()
    On Error Resume Next
    Error 5
    Dim x As Long
    x = 1
    Debug.Print Err.Number
End Sub
