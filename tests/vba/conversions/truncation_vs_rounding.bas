' INV: func.cint, func.int, func.fix, func.clng
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 2
' EXPECT: 4
' EXPECT: -2
' EXPECT: 2
' EXPECT: 2
' EXPECT: -1
' EXPECT: 0
' EXPECT: -4
Public Sub Main()
    Debug.Print CStr(CInt(2.5))
    Debug.Print CStr(CInt(3.5))
    Debug.Print CStr(CInt(-2.5))
    Debug.Print CStr(Int(2.9))
    Debug.Print CStr(Fix(2.9))
    Debug.Print CStr(Int(-0.5))
    Debug.Print CStr(Fix(-0.5))
    Debug.Print CStr(CLng(-3.5))
End Sub
