' INV: vbal.6.1.2.10.1.5, func.log
' SOURCE: spec ms-vbal 6.1.2.10.1.5 Log is the natural logarithm
' EXPECT: one=0
' EXPECT: e=True
' EXPECT: inverse=True
Public Sub Main()
    Debug.Print "one=" & CStr(Log(1))
    Debug.Print "e=" & (Abs(Log(2.71828182845905) - 1) < 0.000000000001)
    Debug.Print "inverse=" & (Abs(Log(Exp(3)) - 3) < 0.000000000001)
End Sub
