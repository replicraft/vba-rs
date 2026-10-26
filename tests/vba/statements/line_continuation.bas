' INV: lex.continuation
' SOURCE: spec ms-vbal 3.2.2 line continuation
' EXPECT: abc
Public Sub Main()
    Dim s As String
    s = "a" & _
        "b" & _
        "c"
    Debug.Print s
End Sub
