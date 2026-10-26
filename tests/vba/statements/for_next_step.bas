' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
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
