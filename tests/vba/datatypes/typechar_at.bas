' INV: typechar.at
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: Currency
Public Sub Main()
    Dim a@
    Debug.Print TypeName(a)
End Sub
