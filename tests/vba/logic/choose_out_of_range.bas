' INV: func.choose, lit.null
' SOURCE: spec ms-vbal 6.1.2.8.1.2 Choose
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Choose(0, "a", "b"))
    Debug.Print IsNull(Choose(3, "a", "b"))
End Sub
