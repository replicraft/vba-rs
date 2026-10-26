' INV: stmt.rem, lex.comment
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/rem-statement
' EXPECT: a
' EXPECT: b
Public Sub Main()
    Rem this is a comment
    Debug.Print "a" ' trailing comment
    Debug.Print "b": Rem another
End Sub
