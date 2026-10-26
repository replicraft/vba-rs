' INV: func.log
' SOURCE: spec ms-vbal 6.1.2.10.1.5 Log
' EXPECT: 0
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(Log(1))
    Debug.Print CStr(Round(Log(Exp(2)), 10))
End Sub
