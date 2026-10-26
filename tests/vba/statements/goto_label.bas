' INV: stmt.goto, lex.label
' SOURCE: spec ms-vbal 5.4.2.12 GoTo statement
' EXPECT: after
Public Sub Main()
    GoTo Skip
    Debug.Print "skipped"
Skip:
    Debug.Print "after"
End Sub
