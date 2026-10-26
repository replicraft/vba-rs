' INV: func.array, expr.index
' SOURCE: derived a Variant array may hold arrays (Array function page); an element of a nested array is assigned in place
' EXPECT:  99  1  4 
' EXPECT:  7  8 
Public Sub Main()
    Dim v As Variant
    v = Array(Array(1, 2, 3), Array(4, 5, 6))
    v(1)(2) = 99
    Debug.Print v(1)(2); v(0)(0); v(1)(0)
    Dim w(1) As Variant
    w(0) = Array(0, 0)
    w(0)(1) = 8
    w(0)(0) = 7
    Debug.Print w(0)(0); w(0)(1)
End Sub
