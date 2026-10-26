' INV: func.hex
' SOURCE: spec ms-vbal 6.1.2.3.1.17 Hex of positive values
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
