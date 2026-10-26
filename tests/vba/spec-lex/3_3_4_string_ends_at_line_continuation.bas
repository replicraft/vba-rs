' INV: vbal.3.3.4, lit.string, lex.continuation
' SOURCE: spec ms-vbal 3.3.4 string tokens, a STRING may end in a line-continuation; trailing whitespace before it is not part of the value
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error
' EXPECT: [ab]
Public Sub Main()
    Debug.Print "[" & "ab   _
        & "]"
End Sub
