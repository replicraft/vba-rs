' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement negative step
' EXPECT:  10 
' EXPECT:  7 
' EXPECT:  4 
' EXPECT:  1 
Public Sub Main()
    Dim i As Long
    For i = 10 To 1 Step -3
        Debug.Print i
    Next
End Sub
