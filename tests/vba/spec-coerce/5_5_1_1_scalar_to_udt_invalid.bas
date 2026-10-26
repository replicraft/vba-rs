' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a non-UDT non-Variant source to a UDT is invalid
' ERROR: load
Type P
    X As Long
End Type
Public Sub Main()
    Dim u As P
    u = 5
End Sub
