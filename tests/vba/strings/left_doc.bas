' INV: func.left
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/left-function
' EXPECT: H
' EXPECT: Hello W
' EXPECT: Hello World
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Left("Hello World", 1))
    Debug.Print CStr(Left("Hello World", 7))
    Debug.Print CStr(Left("Hello World", 20))
    Debug.Print CStr(Left("Hello World", 0))
End Sub
