' INV: vbal.5.6.9.5, op.lt, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational exception: both declared Variant, numeric vs String
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Dim n As Variant, s As Variant
    n = 100
    s = "5"
    Debug.Print CStr(n < s)
    Debug.Print CStr(n = s)
    Debug.Print CStr(n > s)
    Debug.Print CStr(s > n)
    Debug.Print CStr(n <= s)
    Debug.Print CStr(n <> s)
    Debug.Print CStr(s <= n)
    Debug.Print CStr(s >= n)
End Sub
