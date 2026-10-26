' INV: lex.continuation
' SOURCE: derived a trailing underscore continues the logical line
' EXPECT: abc
Public Sub Main()
    Dim s As String
    s = "a" & _
        "b" & _
        "c"
    Debug.Print s
End Sub
