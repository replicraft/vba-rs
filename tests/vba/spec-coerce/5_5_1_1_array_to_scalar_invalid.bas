' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, non-Byte array to a non-array type other than Variant is invalid
' ERROR: load
Public Sub Main()
    Dim a() As Long
    Dim n As Long
    ReDim a(1)
    n = a
End Sub
