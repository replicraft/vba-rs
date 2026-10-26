' INV: derived.harccotan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.549306
Function HArccotan(X As Double) As Double
    HArccotan = Log((X + 1) / (X - 1)) / 2
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArccotan(2.0), 6))
End Sub
