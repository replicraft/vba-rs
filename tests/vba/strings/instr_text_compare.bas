' INV: func.instr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT: 2
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(InStr(1, "ABC", "b", vbTextCompare))
    Debug.Print CStr(InStr(1, "ABC", "b", vbBinaryCompare))
End Sub
