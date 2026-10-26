' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational Single vs Double has effective type Single
' XFAIL: wrong pending
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim s As Single, d As Double
    s = 0.1
    d = 0.1
    Debug.Print CStr(s = d)
    Debug.Print CStr(d = s)
    Debug.Print CStr(s = "0.1")
    Debug.Print CStr(s <= d)
End Sub
