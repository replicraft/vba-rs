' INV: type.longlong
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' ERROR: 6
Public Sub Main()
    Dim n As LongLong
    n = 9223372036854775807^
    n = n + 1
End Sub
