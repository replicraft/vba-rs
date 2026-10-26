' INV: derived.harctan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.549306
Function HArctan(X As Double) As Double
    HArctan = Log((1 + X) / (1 - X)) / 2
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArctan(0.5), 6))
End Sub
