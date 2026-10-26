' INV: func.right
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/right-function
' EXPECT: d
' EXPECT:  World
' EXPECT: Hello World
Public Sub Main()
    Debug.Print CStr(Right("Hello World", 1))
    Debug.Print CStr(Right("Hello World", 6))
    Debug.Print CStr(Right("Hello World", 20))
End Sub
