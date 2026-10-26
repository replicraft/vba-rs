' INV: stmt.erase, decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: []
Public Sub Main()
    Dim s(1 To 2) As String
    s(1) = "x"
    Erase s
    Debug.Print "[" & s(1) & "]"
End Sub
