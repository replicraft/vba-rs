' INV: stmt.redim, decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2  3 z[]
Public Sub Main()
    Dim m() As String
    ReDim m(1 To 2, 1 To 3)
    m(2, 3) = "z"
    Debug.Print UBound(m, 1); UBound(m, 2); m(2, 3); "[" & m(1, 1) & "]"
End Sub
