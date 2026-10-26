' INV: func.typename
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: Integer
' EXPECT: Long
' EXPECT: Double
' EXPECT: Single
' EXPECT: Currency
' EXPECT: Byte
' EXPECT: Boolean
' EXPECT: String
' EXPECT: Date
' EXPECT: Null
' EXPECT: Empty
' EXPECT: Decimal
' EXPECT: Long
Public Sub Main()
    Debug.Print CStr(TypeName(1))
    Debug.Print CStr(TypeName(70000))
    Debug.Print CStr(TypeName(1.5))
    Debug.Print CStr(TypeName(CSng(1)))
    Debug.Print CStr(TypeName(CCur(1)))
    Debug.Print CStr(TypeName(CByte(1)))
    Debug.Print CStr(TypeName(True))
    Debug.Print CStr(TypeName("s"))
    Debug.Print CStr(TypeName(Date))
    Debug.Print CStr(TypeName(Null))
    Debug.Print CStr(TypeName(Empty))
    Debug.Print CStr(TypeName(CDec(1)))
    Debug.Print CStr(TypeName(1&))
End Sub
