' INV: derived.htan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.462117
Function HTan(X As Double) As Double
    HTan = (Exp(X) - Exp(-X)) / (Exp(X) + Exp(-X))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HTan(0.5), 6))
End Sub
