' INV: stmt.function
' SOURCE: spec ms-vbal 5.3.1 function result variable
' EXPECT:  5 
Function Add(a As Long, b As Long) As Long
    Add = a + b
End Function
Public Sub Main()
    Debug.Print Add(2, 3)
End Sub
