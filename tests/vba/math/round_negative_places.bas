' INV: func.round
' SOURCE: spec ms-vbal 6.1.2.10.1.7 Round
' EXPECT: 1234.57
' EXPECT: -1234.57
Public Sub Main()
    Debug.Print CStr(Round(1234.5678, 2))
    Debug.Print CStr(Round(-1234.5678, 2))
End Sub
