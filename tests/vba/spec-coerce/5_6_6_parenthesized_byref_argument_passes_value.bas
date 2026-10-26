' INV: vbal.5.6.6, expr.paren, decl.byref-byval
' SOURCE: spec ms-vbal 5.6.6 Parenthesized expressions, a parenthesized variable is a value expression so a ByRef parameter cannot modify the variable
' EXPECT: plain=9
' EXPECT: paren=1
Sub Bump(ByRef n As Long)
    n = 9
End Sub
Public Sub Main()
    Dim x As Long
    x = 1
    Bump x
    Debug.Print "plain=" & CStr(x)
    x = 1
    Bump (x)
    Debug.Print "paren=" & CStr(x)
End Sub
