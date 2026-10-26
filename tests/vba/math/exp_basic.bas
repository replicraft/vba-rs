' INV: func.exp
' SOURCE: spec ms-vbal 6.1.2.10.1.4 Exp
' EXPECT: 1
' EXPECT: 2.71828182845905
Public Sub Main()
    Debug.Print CStr(Exp(0))
    Debug.Print CStr(Exp(1))
End Sub
