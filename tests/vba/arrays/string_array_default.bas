' INV: decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: []
Public Sub Main()
    Dim s(1) As String
    Debug.Print "[" & s(0) & "]"
End Sub
