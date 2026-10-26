' INV: vbal.5.6.9.3
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators static semantics, an operand with a UDT declared type is invalid
' ERROR: load
Type P
    X As Long
End Type
Public Sub Main()
    Dim u As P
    Dim x As Variant
    x = u * 2
End Sub
