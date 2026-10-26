' INV: stmt.for-next
' SOURCE: derived nested loops
' EXPECT:  11 
' EXPECT:  12 
' EXPECT:  21 
' EXPECT:  22 
Public Sub Main()
    Dim i As Long, j As Long
    For i = 1 To 2
        For j = 1 To 2
            Debug.Print i * 10 + j
        Next j
    Next i
End Sub
