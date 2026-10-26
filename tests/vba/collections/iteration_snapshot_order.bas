' INV: type.collection, stmt.for-each-next
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT: 312
Public Sub Main()
    Dim c As New Collection, v As Variant, s As String
    c.Add 3: c.Add 1: c.Add 2
    For Each v In c: s = s & v: Next
    Debug.Print s
End Sub
