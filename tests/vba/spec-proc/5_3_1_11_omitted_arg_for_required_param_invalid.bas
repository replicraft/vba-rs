' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 omitted positional argument for a non-optional parameter, even if later named
' ERROR: load
Sub S(a As Long, b As Long)
End Sub
Public Sub Main()
    S , a:=1, b:=2
End Sub
