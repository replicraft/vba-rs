' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 arguments for a ParamArray parameter are positional only; naming it is not accepted
' METHOD: by construction
' ERROR: load
Private Sub G(a As Long, ParamArray rest() As Variant)
End Sub
Public Sub Main()
    G 1, rest:=2
End Sub
