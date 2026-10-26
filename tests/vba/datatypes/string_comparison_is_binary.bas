' INV: type.string, op.lt
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: FalseTrueTrue
Public Sub Main()
    Debug.Print "a" < "B"; "A" < "b"; "Z" < "a"
End Sub
