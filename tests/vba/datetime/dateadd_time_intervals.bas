' INV: func.dateadd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateadd-function
' EXPECT: 2000-01-01 05:00:00
' EXPECT: 01:30:00
' EXPECT: 01:01:01
' EXPECT: 1999-12-31 23
Public Sub Main()
    Debug.Print CStr(Format(DateAdd("h", 5, DateSerial(2000, 1, 1)), "yyyy-mm-dd hh:nn:ss"))
    Debug.Print CStr(Format(DateAdd("n", 90, DateSerial(2000, 1, 1)), "hh:nn:ss"))
    Debug.Print CStr(Format(DateAdd("s", 3661, DateSerial(2000, 1, 1)), "hh:nn:ss"))
    Debug.Print CStr(Format(DateAdd("h", -1, DateSerial(2000, 1, 1)), "yyyy-mm-dd hh"))
End Sub
