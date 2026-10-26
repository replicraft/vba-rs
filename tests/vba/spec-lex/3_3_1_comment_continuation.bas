' INV: vbal.3.3.1, lex.comment, lex.continuation
' SOURCE: spec ms-vbal 3.3.1 separator tokens, comment-body may contain a line continuation so the comment extends to the next physical line
' EXPECT: a
' EXPECT: c
Public Sub Main()
    Debug.Print "a" ' this comment continues _
    Debug.Print "b"
    Debug.Print "c"
End Sub
