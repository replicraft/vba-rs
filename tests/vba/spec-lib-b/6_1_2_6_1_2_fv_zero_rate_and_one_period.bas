' INV: vbal.6.1.2.6.1.2, func.fv
' SOURCE: spec ms-vbal 6.1.2.6.1.2 FV sign convention, zero rate and single-period cases
' EXPECT: zero=1000
' EXPECT: lump=110
' EXPECT: begin=110
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "zero=" & CStr(Round(FV(0, 10, -100, 0, 0), 6))
    Debug.Print "lump=" & CStr(Round(FV(0.1, 1, 0, -100, 0), 6))
    Debug.Print "begin=" & CStr(Round(FV(0.1, 1, -100, 0, 1), 6))
    Debug.Print "type=" & TypeName(FV(0, 10, -100, 0, 0))
End Sub
