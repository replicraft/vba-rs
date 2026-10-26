' INV: derived.arccosec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.857072
Function Arccosec(X As Double) As Double
    Arccosec = Atn(X / Sqr(X * X - 1)) + (Sgn(X) - 1) * (2 * Atn(1))
End Function
Public Sub Main()
    Debug.Print CStr(Round(Arccosec(2.0), 6))
End Sub
