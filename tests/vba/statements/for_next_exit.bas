' INV: stmt.for-next, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.5 Exit For statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    For i = 1 To 10
        If i = 3 Then Exit For
    Next
    Debug.Print i
End Sub
