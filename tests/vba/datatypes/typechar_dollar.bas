' INV: typechar.dollar
' SOURCE: spec ms-vbal 3.3.5.3 type-suffix declared types
' EXPECT: String
Public Sub Main()
    Dim a$
    Debug.Print TypeName(a)
End Sub
