Public Sub Main()
    Dim c As Collection, i As Long, n As Long, v As Variant, round As Long
    For round = 1 To 30
        Set c = New Collection
        For i = 1 To 150
            c.Add i * 2, "k" & i
        Next
        For i = 1 To 150
            n = n + c.Item("k" & i) + c(i)
        Next
        For Each v In c
            n = n + v
        Next
        Do While c.Count > 50
            c.Remove 1
        Loop
    Next
    Debug.Print n; c.Count
End Sub
