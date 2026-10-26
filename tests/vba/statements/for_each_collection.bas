' INV: stmt.for-each-next, type.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/for-eachnext-statement
' EXPECT: a
' EXPECT: b
Public Sub Main()
    Dim c As New Collection, v As Variant
    c.Add "a"
    c.Add "b"
    For Each v In c
        Debug.Print v
    Next
End Sub
