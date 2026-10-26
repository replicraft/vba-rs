' INV: func.syd
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/syd-function
' EXPECT: 4090.91
' EXPECT: 409.09
Public Sub Main()
    Debug.Print CStr(Round(SYD(30000, 7500, 10, 1), 2))
    Debug.Print CStr(Round(SYD(30000, 7500, 10, 10), 2))
End Sub
