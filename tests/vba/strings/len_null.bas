' INV: func.len, lit.null
' SOURCE: spec ms-vbal 6.1.2.11.1.22 Len of Null is Null
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Len(Null))
End Sub
