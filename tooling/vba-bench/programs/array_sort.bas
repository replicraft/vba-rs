Public Sub Main()
    Dim a(1 To 400) As Long, i As Long, j As Long, t As Long, seed As Long
    seed = 12345
    For i = 1 To 400
        seed = (seed * 1103 + 12345) Mod 65536
        a(i) = seed
    Next
    For i = 1 To 399
        For j = 1 To 400 - i
            If a(j) > a(j + 1) Then
                t = a(j): a(j) = a(j + 1): a(j + 1) = t
            End If
        Next
    Next
    Dim d() As Double
    ReDim d(0 To 20000)
    For i = 0 To 20000
        d(i) = i * 0.5
    Next
    Dim sum As Double
    For i = 0 To 20000
        sum = sum + d(i)
    Next
    Debug.Print a(1); a(400); sum
End Sub
