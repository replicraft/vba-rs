' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 non-optional parameter without argument is statically incompatible
' ERROR: load
Sub S(a As Long, b As Long)
End Sub
Public Sub Main()
    S 1
End Sub
