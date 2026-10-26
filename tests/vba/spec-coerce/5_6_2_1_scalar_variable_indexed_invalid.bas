' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, indexing a variable whose declared type is not Object, Variant, class or array is invalid
' ERROR: load
Public Sub Main()
    Dim n As Long
    Dim x As Long
    n = 5
    x = n(1)
End Sub
