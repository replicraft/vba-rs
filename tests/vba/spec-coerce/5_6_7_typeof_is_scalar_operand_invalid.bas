' INV: vbal.5.6.7, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expressions, the operand must have a declared type of UDT, specific class, Object or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long
    Dim b As Boolean
    b = TypeOf n Is Object
End Sub
