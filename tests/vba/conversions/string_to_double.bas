' INV: func.cdbl, func.val
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 150
' EXPECT: 12
' EXPECT: 255
' EXPECT: 150
' EXPECT: 1234.5
Public Sub Main()
    Debug.Print CStr(CDbl("1.5e2"))
    Debug.Print CStr(CDbl("  12  "))
    Debug.Print CStr(CDbl("&HFF"))
    Debug.Print CStr(Val("1.5e2"))
    Debug.Print CStr(CDbl("1,234.5"))
End Sub
