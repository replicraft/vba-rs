' INV: vbal.6.1.2.10.1.3, func.cos
' SOURCE: spec ms-vbal 6.1.2.10.1.3 Cos cosine in radians, result in -1 to 1
' EXPECT: zero=1
' EXPECT: pi=True
' EXPECT: range=True
Public Sub Main()
    Dim pi As Double, x As Double, ok As Boolean
    pi = 3.14159265358979
    Debug.Print "zero=" & CStr(Cos(0))
    Debug.Print "pi=" & (Abs(Cos(pi) + 1) < 0.000000000001)
    ok = True
    For x = -10 To 10 Step 0.37
        If Cos(x) < -1 Or Cos(x) > 1 Then ok = False
    Next x
    Debug.Print "range=" & ok
End Sub
