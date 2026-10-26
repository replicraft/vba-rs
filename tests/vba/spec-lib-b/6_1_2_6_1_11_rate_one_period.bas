' INV: vbal.6.1.2.6.1.11, func.rate
' SOURCE: spec ms-vbal 6.1.2.6.1.11 Rate iterates to the per-period rate; optional FV, Due, Guess omitted
' EXPECT: one=0.1
' EXPECT: guess=0.1
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "one=" & CStr(Round(Rate(1, -110, 100), 4))
    Debug.Print "guess=" & CStr(Round(Rate(1, -110, 100, 0, 0, 0.05), 4))
    Debug.Print "type=" & TypeName(Rate(1, -110, 100))
End Sub
