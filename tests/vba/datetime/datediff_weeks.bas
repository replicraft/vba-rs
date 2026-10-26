' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT: 1
' EXPECT: 0
' EXPECT: 1
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(DateDiff("ww", DateSerial(2000, 1, 1), DateSerial(2000, 1, 2)))
    Debug.Print CStr(DateDiff("ww", DateSerial(2000, 1, 2), DateSerial(2000, 1, 8)))
    Debug.Print CStr(DateDiff("ww", DateSerial(2000, 1, 2), DateSerial(2000, 1, 9)))
    Debug.Print CStr(DateDiff("w", DateSerial(2000, 1, 3), DateSerial(2000, 1, 17)))
End Sub
