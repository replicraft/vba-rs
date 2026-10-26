' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, String to numeric (whitespace, sign, exponent clause)
' EXPECT: plain=12
' EXPECT: spaces=12
' EXPECT: plus=5
' EXPECT: minus=-5
' EXPECT: signspace=-5
' EXPECT: e=100
' EXPECT: bigE=100
' EXPECT: d=100
' EXPECT: eplus=100
' EXPECT: frac exp=15
' EXPECT: dbl=1.5
' EXPECT: dblexp=100
' EXPECT: dblneg exp=0.05
Public Sub Main()
    Dim i As Integer
    Dim l As Long
    Dim x As Double
    i = "12": Debug.Print "plain=" & CStr(i)
    i = "  12  ": Debug.Print "spaces=" & CStr(i)
    i = "+5": Debug.Print "plus=" & CStr(i)
    i = "-5": Debug.Print "minus=" & CStr(i)
    i = "  - 5  ": Debug.Print "signspace=" & CStr(i)
    l = "1e2": Debug.Print "e=" & CStr(l)
    l = "1E2": Debug.Print "bigE=" & CStr(l)
    l = "1d2": Debug.Print "d=" & CStr(l)
    l = "1e+2": Debug.Print "eplus=" & CStr(l)
    l = "1.5e1": Debug.Print "frac exp=" & CStr(l)
    x = "1.5": Debug.Print "dbl=" & CStr(x)
    x = "1d2": Debug.Print "dblexp=" & CStr(x)
    x = "5e-2": Debug.Print "dblneg exp=" & CStr(x)
End Sub
