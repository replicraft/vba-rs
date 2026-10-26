' INV: vbal.5.6.9.8, op.and
' SOURCE: spec ms-vbal 5.6.9.8 logical operator invalid when a declared operand type is an array
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    Dim r As Long
    r = a And 1
End Sub
