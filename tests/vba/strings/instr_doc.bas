' INV: func.instr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT: 6
' EXPECT: 9
' EXPECT: 9
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(InStr(4, "XXpXXpXXPXXP", "P", 1))
    Debug.Print CStr(InStr(1, "XXpXXpXXPXXP", "P", 0))
    Debug.Print CStr(InStr("XXpXXpXXPXXP", "P"))
    Debug.Print CStr(InStr(1, "XXpXXpXXPXXP", "W"))
End Sub
