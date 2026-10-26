' INV: derived.cosec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 2.08583
Function Cosec(X As Double) As Double
    Cosec = 1 / Sin(X)
End Function
Public Sub Main()
    Debug.Print CStr(Round(Cosec(0.5), 6))
End Sub
