' INV: op.pow, op.precedence
' SOURCE: derived equal-precedence operators evaluate left to right (operator-precedence page)
' EXPECT: 64
Public Sub Main()
    Debug.Print CStr(2 ^ 3 ^ 2)
End Sub
