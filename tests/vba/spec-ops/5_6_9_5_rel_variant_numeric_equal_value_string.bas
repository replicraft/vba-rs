' INV: vbal.5.6.9.5, op.lt, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational exception: numeric never equal to String regardless of values
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim n As Variant, s As Variant, d As Variant, t As Variant, m As Variant
    n = 5
    s = "5"
    d = 2.5
    t = "abc"
    m = -1000000
    Debug.Print CStr(n = s)
    Debug.Print CStr(n < s)
    Debug.Print CStr(d < t)
    Debug.Print CStr(m < s)
    Debug.Print CStr(d < s)
End Sub
