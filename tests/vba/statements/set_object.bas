' INV: stmt.set, expr.new, lit.nothing
' SOURCE: spec ms-vbal 5.4.3.9 Set statement, 6.1.3.1.1.1 Collection Count
' EXPECT:  1 
' EXPECT: True
Public Sub Main()
    Dim c As Collection
    Set c = New Collection
    c.Add 1
    Debug.Print c.Count
    Set c = Nothing
    Debug.Print c Is Nothing
End Sub
