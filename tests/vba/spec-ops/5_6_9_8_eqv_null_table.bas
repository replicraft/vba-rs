' INV: vbal.5.6.9.8.5, op.eqv, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.5 Eqv with a Null operand is Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: -7
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print TypeName(5 Eqv n)
    Debug.Print TypeName(n Eqv 5)
    Debug.Print TypeName(-1 Eqv n)
    Debug.Print TypeName(n Eqv n)
    Debug.Print CStr(12 Eqv 10)
End Sub
