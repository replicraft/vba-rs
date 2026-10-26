Public Sub Main()
    Dim i As Long, j As Long, t As Long
    For i = 1 To 300
        For j = 1 To 1000
            t = (t + i * j) Mod 100003
        Next
    Next
    Debug.Print t
End Sub
