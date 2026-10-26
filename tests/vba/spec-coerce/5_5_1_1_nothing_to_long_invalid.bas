' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, Nothing to a non-class type is invalid
' ERROR: load
Public Sub Main()
    Dim n As Long
    n = Nothing
End Sub
