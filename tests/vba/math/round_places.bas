' INV: func.round
' SOURCE: spec ms-vbal 6.1.2.10.1.7 Round
' EXPECT: 3.14
' EXPECT: 1234.6
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr(Round(3.14159, 2))
    Debug.Print CStr(Round(1234.5678, 1))
    Debug.Print CStr(Round(7))
End Sub
