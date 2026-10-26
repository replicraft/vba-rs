' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Currency effective type coerces the other operand to Currency
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim c As Currency, s As Single
    c = 0.1
    Debug.Print CStr(c = 0.1)
    Debug.Print CStr(0.1 = c)
    Debug.Print CStr(c = "0.1")
    s = 0.1
    Debug.Print CStr(c = s)
End Sub
