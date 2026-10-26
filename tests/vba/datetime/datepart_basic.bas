' INV: func.datepart
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 2000
' EXPECT: 1
' EXPECT: 3
' EXPECT: 65
' EXPECT: 5
' EXPECT: 1
' EXPECT: 11
' EXPECT: 13
' EXPECT: 5
' EXPECT: 9
Public Sub Main()
    Debug.Print CStr(DatePart("yyyy", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("q", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("m", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("y", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("d", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("w", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("ww", DateSerial(2000, 3, 5)))
    Debug.Print CStr(DatePart("h", TimeSerial(13, 5, 9)))
    Debug.Print CStr(DatePart("n", TimeSerial(13, 5, 9)))
    Debug.Print CStr(DatePart("s", TimeSerial(13, 5, 9)))
End Sub
