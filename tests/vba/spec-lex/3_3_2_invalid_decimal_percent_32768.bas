' INV: vbal.3.3.2, lit.integer
' SOURCE: spec ms-vbal 3.3.2 number tokens, decimal n >= 32768 with % suffix is invalid
' ERROR: load
Public Sub Main()
    Dim x As Long
    x = 32768%
End Sub
