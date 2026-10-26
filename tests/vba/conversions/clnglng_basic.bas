' INV: func.clnglng
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT: 1099511627776
' EXPECT: 8
' EXPECT: LongLong
Public Sub Main()
    Debug.Print CStr(CLngLng(2 ^ 40))
    Debug.Print CStr(CLngLng(7.5))
    Debug.Print CStr(TypeName(CLngLng(1)))
End Sub
