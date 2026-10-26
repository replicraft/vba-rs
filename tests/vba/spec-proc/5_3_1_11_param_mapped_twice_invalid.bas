' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 two arguments mapped to one parameter is incompatible
' ERROR: load
Sub S(a As Long, Optional b As Long)
End Sub
Public Sub Main()
    S 1, a:=2
End Sub
