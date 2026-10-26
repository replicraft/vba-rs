' INV: vbal.6.1.2.6.1.4, func.irr
' SOURCE: spec ms-vbal 6.1.2.6.1.4 IRR of cash flows -100 then 110 is 10 percent
' EXPECT: irr=0.1
' EXPECT: guess=0.1
' EXPECT: type=Double
Public Sub Main()
    Dim v(1) As Double
    v(0) = -100
    v(1) = 110
    Debug.Print "irr=" & CStr(Round(IRR(v), 4))
    Debug.Print "guess=" & CStr(Round(IRR(v, 0.2), 4))
    Debug.Print "type=" & TypeName(IRR(v))
End Sub
