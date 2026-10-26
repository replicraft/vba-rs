' INV: op.like
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/like-operator
' ERROR: 93
Public Sub Main()
    Debug.Print "a [xyz" Like "a [*"
End Sub
