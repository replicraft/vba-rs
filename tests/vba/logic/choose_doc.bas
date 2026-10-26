' INV: func.choose
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/choose-function
' EXPECT: three
' EXPECT: Speedy
' EXPECT: 20
Public Sub Main()
    Debug.Print CStr(Choose(3, "one", "two", "three"))
    Debug.Print CStr(Choose(1, "Speedy", "United", "Federal"))
    Debug.Print CStr(Choose(2.4, 10, 20, 30))
End Sub
