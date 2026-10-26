' INV: func.log
' SOURCE: derived natural logarithm
' EXPECT: 0
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(Log(1))
    Debug.Print CStr(Round(Log(Exp(2)), 10))
End Sub
