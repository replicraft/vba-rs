' INV: type.currency
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  1.2346 
' EXPECT: True
Public Sub Main()
    Dim c As Currency
    c = 1.23456789
    Debug.Print c
    c = 0.1 + 0.2
    Debug.Print c = 0.3
End Sub
