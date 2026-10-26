' INV: type.currency, typechar.at
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  922337203685477.5807 
Public Sub Main()
    Dim c As Currency
    c = 922337203685477.5807@
    Debug.Print c
End Sub
