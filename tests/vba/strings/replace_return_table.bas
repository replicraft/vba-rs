' INV: func.replace
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' EXPECT: 
' EXPECT: abc
' EXPECT: acac
' EXPECT: 
' EXPECT: abcabc
Public Sub Main()
    Debug.Print CStr(Replace("", "a", "b"))
    Debug.Print CStr(Replace("abc", "", "x"))
    Debug.Print CStr(Replace("abcabc", "b", ""))
    Debug.Print CStr(Replace("abc", "b", "x", 4))
    Debug.Print CStr(Replace("abcabc", "b", "x", 1, 0))
End Sub
