' INV: func.instr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT: 0
' EXPECT: 3
' EXPECT: 1
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(InStr("", "a"))
    Debug.Print CStr(InStr(3, "abc", ""))
    Debug.Print CStr(InStr("abc", ""))
    Debug.Print CStr(InStr(5, "abc", "a"))
End Sub
