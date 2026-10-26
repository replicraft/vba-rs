' INV: func.isdate
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(IsDate("2000-01-01"))
    Debug.Print CStr(IsDate("March 5, 2000"))
    Debug.Print CStr(IsDate("hello"))
    Debug.Print CStr(IsDate(DateSerial(2000, 1, 1)))
    Debug.Print CStr(IsDate(5))
    Debug.Print CStr(IsDate("13:45"))
    Debug.Print CStr(IsDate(""))
End Sub
