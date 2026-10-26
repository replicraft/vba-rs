' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, numeric to resizable Byte() is invalid
' ERROR: load
Public Sub Main()
    Dim b() As Byte
    Dim n As Long
    n = 5
    b = n
End Sub
