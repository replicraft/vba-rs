' INV: func.hex
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/hex-function
' EXPECT: 5
' EXPECT: A
' EXPECT: 1CB
' EXPECT: FF
Public Sub Main()
    Debug.Print CStr(Hex(5))
    Debug.Print CStr(Hex(10))
    Debug.Print CStr(Hex(459))
    Debug.Print CStr(Hex(255))
End Sub
