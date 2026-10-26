' INV: func.round
' SOURCE: derived numdecimalplaces
' EXPECT: 3.14
' EXPECT: 1234.6
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr(Round(3.14159, 2))
    Debug.Print CStr(Round(1234.5678, 1))
    Debug.Print CStr(Round(7))
End Sub
