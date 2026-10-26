' INV: vbal.3.3.2, lit.octal
' SOURCE: spec ms-vbal 3.3.2 number tokens, octal &o200000..&o17777777777 with % suffix is invalid
' ERROR: load
Public Sub Main()
    Dim x As Long
    x = &O200000%
End Sub
