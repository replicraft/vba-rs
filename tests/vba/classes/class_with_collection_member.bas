' INV: type.collection, expr.new
' SOURCE: derived a class owning a Collection
' EXPECT:  2 
'#MODULE Bag class
Private items As New Collection
Public Sub Add(ByVal s As String)
    items.Add s
End Sub
Public Property Get Count() As Long
    Count = items.Count
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Bag
    b.Add "x": b.Add "y"
    Debug.Print b.Count
End Sub
