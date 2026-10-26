' INV: derived.harcsin
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 0.481212
Function HArcsin(X As Double) As Double
    HArcsin = Log(X + Sqr(X * X + 1))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArcsin(0.5), 6))
End Sub
