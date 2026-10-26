' INV: func.dateserial
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateserial-function
' EXPECT: 2001-01-01
' EXPECT: 2000-02-04
' EXPECT: 1999-12-01
' EXPECT: 2000-02-29
Public Sub Main()
    Debug.Print CStr(Format(DateSerial(2000, 13, 1), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2000, 1, 35), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2000, 0, 1), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(2000, 3, 0), "yyyy-mm-dd"))
End Sub
