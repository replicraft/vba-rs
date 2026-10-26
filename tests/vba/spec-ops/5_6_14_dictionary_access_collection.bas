' INV: vbal.5.6.14
' SOURCE: spec ms-vbal 5.6.14 dictionary access invokes the default member (Item) with a String key
' EXPECT: red-value
' EXPECT: blue-value
Public Sub Main()
    Dim c As New Collection
    c.Add "red-value", "red"
    c.Add "blue-value", "blue"
    Debug.Print c!red
    Debug.Print c!blue
End Sub
