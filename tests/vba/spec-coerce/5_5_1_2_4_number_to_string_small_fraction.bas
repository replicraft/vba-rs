' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a number with integer part 0 uses normal notation (up to 15 digits)
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: small fractions format as 1E-05
' NOTE: the text uses normal notation whenever the integer part has at most the maximum number of integral digits
' EXPECT: small=0.00001
Public Sub Main()
    Dim s As String
    Dim d As Double
    d = 0.00001
    s = d
    Debug.Print "small=" & s
End Sub
