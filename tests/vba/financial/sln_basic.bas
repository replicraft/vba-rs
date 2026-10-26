' INV: func.sln
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/sln-function
' EXPECT: 2250
' EXPECT: 250
Public Sub Main()
    Debug.Print CStr(SLN(30000, 7500, 10))
    Debug.Print CStr(SLN(1000, 0, 4))
End Sub
