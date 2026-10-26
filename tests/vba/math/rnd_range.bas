' INV: func.rnd
' SOURCE: derived Rnd returns a value in [0,1)
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
