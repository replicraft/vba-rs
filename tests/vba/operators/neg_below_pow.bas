' INV: op.neg, op.pow, op.precedence
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/operator-precedence
' EXPECT: -4
Public Sub Main()
    Debug.Print CStr(-2 ^ 2)
End Sub
