' INV: stmt.property-set, stmt.set
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/property-set-statement
' EXPECT:  1 
'#MODULE Holder class
Private o As Object
Public Property Set Item(ByVal x As Object)
    Set o = x
End Property
Public Property Get Item() As Object
    Set Item = o
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim h As New Holder, c As New Collection
    c.Add 5
    Set h.Item = c
    Debug.Print h.Item.Count
End Sub
