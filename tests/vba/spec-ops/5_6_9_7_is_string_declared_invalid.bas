' INV: vbal.5.6.9.7, op.is
' SOURCE: spec ms-vbal 5.6.9.7 Is operator operand declared String is invalid
' ERROR: load
Public Sub Main()
    Dim s As String, r As Boolean
    r = (s Is Nothing)
End Sub
