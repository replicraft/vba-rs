' INV: func.dateadd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateadd-function
' EXPECT: 2001-02-28
' EXPECT: 2000-04-15
' EXPECT: 2000-02-29
' EXPECT: 2000-01-11
' EXPECT: 2000-02-04
' EXPECT: 2000-01-04
' EXPECT: 2000-01-15
Public Sub Main()
    Debug.Print CStr(Format(DateAdd("yyyy", 1, DateSerial(2000, 2, 29)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("q", 1, DateSerial(2000, 1, 15)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("m", -1, DateSerial(2000, 3, 31)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("y", 10, DateSerial(2000, 1, 1)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("d", 10, DateSerial(2000, 1, 25)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("w", 3, DateSerial(2000, 1, 1)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("ww", 2, DateSerial(2000, 1, 1)), "yyyy-mm-dd"))
End Sub
