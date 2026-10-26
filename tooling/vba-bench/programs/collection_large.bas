Public Sub Main()
    Dim c As Collection, i As Long, n As Long, v As Variant
    Set c = New Collection
    For i = 1 To 5000
        c.Add i, "key" & i
    Next
    For i = 5000 To 1 Step -1
        n = n + c.Item("KEY" & i)
    Next
    For i = 1 To 5000 Step 7
        n = n + c("key" & i)
    Next
    For Each v In c
        n = n + v
    Next
    Debug.Print n; c.Count
End Sub
