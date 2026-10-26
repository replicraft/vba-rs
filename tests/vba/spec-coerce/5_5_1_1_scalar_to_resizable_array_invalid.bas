' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a scalar to a non-Byte resizable array is invalid
' ERROR: load
Public Sub Main()
    Dim a() As Long
    a = 5
End Sub
