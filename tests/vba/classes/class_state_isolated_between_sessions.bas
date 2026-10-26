' INV: decl.scope
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1 
'#MODULE Counter class
Public N As Long
'#MODULE Module1 standard
Dim c As New Counter
Public Sub Main()
    c.N = c.N + 1
    Debug.Print c.N
End Sub
