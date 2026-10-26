' INV: func.sgn
' SOURCE: spec ms-vbal 6.1.2.10.1.8 Sgn
' EXPECT: -1
' EXPECT: 0
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(Sgn(-3))
    Debug.Print CStr(Sgn(0))
    Debug.Print CStr(Sgn(2.5))
End Sub
