' INV: func.array, expr.index
' SOURCE: derived a Variant array may hold arrays; chained indexes
' EXPECT:  3  2 
Public Sub Main()
    Dim v As Variant
    v = Array(Array(1, 2), Array(3, 4))
    Debug.Print v(1)(0); v(0)(1)
End Sub
