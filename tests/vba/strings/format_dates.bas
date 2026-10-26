' INV: func.format, func.dateserial
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 2000-01-05
' EXPECT: 5/1/00
' EXPECT: March 5, 2000
' EXPECT: Sun
Public Sub Main()
    Debug.Print CStr(Format(DateSerial(2000, 1, 5), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2000, 1, 5), "d/m/yy"))
    Debug.Print CStr(Format(DateSerial(2000, 3, 5), "mmmm d, yyyy"))
    Debug.Print CStr(Format(DateSerial(2000, 3, 5), "ddd"))
End Sub
