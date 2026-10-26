' INV: typechar.bang
' SOURCE: spec ms-vbal 3.3.5.3 type-suffix declared types
' EXPECT: Single
Public Sub Main()
    Dim a!
    Debug.Print TypeName(a)
End Sub
