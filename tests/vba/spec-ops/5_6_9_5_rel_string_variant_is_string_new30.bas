' INV: vbal.5.6.9.5, op.lt, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational String vs Variant (except Null) has effective type String (row new in 3.0)
' XFAIL: wrong pending
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Dim s As String, v As Variant
    s = "10"
    v = 9
    Debug.Print CStr(s < v)
    Debug.Print CStr(v > s)
    s = "abc"
    Debug.Print CStr(s > v)
    s = "9"
    v = 10
    Debug.Print CStr(s < v)
End Sub
