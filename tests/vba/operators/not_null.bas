' INV: op.not, lit.null
' SOURCE: spec ms-vbal 5.6.9.8.1 Not operator (Null)
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Not Null)
End Sub
