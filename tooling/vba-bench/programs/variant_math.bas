Public Sub Main()
    Dim v As Variant, w As Variant, i As Long
    v = 0: w = 1.5
    For i = 1 To 60000
        v = v + i
        w = w * 1.0001 + v / 7
        If i Mod 3 = 0 Then v = v - 1
    Next
    Debug.Print v; Int(w)
End Sub
