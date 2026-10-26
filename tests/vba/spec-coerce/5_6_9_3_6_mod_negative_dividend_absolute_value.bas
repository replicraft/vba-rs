' INV: vbal.5.6.9.3.6, op.mod
' SOURCE: spec ms-vbal 5.6.9.3.6 Mod Operator, the result is the absolute value of (left - truncated quotient * right)
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: Mod keeps the sign of the dividend (-7 Mod 4 is -3)
' NOTE: the text defines the result as an absolute value; the Learn documentation and real VBA keep the sign of the dividend (-7 Mod 4 is -3)
' EXPECT: -7 Mod 4=3
' EXPECT: -7 Mod -4=3
Public Sub Main()
    Dim a As Long, b As Long
    a = -7: b = 4
    Debug.Print "-7 Mod 4=" & CStr(a Mod b)
    a = -7: b = -4
    Debug.Print "-7 Mod -4=" & CStr(a Mod b)
End Sub
