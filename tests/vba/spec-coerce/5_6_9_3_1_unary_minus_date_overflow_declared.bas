' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 Unary - operator, Date negation that overflows Date raises error 6 when the operand is not a Variant
' ERROR: 6
Public Sub Main()
    Dim d As Date
    Dim r As Variant
    d = 2958465
    r = -d
    Debug.Print "not reached"
End Sub
