' INV: op.pow, op.precedence
' SOURCE: spec ms-vbal 5.6.9.1 operator associativity
' EXPECT: 64
Public Sub Main()
    Debug.Print CStr(2 ^ 3 ^ 2)
End Sub
