' INV: vbal.5.6.9.6, op.like
' SOURCE: spec ms-vbal 5.6.9.6 Like invalid when a declared operand type is an array
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As String
    Dim r As Boolean
    r = a Like "x"
End Sub
