' INV: func.rnd
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd
' EXPECT: True
Public Sub Main()
    Dim i As Long, ok As Boolean
    ok = True
    For i = 1 To 100
        Dim x As Double
        x = Rnd
        If x < 0 Or x >= 1 Then ok = False
    Next
    Debug.Print ok
End Sub
