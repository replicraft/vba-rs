' INV: vbal.6.1.2.10.1.4, func.exp
' SOURCE: spec ms-vbal 6.1.2.10.1.4 Exp raises e to a power
' EXPECT: zero=1
' EXPECT: one=True
' EXPECT: neg=True
Public Sub Main()
    Debug.Print "zero=" & CStr(Exp(0))
    Debug.Print "one=" & (Abs(Exp(1) - 2.71828182845905) < 0.000000000001)
    Debug.Print "neg=" & (Abs(Exp(-1) - 0.367879441171442) < 0.000000000001)
End Sub
