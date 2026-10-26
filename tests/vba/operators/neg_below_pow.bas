' INV: op.neg, op.pow, op.precedence
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: -4
Public Sub Main()
    Debug.Print CStr(-2 ^ 2)
End Sub
