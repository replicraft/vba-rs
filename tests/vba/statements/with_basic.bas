' INV: stmt.with, expr.with-member
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/with-statement
' EXPECT:  2 
Public Sub Main()
    Dim c As New Collection
    With c
        .Add "x"
        .Add "y"
        Debug.Print .Count
    End With
End Sub
