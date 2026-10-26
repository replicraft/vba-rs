' INV: type.long
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  2147483647 
' EXPECT: -2147483648 
Public Sub Main()
    Dim n As Long
    n = 2147483647
    Debug.Print n
    n = -2147483647 - 1
    Debug.Print n
End Sub
