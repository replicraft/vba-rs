' INV: stmt.if-then-else, expr.implicit-conv
' SOURCE: derived a nonzero number is True in a condition
' EXPECT: t
' EXPECT: s
Public Sub Main()
    If 2 Then Debug.Print "t"
    If 0 Then Debug.Print "f"
    If "1" Then Debug.Print "s"
End Sub
