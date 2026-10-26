' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 named argument without a same-named parameter is incompatible
' ERROR: load
Sub S(a As Long)
End Sub
Public Sub Main()
    S zzz:=1
End Sub
