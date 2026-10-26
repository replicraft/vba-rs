' INV: vbal.6.1.2.10.1.8, func.sgn
' SOURCE: spec ms-vbal 6.1.2.10.1.8 Sgn returns 1, 0 or -1 as an Integer
' EXPECT: 1 0 -1
' EXPECT: Integer
' EXPECT: 1 -1
Public Sub Main()
    Debug.Print CStr(Sgn(12.5)) & " " & CStr(Sgn(0)) & " " & CStr(Sgn(-0.001))
    Debug.Print TypeName(Sgn(3.5))
    Debug.Print CStr(Sgn(100000)) & " " & CStr(Sgn(-100000))
End Sub
