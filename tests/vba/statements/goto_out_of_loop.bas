' INV: stmt.goto, stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.12 GoTo statement, 5.4.2.3 For statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    For i = 1 To 10
        If i = 4 Then GoTo Done
    Next
Done:
    Debug.Print i
End Sub
