' INV: stmt.for-each-next, stmt.exit
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT:  1 
Public Sub Main()
    Dim v As Variant
    For Each v In Array(1, 2, 3)
        If v = 2 Then Exit For
        Debug.Print v
    Next
End Sub
