' INV: derived.hcosec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.919035
Function HCosec(X As Double) As Double
    HCosec = 2 / (Exp(X) - Exp(-X))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HCosec(0.5), 6))
End Sub
