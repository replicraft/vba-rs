' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 constant name must differ from parameter name
' ERROR: load
Private Sub P(ByVal a As Integer)
    Const a = 1
End Sub
Public Sub Main()
    P 1
End Sub
