' INV: vbal.6.1.2.6.1.3, vbal.6.1.2.6.1.9, func.ipmt, func.ppmt
' SOURCE: spec ms-vbal 6.1.2.6.1.3 IPmt interest and 6.1.2.6.1.9 PPmt principal of a one-period loan
' EXPECT: ipmt=-10
' EXPECT: ppmt=-100
' EXPECT: sum=-110
' EXPECT: type=Double
Public Sub Main()
    Debug.Print "ipmt=" & CStr(Round(IPmt(0.1, 1, 1, 100), 6))
    Debug.Print "ppmt=" & CStr(Round(PPmt(0.1, 1, 1, 100), 6))
    Debug.Print "sum=" & CStr(Round(IPmt(0.1, 1, 1, 100) + PPmt(0.1, 1, 1, 100), 6))
    Debug.Print "type=" & TypeName(IPmt(0.1, 1, 1, 100))
End Sub
