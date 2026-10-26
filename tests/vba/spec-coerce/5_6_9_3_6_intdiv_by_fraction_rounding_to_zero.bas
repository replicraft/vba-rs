' INV: vbal.5.6.9.3.6, op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator, a divisor that rounds to 0 before dividing raises Division by zero
' ERROR: 11
Public Sub Main()
    Dim a As Long
    Dim d As Double
    Dim r As Variant
    a = 7: d = 0.4
    r = a \ d
    Debug.Print "not reached"
End Sub
