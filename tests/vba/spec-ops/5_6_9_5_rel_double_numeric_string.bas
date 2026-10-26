' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Double effective type with integral, String, Empty
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim d As Double, e As Variant
    d = 0.1
    Debug.Print CStr(d = "0.1")
    Debug.Print CStr(d < 1)
    Debug.Print CStr(2 > d)
    Debug.Print CStr(d > e)
    Debug.Print CStr(1.5 < "10")
End Sub
