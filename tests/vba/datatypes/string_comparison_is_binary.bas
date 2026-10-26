' INV: type.string, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational operators, binary comparison
' EXPECT: FalseTrueTrue
Public Sub Main()
    Debug.Print "a" < "B"; "A" < "b"; "Z" < "a"
End Sub
