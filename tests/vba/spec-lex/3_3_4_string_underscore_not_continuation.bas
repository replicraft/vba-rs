' INV: vbal.3.3.4, lit.string, lex.continuation
' SOURCE: spec ms-vbal 3.3.4 string tokens, an underscore inside a terminated string is an ordinary character
' EXPECT: 3
' EXPECT: a _
' EXPECT: 9
Public Sub Main()
    Debug.Print CStr(Len("a _"))
    Debug.Print "a _"
    Debug.Print CStr(Len("x _ y _ z"))
End Sub
