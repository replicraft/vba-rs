' INV: stmt.with, expr.with-member
' SOURCE: spec ms-vbal 5.4.2.21 With statement, 6.1.3.1.1.1 Collection Count
' EXPECT:  2 
Public Sub Main()
    Dim c As New Collection
    With c
        .Add "x"
        .Add "y"
        Debug.Print .Count
    End With
End Sub
