' INV: vbal.5.6.9.3.7, op.pow
' SOURCE: spec ms-vbal 5.6.9.3.7 ^ Operator, a Null operand gives Null
' EXPECT: Null
' EXPECT: Null
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print TypeName(n ^ 2)
    Debug.Print TypeName(2 ^ n)
End Sub
