' INV: func.hex
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/hex-function
' EXPECT: FFFF
' EXPECT: FFFFFFFF
' EXPECT: 0
' EXPECT: 2
' EXPECT: 4
Public Sub Main()
    Debug.Print CStr(Hex(-1))
    Debug.Print CStr(Hex(-1&))
    Debug.Print CStr(Hex(Empty))
    Debug.Print CStr(Hex(2.5))
    Debug.Print CStr(Hex(3.5))
End Sub
