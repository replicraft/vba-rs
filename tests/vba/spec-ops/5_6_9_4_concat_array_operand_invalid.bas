' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, array declared operand is invalid
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim s As String
    s = a & "x"
End Sub
