' INV: stmt.rem, lex.comment
' SOURCE: spec ms-vbal 5.4.1.2 Rem statement, 3.3.1 comment text
' EXPECT: a
' EXPECT: b
Public Sub Main()
    Rem this is a comment
    Debug.Print "a" ' trailing comment
    Debug.Print "b": Rem another
End Sub
