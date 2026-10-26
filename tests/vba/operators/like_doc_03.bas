' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' EXPECT: False
Public Sub Main()
    Debug.Print CStr("F" Like "[!A-Z]")
End Sub
