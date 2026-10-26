' INV: derived.arccotan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 2.034444
Function Arccotan(X As Double) As Double
    Arccotan = Atn(X) + 2 * Atn(1)
End Function
Public Sub Main()
    Debug.Print CStr(Round(Arccotan(0.5), 6))
End Sub
