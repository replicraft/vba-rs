' INV: decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
Public Sub Main()
    Dim o(1) As Object
    Debug.Print o(0) Is Nothing
End Sub
