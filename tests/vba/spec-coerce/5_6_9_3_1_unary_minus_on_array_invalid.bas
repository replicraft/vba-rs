' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, unary - on an array declared type is invalid
' ERROR: load
Public Sub Main()
    Dim a(1) As Long
    Dim x As Variant
    x = -a
End Sub
