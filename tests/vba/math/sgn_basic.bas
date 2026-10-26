' INV: func.sgn
' SOURCE: derived sign
' EXPECT: -1
' EXPECT: 0
' EXPECT: 1
Public Sub Main()
    Debug.Print CStr(Sgn(-3))
    Debug.Print CStr(Sgn(0))
    Debug.Print CStr(Sgn(2.5))
End Sub
