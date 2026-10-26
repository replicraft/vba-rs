' INV: expr.index
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  5  6  8 
Public Sub Main()
    Dim a(2) As Long, c As New Collection
    a(1) = 5
    c.Add 6
    Debug.Print a(1); c(1); Array(7, 8)(1)
End Sub
