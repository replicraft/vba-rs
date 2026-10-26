' INV: vbal.5.6.9.5, op.eq
' SOURCE: spec ms-vbal 5.6.9.5 relational Single vs Double has effective type Single
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: a Single compared with a Double (or with a numeric String) compares as Double, so CSng(0.1) = 0.1 is False
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
