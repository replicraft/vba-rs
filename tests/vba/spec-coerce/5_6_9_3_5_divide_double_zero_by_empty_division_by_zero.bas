' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, 0 divided by Empty with a Double left operand raises Division by zero (error 11)
' ERROR: 11
Public Sub Main()
    Dim a As Double
    Dim e As Variant
    Dim r As Variant
    a = 0
    r = a / e
    Debug.Print "not reached"
End Sub
