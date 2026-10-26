Public Sub Main()
    Dim c As New Collection, i As Long, n As Long, v As Variant
    For i = 1 To 3000
        c.Add i * 2, "k" & i
    Next
    For i = 1 To 3000
        n = n + c.Item("k" & i) + c(i)
    Next
    For Each v In c
        n = n + v
    Next
    Do While c.Count > 1000
        c.Remove 1
    Loop
    Debug.Print n; c.Count
End Sub
