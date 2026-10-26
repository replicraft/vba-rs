' INV: func.int
' SOURCE: spec ms-vbal 6.1.2.3.1.18 Int
' EXPECT: 99
' EXPECT: -100
' EXPECT: -100
Public Sub Main()
    Debug.Print CStr(Int(99.8))
    Debug.Print CStr(Int(-99.8))
    Debug.Print CStr(Int(-99.2))
End Sub
