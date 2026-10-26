' INV: typechar.bang
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: Single
Public Sub Main()
    Dim a!
    Debug.Print TypeName(a)
End Sub
