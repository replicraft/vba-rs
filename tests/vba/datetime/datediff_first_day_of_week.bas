' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT: 0
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(DateDiff("ww", DateSerial(2000, 1, 1), DateSerial(2000, 1, 2), vbMonday))
    Debug.Print CStr(DateDiff("ww", DateSerial(2000, 1, 2), DateSerial(2000, 1, 3), vbMonday))
End Sub
