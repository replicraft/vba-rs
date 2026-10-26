' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, Double beyond Currency magnitude range is Overflow
' ERROR: 6
Public Sub Main()
    Dim c As Currency
    Dim d As Double
    d = 1E+300
    c = d
    Debug.Print "not reached"
End Sub
