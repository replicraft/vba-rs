' INV: func.array, expr.index
' SOURCE: derived a Variant array may hold arrays nested to any depth; the innermost element is assigned in place
' EXPECT:  5 
Public Sub Main()
    Dim v As Variant
    v = Array(Array(Array(1, 2), Array(3, 4)))
    v(0)(1)(0) = 5
    Debug.Print v(0)(1)(0)
End Sub
