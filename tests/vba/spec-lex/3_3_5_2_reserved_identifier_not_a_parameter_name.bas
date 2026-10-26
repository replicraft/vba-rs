' INV: vbal.3.3.5.2
' SOURCE: spec ms-vbal 3.3.5.2 reserved identifiers, a reserved identifier cannot name a parameter
' ERROR: load
Public Sub Main()
    P 1
End Sub
Private Sub P(ByVal Select As Long)
End Sub
