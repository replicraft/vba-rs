' INV: vbal.3.3.1, lex.comment
' SOURCE: spec ms-vbal 3.3.1 separator tokens, characters from a single quote to LINE-END are comment text even if they contain colons
' EXPECT: a
' EXPECT: b
Public Sub Main()
    Debug.Print "a" ' c : Debug.Print "no"
    Debug.Print "b" : ' : Debug.Print "no"
End Sub
