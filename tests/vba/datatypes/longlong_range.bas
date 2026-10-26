' INV: type.longlong
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  9223372036854775807 
' EXPECT: -9223372036854775808 
Public Sub Main()
    Dim n As LongLong
    n = 9223372036854775807^
    Debug.Print n
    n = -n - 1
    Debug.Print n
End Sub
