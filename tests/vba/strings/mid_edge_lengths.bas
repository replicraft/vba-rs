' INV: func.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-function
' EXPECT: 
' EXPECT: bc
' EXPECT: c
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Mid("abc", 2, 0))
    Debug.Print CStr(Mid("abc", 2, 100))
    Debug.Print CStr(Mid("abc", 3, 1))
    Debug.Print CStr(Mid("", 1))
End Sub
