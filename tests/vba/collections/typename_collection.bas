' INV: type.collection, func.typename
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/typename-function
' EXPECT: Collection
Public Sub Main()
    Dim c As New Collection
    Debug.Print TypeName(c)
End Sub
