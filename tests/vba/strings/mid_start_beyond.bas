' INV: func.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-function
' EXPECT: 
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Mid("abc", 4))
    Debug.Print CStr(Mid("abc", 10, 2))
End Sub
