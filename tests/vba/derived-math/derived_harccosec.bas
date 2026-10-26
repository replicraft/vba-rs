' INV: derived.harccosec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.443635
Function HArccosec(X As Double) As Double
    HArccosec = Log((Sgn(X) * Sqr(X * X + 1) + 1) / X)
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArccosec(0.5), 6))
End Sub
