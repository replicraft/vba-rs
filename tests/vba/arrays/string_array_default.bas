' INV: decl.array-bounds
' SOURCE: spec ms-vbal 2.3 default value of String array elements
' EXPECT: []
Public Sub Main()
    Dim s(1) As String
    Debug.Print "[" & s(0) & "]"
End Sub
