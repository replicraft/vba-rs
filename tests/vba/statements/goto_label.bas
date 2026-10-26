' INV: stmt.goto, lex.label
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/goto-statement
' EXPECT: after
Public Sub Main()
    GoTo Skip
    Debug.Print "skipped"
Skip:
    Debug.Print "after"
End Sub
