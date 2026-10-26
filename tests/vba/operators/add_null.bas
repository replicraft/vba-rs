' INV: op.add, lit.null
' SOURCE: spec ms-vbal 5.6.9.3.2 plus operator (Null)
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Null + 1)
    Debug.Print IsNull(1 + Null)
End Sub
