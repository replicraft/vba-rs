' INV: stmt.for-next
' SOURCE: spec ms-vbal 5.4.2.3 For statement zero iterations
' EXPECT:  5 
Public Sub Main()
    Dim i As Long
    For i = 5 To 1
        Debug.Print "never"
    Next
    Debug.Print i
End Sub
