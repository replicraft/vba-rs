' INV: obj.err
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  5 
Sub Inner()
    On Error Resume Next
    Error 5
End Sub
Public Sub Main()
    Inner
    Debug.Print Err.Number
End Sub
