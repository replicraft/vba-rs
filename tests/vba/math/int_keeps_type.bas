' INV: func.int, func.fix
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/int-fix-functions
' EXPECT: Double
' EXPECT: Single
' EXPECT: Integer
Public Sub Main()
    Debug.Print CStr(TypeName(Int(5.5)))
    Debug.Print CStr(TypeName(Fix(CSng(5.5))))
    Debug.Print CStr(TypeName(Int(5)))
End Sub
