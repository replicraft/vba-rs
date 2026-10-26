' INV: func.dateadd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dateadd-function
' EXPECT: 1995-02-28
' EXPECT: 1996-02-29
Public Sub Main()
    Debug.Print Format(DateAdd("m", 1, DateSerial(1995, 1, 31)), "yyyy-mm-dd")
    Debug.Print Format(DateAdd("m", 1, DateSerial(1996, 1, 31)), "yyyy-mm-dd")
End Sub
