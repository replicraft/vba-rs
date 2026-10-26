' INV: stmt.sub, stmt.type, decl.byref-byval
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2 
Type P
    X As Long
End Type
Sub Bump(ByRef q As P)
    q.X = q.X + 1
End Sub
Public Sub Main()
    Dim p As P
    Bump p
    Bump p
    Debug.Print p.X
End Sub
