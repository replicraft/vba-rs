' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a UDT not imported from an external reference to Variant is invalid
' ERROR: load
Type P
    X As Long
End Type
Public Sub Main()
    Dim a As P
    Dim v As Variant
    v = a
End Sub
