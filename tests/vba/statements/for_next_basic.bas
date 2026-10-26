' INV: stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/fornext-statement
' EXPECT:  15 
Public Sub Main()
    Dim i As Long, s As Long
    For i = 1 To 5
        s = s + i
    Next i
    Debug.Print s
End Sub
