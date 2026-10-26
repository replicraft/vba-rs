' INV: vbal.6.1.2.6.1.8, func.pmt
' SOURCE: spec ms-vbal 6.1.2.6.1.8 Pmt sign convention, optional FV and Due omitted
' EXPECT: zero=-100
' EXPECT: one=-110
' EXPECT: due=-100
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "zero=" & CStr(Round(Pmt(0, 10, 1000), 6))
    Debug.Print "one=" & CStr(Round(Pmt(0.1, 1, 100), 6))
    Debug.Print "due=" & CStr(Round(Pmt(0.1, 1, 100, 0, 1), 6))
    Debug.Print "type=" & TypeName(Pmt(0, 10, 1000))
End Sub
