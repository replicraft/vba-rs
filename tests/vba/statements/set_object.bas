' INV: stmt.set, expr.new, lit.nothing
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/set-statement
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
