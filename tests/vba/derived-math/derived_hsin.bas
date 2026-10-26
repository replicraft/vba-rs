' INV: derived.hsin
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.521095
Function HSin(X As Double) As Double
    HSin = (Exp(X) - Exp(-X)) / 2
End Function
Public Sub Main()
    Debug.Print CStr(Round(HSin(0.5), 6))
End Sub
