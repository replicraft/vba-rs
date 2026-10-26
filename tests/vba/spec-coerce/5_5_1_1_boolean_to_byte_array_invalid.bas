' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, Boolean to resizable Byte() is invalid
' ERROR: load
Public Sub Main()
    Dim b() As Byte
    Dim t As Boolean
    t = True
    b = t
End Sub
