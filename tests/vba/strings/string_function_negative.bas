' INV: func.string
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 5
Public Sub Main()
    Debug.Print CStr(Len(String(5, "ab")))
End Sub
