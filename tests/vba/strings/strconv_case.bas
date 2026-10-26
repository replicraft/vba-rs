' INV: func.strconv
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: HELLO WORLD
' EXPECT: hello
' EXPECT: Hello World
Public Sub Main()
    Debug.Print CStr(StrConv("hello world", vbUpperCase))
    Debug.Print CStr(StrConv("HELLO", vbLowerCase))
    Debug.Print CStr(StrConv("hello wORLD", vbProperCase))
End Sub
