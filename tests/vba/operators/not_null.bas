' INV: op.not, lit.null
' SOURCE: derived Not Null is Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Not Null)
End Sub
