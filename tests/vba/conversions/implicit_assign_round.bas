' INV: expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1.1 Banker's rounding
' EXPECT:  2 
' EXPECT:  4 
Public Sub Main()
    Dim i As Integer
    i = 2.5
    Debug.Print i
    i = 3.5
    Debug.Print i
End Sub
