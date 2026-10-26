' INV: stmt.goto, lex.label
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/goto-statement
' EXPECT: twenty
Public Sub Main()
    GoTo 20
    10 Debug.Print "ten"
    20 Debug.Print "twenty"
End Sub
