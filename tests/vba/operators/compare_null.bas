' INV: op.eq, lit.null
' SOURCE: derived a comparison with Null yields Null
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Null = 1)
    Debug.Print IsNull(1 < Null)
End Sub
