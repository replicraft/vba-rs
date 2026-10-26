' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, a non-Variant source to a class or Object destination is invalid
' ERROR: load
Public Sub Main()
    Dim o As Object
    Dim n As Long
    n = 5
    o = n
End Sub
