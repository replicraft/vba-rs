' INV: op.add, op.mul, type.integer, type.long, type.double
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: Integer
' EXPECT: Long
' EXPECT: Double
' EXPECT: Integer
' EXPECT: Single
' EXPECT: Currency
' EXPECT: Long
Public Sub Main()
    Debug.Print CStr(TypeName(1 + 1))
    Debug.Print CStr(TypeName(1 + 100000))
    Debug.Print CStr(TypeName(1 + 1.5))
    Debug.Print CStr(TypeName(CByte(1) + 1))
    Debug.Print CStr(TypeName(CSng(1) + 1))
    Debug.Print CStr(TypeName(CCur(1) + 1))
    Debug.Print CStr(TypeName(1& * 2))
End Sub
