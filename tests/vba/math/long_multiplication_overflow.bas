' INV: op.mul, type.long
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim a As Long
    a = 100000
    Debug.Print a * a
End Sub
