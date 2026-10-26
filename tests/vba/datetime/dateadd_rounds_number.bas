' INV: func.dateadd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateadd-function
' EXPECT: 2000-01-03
' EXPECT: 2000-01-02
Public Sub Main()
    Debug.Print CStr(Format(DateAdd("d", 1.6, DateSerial(2000, 1, 1)), "yyyy-mm-dd"))
    Debug.Print CStr(Format(DateAdd("d", 1.4, DateSerial(2000, 1, 1)), "yyyy-mm-dd"))
End Sub
