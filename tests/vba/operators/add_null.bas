' INV: op.add, lit.null
' SOURCE: derived Null propagates through arithmetic
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Null + 1)
    Debug.Print IsNull(1 + Null)
End Sub
