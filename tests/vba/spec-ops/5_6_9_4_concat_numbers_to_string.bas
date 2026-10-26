' INV: vbal.5.6.9.4, op.concat
' SOURCE: spec ms-vbal 5.6.9.4 & operator, numeric operands coerced to String
' EXPECT: 12
' EXPECT: String
' EXPECT: 3x
' EXPECT: String
' EXPECT: x
Public Sub Main()
    Dim v As Variant, w As Variant
    Debug.Print "" & 1 & 2
    Debug.Print TypeName(1 & 2)
    Debug.Print CLng(3) & "x"
    v = 5
    w = 6
    Debug.Print TypeName(v & w)
    Dim e As Variant
    Debug.Print e & "x"
End Sub
