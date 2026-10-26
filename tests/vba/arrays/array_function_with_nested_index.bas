' INV: func.array, expr.index
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  3 
Public Sub Main()
    Debug.Print Array(Array(1, 2, 3), 4)(0)(2)
End Sub
