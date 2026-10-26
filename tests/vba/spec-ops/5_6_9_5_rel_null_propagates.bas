' INV: vbal.5.6.9.5, op.eq, lit.null
' SOURCE: spec ms-vbal 5.6.9.5 relational Null operand yields Null for every operator
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim n As Variant, e As Variant
    n = Null
    Debug.Print IsNull(n = 1)
    Debug.Print IsNull(1 <> n)
    Debug.Print IsNull(n < "a")
    Debug.Print IsNull("a" > n)
    Debug.Print IsNull(n <= n)
    Debug.Print IsNull(n >= e)
    Debug.Print IsNull(e = n)
    Debug.Print IsNull(n = #1/1/2000#)
    Debug.Print IsNull(n = True)
    Debug.Print IsNull(n = n)
End Sub
