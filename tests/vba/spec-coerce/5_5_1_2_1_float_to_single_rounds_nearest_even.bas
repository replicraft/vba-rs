' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, Double to Single rounds to nearest representable using Banker's rounding; tiny values become 0
' EXPECT: 16777217=16777216
' EXPECT: 16777219=16777220
' EXPECT: tiny=0
' EXPECT: exact=0.5
Public Sub Main()
    Dim s As Single
    Dim d As Double
    d = 16777217#
    s = d
    Debug.Print "16777217=" & CStr(CDbl(s))
    d = 16777219#
    s = d
    Debug.Print "16777219=" & CStr(CDbl(s))
    d = 1E-300
    s = d
    Debug.Print "tiny=" & CStr(CDbl(s))
    d = 0.5
    s = d
    Debug.Print "exact=" & CStr(CDbl(s))
End Sub
