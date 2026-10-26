' INV: vbal.3.3.4, lit.string
' SOURCE: spec ms-vbal 3.3.4 string tokens, a STRING may end at LINE-END without a closing quote
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: an unterminated string literal is a lexical error
' EXPECT: abc
Public Sub Main()
    Debug.Print "abc
End Sub
