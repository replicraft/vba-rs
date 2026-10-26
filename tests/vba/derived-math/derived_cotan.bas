' INV: derived.cotan
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.830488
Function Cotan(X As Double) As Double
    Cotan = 1 / Tan(X)
End Function
Public Sub Main()
    Debug.Print CStr(Round(Cotan(0.5), 6))
End Sub
