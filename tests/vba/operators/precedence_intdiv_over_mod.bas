' INV: op.precedence, op.intdiv, op.mod
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: 1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(7 \ 2 Mod 2)
    Debug.Print CStr(11 Mod 4 \ 2)
End Sub
