' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 more positional arguments than parameters is statically incompatible
' ERROR: load
Sub S(a As Long)
End Sub
Public Sub Main()
    S 1, 2
End Sub
