' INV: func.hex, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens (hex and octal literals) and 6.1.2.3.1.17 Hex
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
