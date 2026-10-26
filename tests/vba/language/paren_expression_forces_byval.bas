' INV: expr.paren, decl.byref-byval
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0 
' EXPECT:  1 
Sub Bump(ByRef n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim x As Long
    Bump (x)
    Debug.Print x
    Bump x
    Debug.Print x
End Sub
