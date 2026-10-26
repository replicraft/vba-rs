' INV: stmt.if-then-else, expr.implicit-conv
' SOURCE: spec ms-vbal 5.6.16.3 Boolean expressions, 5.5.1.2.2 Let-coercion to Boolean
' EXPECT: t
' EXPECT: s
Public Sub Main()
    If 2 Then Debug.Print "t"
    If 0 Then Debug.Print "f"
    If "1" Then Debug.Print "s"
End Sub
