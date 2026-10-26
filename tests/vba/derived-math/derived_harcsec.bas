' INV: derived.harcsec
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1.316958
Function HArcsec(X As Double) As Double
    HArcsec = Log((Sqr(-X * X + 1) + 1) / X)
End Function
Public Sub Main()
    Debug.Print CStr(Round(HArcsec(0.5), 6))
End Sub
