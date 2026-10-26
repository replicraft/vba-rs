' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, Null and Empty operand value types
' EXPECT: [ab]
' EXPECT: [a]
' EXPECT: [a]
' EXPECT: [1]
' EXPECT: []
' EXPECT: True
' EXPECT: String
' EXPECT: String
Public Sub Main()
    Dim n As Variant, e As Variant
    n = Null
    Debug.Print "[" & ("a" & "b") & "]"
    Debug.Print "[" & (n & "a") & "]"
    Debug.Print "[" & ("a" & n) & "]"
    Debug.Print "[" & (n & 1) & "]"
    Debug.Print "[" & (n & e) & "]"
    Debug.Print IsNull(n & n)
    Debug.Print TypeName(n & "x")
    Debug.Print TypeName(1 & 2)
End Sub
