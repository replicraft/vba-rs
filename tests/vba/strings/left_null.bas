' INV: func.left, func.right, func.mid, lit.null
' SOURCE: spec ms-vbal 6.1.2.11.1.19 Left, 6.1.2.11.1.30 Right, 6.1.2.11.1.25 Mid return Null for Null
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Left(Null, 2))
    Debug.Print IsNull(Right(Null, 2))
    Debug.Print IsNull(Mid(Null, 1))
End Sub
