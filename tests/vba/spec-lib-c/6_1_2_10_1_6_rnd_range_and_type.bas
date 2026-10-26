' INV: vbal.6.1.2.10.1.6, func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd returns a Single in [0,1)
' EXPECT: Single
' EXPECT: inrange=True
Public Sub Main()
    Dim i As Long, ok As Boolean, x As Variant
    x = Rnd
    Debug.Print TypeName(x)
    ok = True
    For i = 1 To 2000
        x = Rnd
        If x < 0 Or x >= 1 Then ok = False
    Next i
    Debug.Print "inrange=" & ok
End Sub
