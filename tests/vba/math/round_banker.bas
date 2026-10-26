' INV: func.round
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/round-function
' EXPECT: 2
' EXPECT: 4
' EXPECT: -2
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Round(2.5))
    Debug.Print CStr(Round(3.5))
    Debug.Print CStr(Round(-2.5))
    Debug.Print CStr(Round(0.5))
End Sub
