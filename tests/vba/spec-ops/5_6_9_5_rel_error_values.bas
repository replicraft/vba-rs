' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Error vs Error compares standard error codes
' XFAIL: wrong pending
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As Variant, b As Variant
    a = CVErr(3)
    b = CVErr(5)
    Debug.Print CStr(a = CVErr(3))
    Debug.Print CStr(a = b)
    Debug.Print CStr(a < b)
    Debug.Print CStr(b > a)
End Sub
