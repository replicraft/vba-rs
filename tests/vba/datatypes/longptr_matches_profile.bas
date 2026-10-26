' INV: type.longptr
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT:  6 
' EXPECT: LongLong
Public Sub Main()
    Dim p As LongPtr
    p = 5
    Debug.Print p + 1
    Debug.Print TypeName(p)
End Sub
