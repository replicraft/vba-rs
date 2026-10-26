' INV: stmt.for-next, stmt.exit
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT:  3 
Public Sub Main()
    Dim i As Long
    For i = 1 To 10
        If i = 3 Then Exit For
    Next
    Debug.Print i
End Sub
