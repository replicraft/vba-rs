' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: True
Public Sub Main()
    Debug.Print CStr("ab" Like "a*b")
End Sub
