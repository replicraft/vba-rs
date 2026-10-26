Public Sub Main()
    Dim s As String, i As Long, n As Long
    For i = 1 To 20000
        s = s & Chr(65 + i Mod 26)
        If Len(s) > 3000 Then s = Mid(s, 1500)
    Next
    For i = 1 To 20000
        n = n + Len(Left(s, i Mod 50) & Right(s, 7) & CStr(i))
    Next
    Debug.Print Len(s); n
End Sub
