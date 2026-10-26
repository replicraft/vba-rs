' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex n >= &H10000000000000000 is invalid with any suffix
' ERROR: load
Public Sub Main()
    Dim x As Variant
    x = &H10000000000000000^
End Sub
