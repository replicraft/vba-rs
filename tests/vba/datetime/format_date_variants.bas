' INV: func.format
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: Monday, December 25, 2000
' EXPECT: 1/2/00
' EXPECT: 01/02/2000
' EXPECT: 12:05 AM
' EXPECT: 12:00:00
Public Sub Main()
    Debug.Print CStr(Format(DateSerial(2000, 12, 25), "dddd, mmmm d, yyyy"))
    Debug.Print CStr(Format(DateSerial(2000, 1, 2), "m/d/yy"))
    Debug.Print CStr(Format(DateSerial(2000, 1, 2), "mm/dd/yyyy"))
    Debug.Print CStr(Format(TimeSerial(0, 5, 0), "h:mm AM/PM"))
    Debug.Print CStr(Format(TimeSerial(12, 0, 0), "hh:mm:ss"))
End Sub
