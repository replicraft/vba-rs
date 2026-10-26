' INV: func.cstr, type.single
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 0.3333333
' EXPECT: 1.677722E+07
' EXPECT: 1E+10
' EXPECT: 0.1
Public Sub Main()
    Debug.Print CStr(CSng(1 / 3))
    Debug.Print CStr(CSng(16777216))
    Debug.Print CStr(CSng(1E10))
    Debug.Print CStr(CSng(0.1))
End Sub
