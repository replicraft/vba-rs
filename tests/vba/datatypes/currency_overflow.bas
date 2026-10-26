' INV: type.currency
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' ERROR: 6
Public Sub Main()
    Dim c As Currency
    c = CCur("922337203685477.5807")
    c = c + 1
End Sub
