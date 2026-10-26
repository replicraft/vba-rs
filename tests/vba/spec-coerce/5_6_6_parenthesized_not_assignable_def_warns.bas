' INV: vbal.5.6.6, expr.paren
' SOURCE: spec ms-vbal 5.6.6 Parenthesized expressions, a parenthesized expression is a value expression and cannot be an assignment target (spec-strict: a warning by default, the program runs)
' EXPECT: 5
Public Sub Main()
    Dim x As Long
    (x) = 5
    Debug.Print CStr(x)
End Sub
