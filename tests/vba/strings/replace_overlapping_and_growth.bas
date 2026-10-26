' INV: func.replace
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' EXPECT: bb
' EXPECT: abcabc
' EXPECT: abc
' EXPECT: yy
Public Sub Main()
    Debug.Print CStr(Replace("aaaa", "aa", "b"))
    Debug.Print CStr(Replace("abc", "abc", "abcabc"))
    Debug.Print CStr(Replace("a.b.c", ".", ""))
    Debug.Print CStr(Replace("x", "x", "yy", 1, 1))
End Sub
