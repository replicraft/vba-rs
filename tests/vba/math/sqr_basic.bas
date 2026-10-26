' INV: func.sqr
' SOURCE: derived square root
' EXPECT: 4
' EXPECT: True
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Sqr(16))
    Debug.Print CStr(Sqr(2) * Sqr(2) > 1.99)
    Debug.Print CStr(Sqr(0))
End Sub
