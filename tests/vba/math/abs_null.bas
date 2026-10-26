' INV: func.abs, lit.null
' SOURCE: spec ms-vbal 6.1.2.10.1.1 Abs
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Abs(Null))
End Sub
