' INV: func.array, expr.index
' SOURCE: derived a subscript outside the nested array is error 9 (Subscript out of range)
' ERROR: 9
Public Sub Main()
    Dim v As Variant
    v = Array(Array(1, 2))
    v(0)(5) = 1
End Sub
