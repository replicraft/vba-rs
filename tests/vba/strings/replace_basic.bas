' INV: func.replace
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' EXPECT: aXcaXc
' EXPECT: bbbbbb
Public Sub Main()
    Debug.Print CStr(Replace("abcabc", "b", "X"))
    Debug.Print CStr(Replace("aaa", "a", "bb"))
End Sub
