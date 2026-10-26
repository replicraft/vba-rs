' INV: expr.implicit-conv
' SOURCE: derived assigning a Double to an Integer rounds like CInt
' EXPECT:  2 
' EXPECT:  4 
Public Sub Main()
    Dim i As Integer
    i = 2.5
    Debug.Print i
    i = 3.5
    Debug.Print i
End Sub
