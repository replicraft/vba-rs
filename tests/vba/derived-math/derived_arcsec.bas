' INV: derived.arcsec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 2.427868
Function Arcsec(X As Double) As Double
    Arcsec = Atn(X / Sqr(X * X - 1)) + Sgn((X) - 1) * (2 * Atn(1))
End Function
Public Sub Main()
    Debug.Print CStr(Round(Arcsec(2.0), 6))
End Sub
