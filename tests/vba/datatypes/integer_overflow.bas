' INV: type.integer
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    i = 32767
    i = i + 1
End Sub
