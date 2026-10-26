' INV: func.len, lit.null
' SOURCE: derived Len(Null) is Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Len(Null))
End Sub
