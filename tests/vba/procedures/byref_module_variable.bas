' INV: decl.byref-byval, decl.scope
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  11 
Dim g As Long
Sub Inc(ByRef n As Long)
    n = n + 1
    g = g + 10
End Sub
Public Sub Main()
    Inc g
    Debug.Print g
End Sub
