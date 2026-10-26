' INV: func.format
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1,234.50
' EXPECT: 50%
' EXPECT: 005
' EXPECT: 3.14
' EXPECT: -5.0
' EXPECT: 0.0
Public Sub Main()
    Debug.Print CStr(Format(1234.5, "#,##0.00"))
    Debug.Print CStr(Format(0.5, "0%"))
    Debug.Print CStr(Format(5, "000"))
    Debug.Print CStr(Format(3.14159, "0.00"))
    Debug.Print CStr(Format(-5, "0.0"))
    Debug.Print CStr(Format(0, "0.0"))
End Sub
