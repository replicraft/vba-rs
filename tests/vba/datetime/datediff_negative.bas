' INV: func.datediff
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datediff-function
' EXPECT: -60
Public Sub Main()
    Debug.Print CStr(DateDiff("d", DateSerial(2000, 3, 1), DateSerial(2000, 1, 1)))
End Sub
