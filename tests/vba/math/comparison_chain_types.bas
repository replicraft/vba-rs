' INV: op.eq, op.lt
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 = 1#)
    Debug.Print CStr(CCur(0.1) = 0.1)
    Debug.Print CStr(CSng(0.1) = 0.1)
    Debug.Print CStr("10" < "9")
    Debug.Print CStr(10 < 9)
    Debug.Print CStr(Empty = 0)
    Debug.Print CStr(Empty = "")
End Sub
