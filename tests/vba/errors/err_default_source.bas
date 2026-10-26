' INV: obj.err
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: VBAProject
Public Sub Main()
    On Error Resume Next
    Error 5
    Debug.Print Err.Source
End Sub
