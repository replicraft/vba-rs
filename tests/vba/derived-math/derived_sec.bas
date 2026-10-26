' INV: derived.sec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.139494
Function Sec(X As Double) As Double
    Sec = 1 / Cos(X)
End Function
Public Sub Main()
    Debug.Print CStr(Round(Sec(0.5), 6))
End Sub
