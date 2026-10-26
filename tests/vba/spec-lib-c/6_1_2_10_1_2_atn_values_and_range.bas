' INV: vbal.6.1.2.10.1.2, func.atn
' SOURCE: spec ms-vbal 6.1.2.10.1.2 Atn arctangent in radians, range -pi/2 to pi/2
' EXPECT: zero=0
' EXPECT: quarter=True
' EXPECT: range=True
Public Sub Main()
    Dim pi As Double
    pi = 3.14159265358979
    Debug.Print "zero=" & CStr(Atn(0))
    Debug.Print "quarter=" & (Abs(Atn(1) * 4 - pi) < 0.000000000001)
    Debug.Print "range=" & (Atn(1E+300) <= pi / 2 + 0.000000000001 And Atn(-1E+300) >= -pi / 2 - 0.000000000001 And Atn(1E+300) > 1.5)
End Sub
