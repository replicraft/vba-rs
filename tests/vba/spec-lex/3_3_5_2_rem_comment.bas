' INV: vbal.3.3.5.2, lex.comment
' SOURCE: spec ms-vbal 3.3.5.2 reserved identifiers, Rem is a reserved keyword that starts a comment
' EXPECT: a
' EXPECT: b
Public Sub Main()
    Rem Debug.Print "no"
    Debug.Print "a"
    REM another comment
    Debug.Print "b": rem trailing comment
End Sub
