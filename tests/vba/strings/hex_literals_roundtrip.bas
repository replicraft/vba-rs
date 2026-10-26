' INV: func.hex, lit.hex
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/hex-function
' EXPECT: 255
' EXPECT: 16
' EXPECT: 7FFF
' EXPECT: 8
Public Sub Main()
    Debug.Print CStr(&HFF)
    Debug.Print CStr(&H10)
    Debug.Print CStr(Hex(&H7FFF))
    Debug.Print CStr(&O10)
End Sub
