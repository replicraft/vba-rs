' INV: op.eq, op.lt, expr.implicit-conv
' SOURCE: spec ms-vbal 5.6.9.5 relational operators (effective Integer)
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(10 = "10")
    Debug.Print CStr(9 < "10")
End Sub
