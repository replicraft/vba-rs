' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex n >= &H100000000 with & suffix is invalid
' ERROR: load
Public Sub Main()
    Dim x As Variant
    x = &H100000000&
End Sub
