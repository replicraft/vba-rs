' INV: op.eq, op.lt, expr.implicit-conv
' SOURCE: unverified numeric string compared with number is numeric
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(10 = "10")
    Debug.Print CStr(9 < "10")
End Sub
