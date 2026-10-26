' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a UDT to a different UDT is invalid
' ERROR: load
Type P
    X As Long
End Type
Type Q
    X As Long
End Type
Public Sub Main()
    Dim a As P
    Dim b As Q
    b = a
End Sub
