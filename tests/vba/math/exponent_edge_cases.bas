' INV: op.pow
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1
' EXPECT: 0
' EXPECT: 64
' EXPECT: 2
' EXPECT: 0.01
Public Sub Main()
    Debug.Print CStr(0 ^ 0)
    Debug.Print CStr(0 ^ 2)
    Debug.Print CStr((-8) ^ 2)
    Debug.Print CStr(4 ^ 0.5)
    Debug.Print CStr(10 ^ -2)
End Sub
