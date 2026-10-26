' INV: vbal.5.6.2.1, expr.index
' SOURCE: spec ms-vbal 5.6.2.1 Evaluation to a data value, an array indexed with a different number of arguments than its rank is invalid
' ERROR: load
Public Sub Main()
    Dim a(1 To 2, 1 To 2) As Long
    Dim x As Long
    x = a(1)
End Sub
