' INV: derived.harccos
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.316958
Function HArccos(X As Double) As Double
    HArccos = Log(X + Sqr(X * X - 1))
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArccos(2.0), 6))
End Sub
