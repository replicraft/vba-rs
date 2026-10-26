' INV: derived.hcos
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.127626
Function HCos(X As Double) As Double
    HCos = (Exp(X) + Exp(-X)) / 2
End Function
Public Sub Main()
    Debug.Print CStr(Round(HCos(0.5), 6))
End Sub
