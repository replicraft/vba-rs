' INV: derived.hsec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.886819
Function HSec(X As Double) As Double
    HSec = 2 / (Exp(X) + Exp(-X))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HSec(0.5), 6))
End Sub
