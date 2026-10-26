' INV: stmt.goto, stmt.for-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/goto-statement
' EXPECT:  4 
Public Sub Main()
    Dim i As Long
    For i = 1 To 10
        If i = 4 Then GoTo Done
    Next
Done:
    Debug.Print i
End Sub
