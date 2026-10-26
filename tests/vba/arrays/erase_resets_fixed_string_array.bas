' INV: stmt.erase, decl.array-bounds
' SOURCE: spec ms-vbal 5.4.3.4 Erase resets a fixed-size array to default values (2.3 String default)
' EXPECT: []
Public Sub Main()
    Dim s(1 To 2) As String
    s(1) = "x"
    Erase s
    Debug.Print "[" & s(1) & "]"
End Sub
