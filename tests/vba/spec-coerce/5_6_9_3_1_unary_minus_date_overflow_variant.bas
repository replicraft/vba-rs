' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 Unary - operator, Date negation that overflows Date yields the Double when the operand is a Variant
' EXPECT: Double
' EXPECT: -2958465
Public Sub Main()
    Dim v As Variant
    v = CDate(2958465)
    Debug.Print TypeName(-v)
    Debug.Print CStr(-v)
End Sub
