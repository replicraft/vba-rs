' INV: op.intdiv, op.mod
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1073741823
' EXPECT: 7
' EXPECT: -214748364
' EXPECT: 3
' EXPECT: 3
Public Sub Main()
    Debug.Print CStr(2147483647 \ 2)
    Debug.Print CStr(2147483647 Mod 10)
    Debug.Print CStr(-2147483647 \ 10)
    Debug.Print CStr(5.5 \ 2)
    Debug.Print CStr(6.5 \ 2)
End Sub
