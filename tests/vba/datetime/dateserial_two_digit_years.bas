' INV: func.dateserial
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateserial-function
' EXPECT: 2000-01-01
' EXPECT: 2029-01-01
' EXPECT: 1930-01-01
' EXPECT: 1999-01-01
Public Sub Main()
    Debug.Print CStr(Format(DateSerial(0, 1, 1), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(29, 1, 1), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(30, 1, 1), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateSerial(99, 1, 1), "yyyy-mm-dd"))
End Sub
