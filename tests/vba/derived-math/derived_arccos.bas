' INV: derived.arccos
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.047198
Function Arccos(X As Double) As Double
    Arccos = Atn(-X / Sqr(-X * X + 1)) + 2 * Atn(1)
End Function
Public Sub Main()
    Debug.Print CStr(Round(Arccos(0.5), 6))
End Sub
