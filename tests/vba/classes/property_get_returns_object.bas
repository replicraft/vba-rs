' INV: stmt.property-get, stmt.set
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: load
'#MODULE Owner class
Private c As New Collection
Public Property Get Items() As Collection
    Set Items = c
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim o As New Owner
    o.Items.Add 5
    o.Items.Add 6
    Debug.Print o.Items.Count; o.Items.Item(2); o.Items(1)
End Sub
