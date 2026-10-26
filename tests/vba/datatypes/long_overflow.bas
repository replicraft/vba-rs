' INV: type.long
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' ERROR: 6
Public Sub Main()
    Dim n As Long
    n = 2147483647
    n = n + 1
End Sub
