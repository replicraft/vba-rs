' INV: vbal.3.3.2, lit.integer
' SOURCE: spec ms-vbal 3.3.2 number tokens, decimal n >= 2147483647 with & suffix is invalid
' ERROR: load
Public Sub Main()
    Dim x As Variant
    x = 2147483648&
End Sub
