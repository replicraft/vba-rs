' INV: func.round
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/round-function
' EXPECT: 0.1234
' EXPECT: 0.1234
' EXPECT: 0.1236
' EXPECT: 0.1236
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Round(0.12335, 4))
    Debug.Print CStr(Round(0.12345, 4))
    Debug.Print CStr(Round(0.12355, 4))
    Debug.Print CStr(Round(0.12365, 4))
    Debug.Print CStr(Round(0.00005, 4))
End Sub
