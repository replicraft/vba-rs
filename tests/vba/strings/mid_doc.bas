' INV: func.mid
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/mid-function
' EXPECT: Mid
' EXPECT: Demo
' EXPECT: Function Demo
Public Sub Main()
    Debug.Print CStr(Mid("Mid Function Demo", 1, 3))
    Debug.Print CStr(Mid("Mid Function Demo", 14, 4))
    Debug.Print CStr(Mid("Mid Function Demo", 5))
End Sub
