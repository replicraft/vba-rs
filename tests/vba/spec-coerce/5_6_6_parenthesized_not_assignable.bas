' INV: vbal.5.6.6, expr.paren
' SOURCE: spec ms-vbal 5.6.6 Parenthesized expressions, a parenthesized expression is a value expression and cannot be an assignment target
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim x As Long
    (x) = 5
End Sub
