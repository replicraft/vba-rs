' INV: vbal.5.6.9.6, op.like, lit.null
' SOURCE: spec ms-vbal 5.6.9.6 Like with a Null operand yields Null
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim n As Variant
    n = Null
    Debug.Print IsNull(n Like "a*")
    Debug.Print IsNull("abc" Like n)
    Debug.Print IsNull(n Like n)
End Sub
