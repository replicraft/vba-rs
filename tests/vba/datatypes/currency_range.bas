' INV: type.currency
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  922337203685477.5807 
' EXPECT: -922337203685477.5808 
Public Sub Main()
    Dim c As Currency
    c = CCur("922337203685477.5807")
    Debug.Print c
    c = CCur("-922337203685477.5808")
    Debug.Print c
End Sub
