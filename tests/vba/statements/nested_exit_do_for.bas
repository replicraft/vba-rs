' INV: stmt.exit, stmt.do-loop, stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.7 Exit Do closest enclosing Do statement
' EXPECT:  6 
Public Sub Main()
    Dim i As Long, j As Long, n As Long
    For i = 1 To 3
        j = 0
        Do
            j = j + 1
            n = n + 1
            If j = 2 Then Exit Do
        Loop
    Next
    Debug.Print n
End Sub
