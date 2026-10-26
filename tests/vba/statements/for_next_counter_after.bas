' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    For i = 1 To 3
    Next
    Debug.Print i
End Sub
