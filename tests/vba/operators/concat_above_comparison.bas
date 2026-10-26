' INV: op.concat, op.eq, op.precedence
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("a" & "b" = "ab")
End Sub
