' INV: func.int
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/int-fix-functions
' EXPECT: 99
' EXPECT: -100
' EXPECT: -100
Public Sub Main()
    Debug.Print CStr(Int(99.8))
    Debug.Print CStr(Int(-99.8))
    Debug.Print CStr(Int(-99.2))
End Sub
