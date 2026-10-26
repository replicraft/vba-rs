' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, a nonzero Double divided by 0 raises Division by zero
' ERROR: 11
Public Sub Main()
    Dim a As Double, b As Double
    Dim r As Double
    a = 1.5: b = 0
    r = a / b
    Debug.Print "not reached"
End Sub
