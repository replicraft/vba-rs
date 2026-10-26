' INV: type.object, expr.member
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1  5 
Public Sub Main()
    Dim o As Object
    Set o = New Collection
    o.Add 5
    Debug.Print o.Count; o(1)
End Sub
