' INV: func.replace
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/replace-function
' EXPECT: caXc
' EXPECT: aXaXabab
Public Sub Main()
    Debug.Print CStr(Replace("abcabc", "b", "X", 3))
    Debug.Print CStr(Replace("abababab", "b", "X", 1, 2))
End Sub
