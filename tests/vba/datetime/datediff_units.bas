' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT: 60
' EXPECT: 10
' EXPECT: 1
' EXPECT: 1
' EXPECT: 24
' EXPECT: 90
' EXPECT: 65
Public Sub Main()
    Debug.Print CStr(DateDiff("d", DateSerial(2000, 1, 1), DateSerial(2000, 3, 1)))
    Debug.Print CStr(DateDiff("y", DateSerial(2000, 1, 1), DateSerial(2000, 1, 11)))
    Debug.Print CStr(DateDiff("m", DateSerial(2000, 1, 31), DateSerial(2000, 2, 1)))
    Debug.Print CStr(DateDiff("q", DateSerial(2000, 3, 31), DateSerial(2000, 4, 1)))
    Debug.Print CStr(DateDiff("h", DateSerial(2000, 1, 1), DateSerial(2000, 1, 2)))
    Debug.Print CStr(DateDiff("n", DateSerial(2000, 1, 1), DateSerial(2000, 1, 1) + TimeSerial(1, 30, 0)))
    Debug.Print CStr(DateDiff("s", DateSerial(2000, 1, 1), DateSerial(2000, 1, 1) + TimeSerial(0, 1, 5)))
End Sub
