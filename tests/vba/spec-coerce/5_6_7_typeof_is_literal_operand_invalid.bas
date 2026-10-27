' INV: vbal.5.6.7, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expressions, the operand must be classified as variable, function, property or unbound member (not a value)
' ERROR: load
Public Sub Main()
    Dim b As Boolean
    b = TypeOf Nothing Is Object
End Sub
