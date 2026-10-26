' INV: stmt.type, decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1  9 
Type Box
    V(1 To 2) As Long
End Type
Public Sub Main()
    Dim a As Box, b As Box
    a.V(1) = 1
    b = a
    b.V(1) = 9
    Debug.Print a.V(1); b.V(1)
End Sub
