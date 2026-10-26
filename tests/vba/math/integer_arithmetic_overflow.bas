' INV: op.add, type.integer
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim a As Integer, b As Integer
    a = 20000: b = 20000
    Debug.Print a + b
End Sub
