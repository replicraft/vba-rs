' INV: vbal.3.3.2, lit.octal
' SOURCE: spec ms-vbal 3.3.2 number tokens, octal n >= &o40000000000 without suffix is invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim x As Variant
    x = &O40000000000
End Sub
