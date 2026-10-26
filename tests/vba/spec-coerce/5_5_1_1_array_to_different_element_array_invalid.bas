' INV: vbal.5.5.1.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.1 Let-coercion static semantics, non-Byte array to resizable array of a different element type is invalid
' ERROR: load
Public Sub Main()
    Dim a() As Long
    Dim b() As Integer
    ReDim a(1)
    b = a
End Sub
