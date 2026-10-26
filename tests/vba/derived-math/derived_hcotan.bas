' INV: derived.hcotan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 2.163953
Function HCotan(X As Double) As Double
    HCotan = (Exp(X) + Exp(-X)) / (Exp(X) - Exp(-X))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HCotan(0.5), 6))
End Sub
