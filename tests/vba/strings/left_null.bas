' INV: func.left, func.right, func.mid, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/left-function
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Left(Null, 2))
    Debug.Print IsNull(Right(Null, 2))
    Debug.Print IsNull(Mid(Null, 1))
End Sub
