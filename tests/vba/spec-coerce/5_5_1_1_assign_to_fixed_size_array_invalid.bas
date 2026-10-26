' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, any type to a fixed-size array is invalid
' ERROR: load
Public Sub Main()
    Dim a(1) As Long
    Dim b(1) As Long
    a = b
End Sub
