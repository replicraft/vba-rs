' INV: vbal.5.6.9.8.4, op.xor, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.4 Xor with a Null operand is Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: Null
' EXPECT: 6
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print TypeName(5 Xor n)
    Debug.Print TypeName(n Xor 5)
    Debug.Print TypeName(0 Xor n)
    Debug.Print TypeName(n Xor n)
    Debug.Print CStr(12 Xor 10)
End Sub
