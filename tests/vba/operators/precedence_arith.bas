' INV: op.precedence, op.mul, op.add, op.intdiv, op.mod, op.pow
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: 14
' EXPECT: 20
' EXPECT: 3
' EXPECT: 1
' EXPECT: 1
' EXPECT: 4
' EXPECT: 12
Public Sub Main()
    Debug.Print CStr(2 + 3 * 4)
    Debug.Print CStr((2 + 3) * 4)
    Debug.Print CStr(10 - 4 - 3)
    Debug.Print CStr(2 * 3 \ 4)
    Debug.Print CStr(10 \ 3 * 3)
    Debug.Print CStr(7 Mod 4 + 1)
    Debug.Print CStr(2 ^ 2 * 3)
End Sub
