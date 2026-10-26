' INV: func.int, lit.null
' SOURCE: spec ms-vbal 6.1.2.3.1.18 Int
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Int(Null))
End Sub
