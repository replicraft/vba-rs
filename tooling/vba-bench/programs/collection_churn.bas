Public Sub Main()
    Dim c As Collection, i As Long, n As Long
    Set c = New Collection
    For i = 1 To 2000
        c.Add i, "a" & i
    Next
    ' Keyed inserts in the middle and removes by key keep shifting positions.
    For i = 1 To 1000
        c.Add i, "b" & i, "a" & (i * 2)
        n = n + c("a" & (i * 2 - 1))
        c.Remove "a" & i
        n = n + c("b" & i)
    Next
    For i = 1 To 1000
        c.Add i, "c" & i, , "b" & i
        n = n + c.Item("c" & i)
    Next
    Debug.Print n; c.Count
End Sub
