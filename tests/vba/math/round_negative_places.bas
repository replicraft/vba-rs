' INV: func.round
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1234.57
' EXPECT: -1234.57
Public Sub Main()
    Debug.Print CStr(Round(1234.5678, 2))
    Debug.Print CStr(Round(-1234.5678, 2))
End Sub
