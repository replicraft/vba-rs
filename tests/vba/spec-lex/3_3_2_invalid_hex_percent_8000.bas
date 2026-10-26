' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex &H8000..&H7FFFFFFF with % suffix is invalid
' ERROR: load
Public Sub Main()
    Dim x As Long
    x = &H10000%
End Sub
