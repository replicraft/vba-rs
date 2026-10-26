' INV: func.isnumeric
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(IsNumeric("123"))
    Debug.Print CStr(IsNumeric("1e3"))
    Debug.Print CStr(IsNumeric("&H10"))
    Debug.Print CStr(IsNumeric("abc"))
    Debug.Print CStr(IsNumeric(""))
    Debug.Print CStr(IsNumeric(Empty))
    Debug.Print CStr(IsNumeric(Null))
    Debug.Print CStr(IsNumeric(True))
    Debug.Print CStr(IsNumeric(" 5 "))
    Debug.Print CStr(IsNumeric(DateSerial(2000, 1, 1)))
End Sub
