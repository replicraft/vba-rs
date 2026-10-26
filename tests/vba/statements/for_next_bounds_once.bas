' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement bounds evaluated once
' EXPECT:  1 
Dim g As Long
Function Limit() As Long
    g = g + 1
    Limit = 3
End Function
Public Sub Main()
    Dim i As Long
    For i = 1 To Limit()
    Next
    Debug.Print g
End Sub
