' INV: vbal.3.3.4, lit.string
' SOURCE: spec ms-vbal 3.3.4 string tokens, empty string, doubled quote is one quote, length counts characters
' EXPECT: 0
' EXPECT: 3
' EXPECT: a"b
' EXPECT: 1
' EXPECT: 34
' EXPECT: 2
' EXPECT: it's a:b
Public Sub Main()
    Debug.Print CStr(Len(""))
    Debug.Print CStr(Len("a""b"))
    Debug.Print "a""b"
    Debug.Print CStr(Len(""""))
    Debug.Print CStr(Asc(""""))
    Debug.Print CStr(Len(""""""))
    Debug.Print "it's a:b"
End Sub
