' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable name must differ from parameter name
' ERROR: load
Private Sub P(ByVal a As Integer)
    Dim a As Long
End Sub
Public Sub Main()
    P 1
End Sub
