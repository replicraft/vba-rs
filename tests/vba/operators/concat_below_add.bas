' INV: op.concat, op.add, op.precedence
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: 33
' EXPECT: a3
Public Sub Main()
    Debug.Print CStr(1 + 2 & 3)
    Debug.Print CStr("a" & 1 + 2)
End Sub
