' INV: func.choose, lit.null
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/choose-function
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print IsNull(Choose(0, "a", "b"))
    Debug.Print IsNull(Choose(3, "a", "b"))
End Sub
