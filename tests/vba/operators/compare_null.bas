' INV: op.eq, lit.null
' SOURCE: spec ms-vbal 5.6.9.5 relational operators (Null)
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Null = 1)
    Debug.Print IsNull(1 < Null)
End Sub
