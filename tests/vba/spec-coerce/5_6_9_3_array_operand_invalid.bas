' INV: vbal.5.6.9.3
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators static semantics, an operand with an array declared type is invalid
' ERROR: load
Public Sub Main()
    Dim a(1) As Long
    Dim x As Variant
    x = a + 1
End Sub
