' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
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
