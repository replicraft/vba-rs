' INV: func.format
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: HELLO
' EXPECT: hello
' EXPECT:   abc
Public Sub Main()
    Debug.Print CStr(Format("hello", ">"))
    Debug.Print CStr(Format("HELLO", "<"))
    Debug.Print CStr(Format("abc", "@@@@@"))
End Sub
