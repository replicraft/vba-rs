' INV: vbal.6.1.2.11.1.14, func.instr
' SOURCE: spec ms-vbal 6.1.2.11.1.14 InStr omitted Compare uses the Option Compare setting of the calling module
' EXPECT: 2
' EXPECT: 2
Option Compare Text
Public Sub Main()
    Debug.Print CStr(InStr("ABC", "b"))
    Debug.Print CStr(InStr(1, "ABC", "b"))
End Sub
