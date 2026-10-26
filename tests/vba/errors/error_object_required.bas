' INV: stmt.on-error
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  91 
Public Sub Main()
    Dim o As Object
    On Error Resume Next
    o.Foo
    Debug.Print Err.Number
End Sub
