' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
' EXPECT:  5 
Public Sub Main()
    Dim i As Long
    For i = 5 To 1
        Debug.Print "never"
    Next
    Debug.Print i
End Sub
