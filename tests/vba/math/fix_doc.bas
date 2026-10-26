' INV: func.fix
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/int-fix-functions
' EXPECT: 99
' EXPECT: -99
' EXPECT: -99
Public Sub Main()
    Debug.Print CStr(Fix(99.2))
    Debug.Print CStr(Fix(-99.8))
    Debug.Print CStr(Fix(-99.2))
End Sub
