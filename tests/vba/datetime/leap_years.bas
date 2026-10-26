' INV: func.dateserial
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 2000-02-29
' EXPECT: 1900-03-01
' EXPECT: 2004-02-29
' EXPECT: 2100-02-28
Public Sub Main()
    Debug.Print CStr(Format(DateSerial(2000, 2, 29), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(1900, 2, 29), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2004, 3, 1) - 1, "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2100, 3, 1) - 1, "yyyy-mm-dd"))
End Sub
