' INV: vbal.3.3.2, lit.octal
' SOURCE: spec ms-vbal 3.3.2 number tokens, octal n >= &o2000000000000000000000 is invalid with any suffix
' ERROR: load
Public Sub Main()
    Dim x As Variant
    x = &O2000000000000000000000^
End Sub
