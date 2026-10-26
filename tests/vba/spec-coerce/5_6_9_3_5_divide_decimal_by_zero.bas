' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, Decimal division by 0 raises Division by zero
' ERROR: 11
Public Sub Main()
    Dim a As Variant, b As Variant
    Dim r As Variant
    a = CDec(5): b = CDec(0)
    r = a / b
    Debug.Print "not reached"
End Sub
