' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, 0 divided by 0 raises Overflow (error 6), not Division by zero
' XFAIL: wrong pending
' ERROR: 6
Public Sub Main()
    Dim a As Double, b As Double
    Dim r As Variant
    a = 0: b = 0
    r = a / b
    Debug.Print "not reached"
End Sub
