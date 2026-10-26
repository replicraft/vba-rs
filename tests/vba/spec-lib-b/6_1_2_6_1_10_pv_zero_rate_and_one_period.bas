' INV: vbal.6.1.2.6.1.10, func.pv
' SOURCE: spec ms-vbal 6.1.2.6.1.10 PV sign convention, zero rate and single-period cases
' EXPECT: zero=1000
' EXPECT: one=100
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "zero=" & CStr(Round(PV(0, 10, -100, 0, 0), 6))
    Debug.Print "one=" & CStr(Round(PV(0.1, 1, -110, 0, 0), 6))
    Debug.Print "type=" & TypeName(PV(0, 10, -100, 0, 0))
End Sub
