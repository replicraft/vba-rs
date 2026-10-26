' INV: derived.arcsin
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.523599
Function Arcsin(X As Double) As Double
    Arcsin = Atn(X / Sqr(-X * X + 1))
End Function
Public Sub Main()
    Debug.Print CStr(Round(Arcsin(0.5), 6))
End Sub
