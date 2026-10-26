' INV: expr.dict
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  10 
Public Sub Main()
    Dim c As New Collection
    c.Add 10, "k"
    Debug.Print c!k
End Sub
