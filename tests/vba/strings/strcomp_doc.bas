' INV: func.strcomp
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/strcomp-function
' EXPECT: 0
' EXPECT: -1
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(StrComp("ABCD", "abcd", 1))
    Debug.Print CStr(StrComp("ABCD", "abcd", 0))
    Debug.Print CStr(StrComp("abcd", "ABCD"))
End Sub
