' INV: func.instr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT: 2
' EXPECT: 0
' EXPECT: 0
' EXPECT: 3
Public Sub Main()
    Debug.Print CStr(InStr(2, "aaa", "a"))
    Debug.Print CStr(InStr(4, "aaa", "a"))
    Debug.Print CStr(InStr(1, "", ""))
    Debug.Print CStr(InStr(1, "abc", "c"))
End Sub
