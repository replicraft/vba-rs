' INV: vbal.6.1.2.6.1.5, func.mirr
' SOURCE: spec ms-vbal 6.1.2.6.1.5 MIRR of -100 then 110 is 10 percent whatever the two rates
' EXPECT: mirr=0.1
' EXPECT: type=Double
Public Sub Main()
    Dim v(1) As Double
    v(0) = -100
    v(1) = 110
    Debug.Print "mirr=" & CStr(Round(MIRR(v, 0.12, 0.15), 4))
    Debug.Print "type=" & TypeName(MIRR(v, 0.12, 0.15))
End Sub
