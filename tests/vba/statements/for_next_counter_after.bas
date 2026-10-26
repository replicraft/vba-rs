' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement counter after completion
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
    Next
    Debug.Print i
End Sub
