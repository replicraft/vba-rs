' INV: stmt.type, decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0  5 
Type P
    N As Long
End Type
Public Sub Main()
    Dim a(1 To 2) As P
    a(2).N = 5
    Debug.Print a(1).N; a(2).N
End Sub
