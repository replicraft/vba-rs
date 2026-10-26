' INV: func.instr
' SOURCE: spec ms-vbal 6.1.2.11.1.14 InStr position of first occurrence, Compare argument
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
