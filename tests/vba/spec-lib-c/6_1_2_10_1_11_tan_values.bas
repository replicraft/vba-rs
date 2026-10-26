' INV: vbal.6.1.2.10.1.11, func.tan
' SOURCE: spec ms-vbal 6.1.2.10.1.11 Tan tangent in radians
' EXPECT: zero=0
' EXPECT: quarterpi=True
' EXPECT: odd=True
Public Sub Main()
    Dim pi As Double
    pi = 3.14159265358979
    Debug.Print "zero=" & CStr(Tan(0))
    Debug.Print "quarterpi=" & (Abs(Tan(pi / 4) - 1) < 0.000000000001)
    Debug.Print "odd=" & (Abs(Tan(-0.5) + Tan(0.5)) < 0.000000000001)
End Sub
