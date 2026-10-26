' INV: func.instr
' SOURCE: spec ms-vbal 6.1.2.11.1.14 InStr Compare argument (text and binary)
' EXPECT: 2
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(InStr(1, "ABC", "b", vbTextCompare))
    Debug.Print CStr(InStr(1, "ABC", "b", vbBinaryCompare))
End Sub
