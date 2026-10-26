' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational operator invalid when a declared operand type is an array
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim r As Boolean
    r = (a = 1)
End Sub
