' INV: decl.optional
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  6  11 
Function F(Optional ByVal n As Long = 5) As Long
    n = n + 1
    F = n
End Function
Public Sub Main()
    Debug.Print F(); F(10)
End Sub
