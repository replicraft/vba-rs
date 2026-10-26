' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement
' EXPECT:  3 
' EXPECT:  6 
' EXPECT:  9 
' EXPECT:  12 
Public Sub Main()
    Dim i As Long
    For i = 1 To 10
        i = i + 2
        Debug.Print i
    Next
End Sub
