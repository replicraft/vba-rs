' INV: func.oct
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/oct-function
' EXPECT: 4
' EXPECT: 10
' EXPECT: 713
Public Sub Main()
    Debug.Print CStr(Oct(4))
    Debug.Print CStr(Oct(8))
    Debug.Print CStr(Oct(459))
End Sub
