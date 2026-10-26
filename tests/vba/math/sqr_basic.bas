' INV: func.sqr
' SOURCE: spec ms-vbal 6.1.2.10.1.10 Sqr
' EXPECT: 4
' EXPECT: True
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Sqr(16))
    Debug.Print CStr(Sqr(2) * Sqr(2) > 1.99)
    Debug.Print CStr(Sqr(0))
End Sub
