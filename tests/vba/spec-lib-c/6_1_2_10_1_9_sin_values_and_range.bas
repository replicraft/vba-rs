' INV: vbal.6.1.2.10.1.9, func.sin
' SOURCE: spec ms-vbal 6.1.2.10.1.9 Sin sine in radians, result in -1 to 1
' EXPECT: zero=0
' EXPECT: halfpi=True
' EXPECT: range=True
Public Sub Main()
    Dim pi As Double, x As Double, ok As Boolean
    pi = 3.14159265358979
    Debug.Print "zero=" & CStr(Sin(0))
    Debug.Print "halfpi=" & (Abs(Sin(pi / 2) - 1) < 0.000000000001)
    ok = True
    For x = -10 To 10 Step 0.37
        If Sin(x) < -1 Or Sin(x) > 1 Then ok = False
    Next x
    Debug.Print "range=" & ok
End Sub
