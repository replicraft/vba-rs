' INV: expr.paren
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  9  7 
Public Sub Main()
    Debug.Print (1 + 2) * 3; 1 + (2 * 3)
End Sub
