' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(DateDiff("yyyy", DateSerial(1999, 12, 31), DateSerial(2000, 1, 1)))
End Sub
