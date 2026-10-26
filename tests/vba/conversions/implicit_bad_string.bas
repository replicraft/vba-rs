' INV: expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 String to numeric Let-coercion
' ERROR: 13
Public Sub Main()
    Dim n As Long
    n = "x"
End Sub
