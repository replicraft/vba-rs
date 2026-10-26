' INV: func.abs, lit.null
' SOURCE: derived Abs(Null) is Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Abs(Null))
End Sub
