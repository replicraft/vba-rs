' INV: vbal.5.6.9.3, op.mul
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, a Single overflow with a Variant operand widens to the narrowest of Integer, Long, Double that holds the value
' EXPECT: Double
' EXPECT: True
Public Sub Main()
    Dim s As Single
    Dim v As Variant
    Dim r As Variant
    s = 3E+38
    v = CSng(100)
    r = s * v
    Debug.Print TypeName(r)
    Debug.Print CStr(r > 1E+39)
End Sub
