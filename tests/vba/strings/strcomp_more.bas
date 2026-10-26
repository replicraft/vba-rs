' INV: func.strcomp
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/strcomp-function
' EXPECT: 0
' EXPECT: -1
' EXPECT: 1
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(StrComp("a", "a"))
    Debug.Print CStr(StrComp("a", "b"))
    Debug.Print CStr(StrComp("b", "a"))
    Debug.Print CStr(StrComp("", ""))
End Sub
