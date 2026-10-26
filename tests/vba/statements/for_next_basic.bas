' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement
' EXPECT:  15 
Public Sub Main()
    Dim i As Long, s As Long
    For i = 1 To 5
        s = s + i
    Next i
    Debug.Print s
End Sub
