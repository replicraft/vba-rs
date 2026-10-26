' INV: vbal.3.3.4, lit.string
' SOURCE: spec ms-vbal 3.3.4 string tokens, a STRING may end at LINE-END without a closing quote
' XFAIL: wrong pending
' EXPECT: abc
Public Sub Main()
    Debug.Print "abc
End Sub
